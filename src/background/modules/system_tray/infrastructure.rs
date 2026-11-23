use std::sync::Once;

use macui_core::{
    handlers::MacUiEvent,
    system_state::{SysTrayIcon, SysTrayIconId, SystrayIconAction},
};

use crate::{
    app::emit_to_webviews,
    error::Result,
    modules::system_tray::application::{SystemTrayEvent, SystemTrayManager},
};

fn get_system_tray_manager() -> &'static SystemTrayManager {
    static TAURI_EVENT_REGISTRATION: Once = Once::new();
    TAURI_EVENT_REGISTRATION.call_once(|| {
        SystemTrayManager::subscribe(|event| {
            if !matches!(event, SystemTrayEvent::Changed) {
                return;
            }
            emit_to_webviews(
                MacUiEvent::SystemTrayChanged,
                SystemTrayManager::instance().icons(),
            );
        });
    });
    SystemTrayManager::instance()
}

#[tauri::command(async)]
pub fn get_system_tray_icons() -> Vec<SysTrayIcon> {
    get_system_tray_manager().icons()
}

#[tauri::command(async)]
pub fn send_system_tray_icon_action(id: SysTrayIconId, action: SystrayIconAction) -> Result<()> {
    get_system_tray_manager().send_action(&id, &action)?;
    if matches!(
        action,
        SystrayIconAction::LeftClick | SystrayIconAction::LeftDoubleClick
    ) {
        get_system_tray_manager().acknowledge_notification(&id);
    }
    Ok(())
}
