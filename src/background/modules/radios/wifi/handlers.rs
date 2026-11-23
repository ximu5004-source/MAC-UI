use std::sync::Once;

use macui_core::handlers::MacUiEvent;

use crate::{app::emit_to_webviews, error::Result};

use super::{WifiManager, WifiManagerEvent};

fn get_wifi_manager() -> &'static WifiManager {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        WifiManager::subscribe(|event| match event {
            WifiManagerEvent::NetworksChanged => {
                match WifiManager::instance().get_available_networks() {
                    Ok(networks) => emit_to_webviews(MacUiEvent::NetworkWlanScanned, &networks),
                    Err(e) => log::error!("Failed to emit WiFi networks: {e}"),
                }
            }
        });
    });
    WifiManager::instance()
}

#[tauri::command(async)]
pub fn wlan_scan() {
    get_wifi_manager().scan_networks();
}

#[tauri::command(async)]
pub fn wlan_connect(ssid: String, password: Option<String>, hidden: bool) -> Result<bool> {
    let manager = get_wifi_manager();
    match manager.connect(&ssid, password.as_deref(), hidden) {
        Ok(result) => {
            if result && let Ok(networks) = manager.get_available_networks() {
                emit_to_webviews(MacUiEvent::NetworkWlanScanned, &networks);
            }
            Ok(result)
        }
        Err(err) => {
            log::error!("WiFi connect error: {err}");
            Ok(false)
        }
    }
}

#[tauri::command(async)]
pub fn wlan_forget(ssid: String) -> Result<()> {
    let manager = get_wifi_manager();
    manager.forget(&ssid)?;
    if let Ok(networks) = manager.get_available_networks() {
        emit_to_webviews(MacUiEvent::NetworkWlanScanned, &networks);
    }
    Ok(())
}

#[tauri::command(async)]
pub fn wlan_disconnect() -> Result<()> {
    let manager = get_wifi_manager();
    manager.disconnect()?;
    if let Ok(networks) = manager.get_available_networks() {
        emit_to_webviews(MacUiEvent::NetworkWlanScanned, &networks);
    }
    Ok(())
}
