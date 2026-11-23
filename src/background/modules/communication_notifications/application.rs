use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use macui_core::system_state::{CommunicationNotification, SystrayIconAction};
use parking_lot::Mutex;

use crate::{
    error::Result,
    event_manager,
    modules::system_tray::{SystemTrayEvent, SystemTrayManager},
};

use super::{
    domain::{CommunicationNotificationStore, PopupNotification, communication_app},
    popup::{PopupCollector, activate_client, owns_window},
};

pub struct CommunicationNotificationManager {
    store: Mutex<CommunicationNotificationStore>,
    tray_subscription: String,
    _popup_collector: Option<PopupCollector>,
}

#[derive(Clone, Debug)]
pub enum CommunicationNotificationEvent {
    Changed,
}

event_manager!(
    CommunicationNotificationManager,
    CommunicationNotificationEvent
);

impl CommunicationNotificationManager {
    fn create() -> Self {
        let mut store = CommunicationNotificationStore::default();
        // Existing metadata is a baseline, not a new-message event on startup.
        store.baseline(&SystemTrayManager::instance().icons());
        let tray_subscription = SystemTrayManager::subscribe(|event| {
            let date = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| {
                    duration.as_millis().min(i64::MAX as u128) as i64
                });
            let changed = match event {
                SystemTrayEvent::NativeNotification(icon) => {
                    Self::instance().store.lock().update_native(&icon, date)
                }
                SystemTrayEvent::Changed => {
                    let icons = SystemTrayManager::instance().icons();
                    Self::instance().store.lock().update(&icons, date)
                }
            };
            if changed {
                Self::send(CommunicationNotificationEvent::Changed);
            }
        });
        let popup_collector = PopupCollector::start()
            .map_err(|error| {
                log::error!("Unable to collect accessible notification popups: {error:?}");
            })
            .ok();
        Self {
            store: Mutex::new(store),
            tray_subscription,
            _popup_collector: popup_collector,
        }
    }

    pub fn instance() -> &'static Self {
        static MANAGER: LazyLock<CommunicationNotificationManager> =
            LazyLock::new(CommunicationNotificationManager::create);
        &MANAGER
    }

    pub fn notifications(&self) -> Vec<CommunicationNotification> {
        self.store.lock().notifications()
    }

    pub(super) fn publish_popup(popup: PopupNotification) {
        let date = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| {
                duration.as_millis().min(i64::MAX as u128) as i64
            });
        if Self::instance().store.lock().update_popup(popup, date) {
            Self::send(CommunicationNotificationEvent::Changed);
        }
    }

    pub fn dismiss(&self, id: &str) {
        if self.store.lock().dismiss(id) {
            Self::send(CommunicationNotificationEvent::Changed);
        }
    }

    pub fn clear(&self) {
        if self.store.lock().clear() {
            Self::send(CommunicationNotificationEvent::Changed);
        }
    }

    pub fn activate(&self, id: &str) -> Result<()> {
        let (notification, popup_target) = {
            let store = self.store.lock();
            (
                store.get(id).ok_or("Notification is no longer available")?,
                store.popup_target(id),
            )
        };
        let tray = SystemTrayManager::instance();
        let still_owned = notification.tray_id.as_ref().filter(|id| {
            tray.icons().iter().any(|icon| {
                &icon.stable_id == *id
                    && communication_app(icon) == Some(notification.app)
                    && popup_target.as_ref().is_none_or(|client| {
                        icon.window_handle.is_some_and(|handle| {
                            owns_window(client, crate::windows_api::window::Window::from(handle))
                        })
                    })
            })
        });
        if let Some(tray_id) = still_owned {
            tray.send_action(tray_id, &SystrayIconAction::LeftDoubleClick)?;
            tray.acknowledge_notification(tray_id);
        } else if let Some(client) = popup_target {
            activate_client(&client)?;
        } else {
            return Err("The messaging client is no longer available".into());
        }
        if self.store.lock().dismiss_app(notification.app) {
            Self::send(CommunicationNotificationEvent::Changed);
        }
        Ok(())
    }
}

impl Drop for CommunicationNotificationManager {
    fn drop(&mut self) {
        SystemTrayManager::unsubscribe(&self.tray_subscription);
    }
}
