use std::sync::Once;

use macui_core::{handlers::MacUiEvent, system_state::CommunicationNotification};

use crate::{app::emit_to_webviews, error::Result};

use super::application::CommunicationNotificationManager;

fn get_manager() -> &'static CommunicationNotificationManager {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        CommunicationNotificationManager::subscribe(|_| {
            emit_to_webviews(
                MacUiEvent::CommunicationNotificationsChanged,
                CommunicationNotificationManager::instance().notifications(),
            );
        });
    });
    CommunicationNotificationManager::instance()
}

#[tauri::command(async)]
pub fn get_communication_notifications() -> Vec<CommunicationNotification> {
    get_manager().notifications()
}

#[tauri::command(async)]
pub fn activate_communication_notification(id: String) -> Result<()> {
    get_manager().activate(&id)
}

#[tauri::command(async)]
pub fn dismiss_communication_notification(id: String) {
    get_manager().dismiss(&id);
}

#[tauri::command(async)]
pub fn clear_communication_notifications() {
    get_manager().clear();
}
