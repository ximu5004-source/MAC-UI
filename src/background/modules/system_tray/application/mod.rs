mod event_queue;
pub mod tray_hook_loader;
pub mod tray_icon;
mod util;

use std::sync::LazyLock;

use seelen_core::system_state::{SysTrayIcon, SysTrayIconId};
use slu_ipc::messages::Win32TrayEvent;
use windows::Win32::UI::WindowsAndMessaging::{CopyIcon, DestroyIcon, HICON};

use self::{event_queue::EventQueue, util::Util};
use crate::{
    error::ResultLogExt, event_manager,
    modules::system_tray::application::tray_hook_loader::TrayHookLoader,
    utils::lock_free::SyncHashMap,
};

struct PendingTrayEvent {
    event: Win32TrayEvent,
    // Copy the native icon before acknowledging Explorer, while its sender
    // still owns the source HICON. Slow pixel extraction happens on our worker.
    copied_icon: Option<isize>,
}

impl PendingTrayEvent {
    fn new(event: Win32TrayEvent) -> Self {
        let data = match &event {
            Win32TrayEvent::IconAdd { data }
            | Win32TrayEvent::IconUpdate { data }
            | Win32TrayEvent::IconRemove { data } => data,
        };
        let copied_icon = data.icon_handle.and_then(|handle| {
            unsafe { CopyIcon(HICON(handle as _)) }
                .ok()
                .map(|icon| icon.0 as isize)
        });
        Self { event, copied_icon }
    }
}

impl Drop for PendingTrayEvent {
    fn drop(&mut self) {
        if let Some(handle) = self.copied_icon {
            let _ = unsafe { DestroyIcon(HICON(handle as _)) };
        }
    }
}

pub struct SystemTrayManager {
    icons: SyncHashMap<SysTrayIconId, SysTrayIcon>,
    _loader: Option<TrayHookLoader>,
}

#[derive(Debug, Clone)]
pub enum SystemTrayEvent {
    Changed,
}

event_manager!(SystemTrayManager, SystemTrayEvent);

impl SystemTrayManager {
    pub fn acknowledge_notification(&self, id: &SysTrayIconId) {
        if let Some(mut icon) = self.icons.get(id, |icon| icon.clone()) {
            if icon.notification.take().is_some() {
                self.icons.upsert(id.clone(), icon);
                Self::send(SystemTrayEvent::Changed);
            }
        }
    }
    fn create() -> Self {
        log::trace!("Creating system tray manager");

        let loader = match TrayHookLoader::new() {
            Ok(loader) => Some(loader),
            Err(err) => {
                log::error!("Failed to create tray hook loader: {:?}", err);
                None
            }
        };

        Self {
            icons: SyncHashMap::new(),
            _loader: loader,
        }
    }

    pub fn instance() -> &'static Self {
        static SYSTEM_TRAY_MANAGER: LazyLock<SystemTrayManager> =
            LazyLock::new(SystemTrayManager::create);
        &SYSTEM_TRAY_MANAGER
    }

    /// Handles a tray event received via IPC
    /// This method should be called from the AppIpc handler
    pub fn enqueue_tray_event(event: Win32TrayEvent) {
        static QUEUE: LazyLock<Option<EventQueue<PendingTrayEvent>>> = LazyLock::new(|| {
            EventQueue::start(
                1024,
                |pending: PendingTrayEvent| {
                    if SystemTrayManager::instance()
                        .process_event(pending.event.clone(), pending.copied_icon)
                        .is_some()
                    {
                        SystemTrayManager::send(SystemTrayEvent::Changed);
                    }
                },
                || {
                    log::warn!("Tray event queue overflowed; refreshing the native icon snapshot");
                    SystemTrayManager::instance().icons.clear();
                    SystemTrayManager::send(SystemTrayEvent::Changed);
                    Util::refresh_icons().log_error();
                },
            )
            .map_err(|err| log::error!("Could not start tray event worker: {err}"))
            .ok()
        });
        if let Some(queue) = &*QUEUE {
            queue.push(PendingTrayEvent::new(event));
        }
    }
}
