mod event_queue;
mod hook_lifecycle;
mod identity;
pub mod tray_hook_loader;
pub mod tray_icon;
mod util;

use std::sync::LazyLock;

use macui_core::system_state::{SysTrayIcon, SysTrayIconId};
use macui_ipc::messages::Win32TrayEvent;
use windows::Win32::UI::WindowsAndMessaging::{CopyIcon, DestroyIcon, HICON};

use self::{event_queue::EventQueue, tray_icon::SystrayEvent, util::Util};
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
        let copied_icon = data
            .icon_handle
            .filter(|handle| *handle != 0)
            .and_then(|handle| {
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
    /// Immutable per-message snapshot: Changed alone can coalesce a burst
    /// into the latest icon state before its subscribers read that state.
    NativeNotification(SysTrayIcon),
}

event_manager!(SystemTrayManager, SystemTrayEvent);

impl SystemTrayManager {
    pub fn acknowledge_notification(&self, id: &SysTrayIconId) {
        // Change only the notification field under its existing lock. Copying
        // the icon and reinserting it could roll back a concurrent native frame.
        let changed = self
            .icons
            .get(id, |icon| icon.notification.take().is_some())
            .unwrap_or(false);
        if changed {
            Self::send(SystemTrayEvent::Changed);
        }
    }
    fn create() -> Self {
        log::trace!("Creating system tray manager");

        let loader = match TrayHookLoader::new(|| {
            // A replaced Explorer invalidates its icon snapshot. The lifecycle
            // worker requests fresh native registrations only after reconnect.
            SystemTrayManager::instance().icons.clear();
            SystemTrayManager::send(SystemTrayEvent::Changed);
        }) {
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
            let mut last_cleanup = std::time::Instant::now();
            EventQueue::start(
                1024,
                |pending: PendingTrayEvent| {
                    let has_notification = match &pending.event {
                        Win32TrayEvent::IconAdd { data } | Win32TrayEvent::IconUpdate { data } => {
                            data.notification
                                .as_ref()
                                .is_some_and(|text| !text.is_empty())
                        }
                        Win32TrayEvent::IconRemove { .. } => false,
                    };
                    if let Some(event) = SystemTrayManager::instance()
                        .process_event(pending.event.clone(), pending.copied_icon)
                    {
                        if has_notification {
                            match event {
                                SystrayEvent::IconAdd(icon) | SystrayEvent::IconUpdate(icon) => {
                                    SystemTrayManager::send(SystemTrayEvent::NativeNotification(
                                        icon,
                                    ));
                                }
                                SystrayEvent::IconRemove(_) => {}
                            }
                        }
                        SystemTrayManager::send(SystemTrayEvent::Changed);
                    }
                },
                || {
                    log::warn!("Tray event queue overflowed; refreshing the native icon snapshot");
                    SystemTrayManager::instance().icons.clear();
                    SystemTrayManager::send(SystemTrayEvent::Changed);
                    Util::refresh_icons().log_error();
                },
                move || {
                    if last_cleanup.elapsed() >= std::time::Duration::from_secs(2) {
                        last_cleanup = std::time::Instant::now();
                        if SystemTrayManager::instance().remove_dead_icons() {
                            SystemTrayManager::send(SystemTrayEvent::Changed);
                        }
                    }
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
