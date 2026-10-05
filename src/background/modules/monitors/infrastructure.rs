use std::sync::Once;

use seelen_core::{
    handlers::SeelenEvent,
    system_state::{MonitorId, PhysicalMonitor},
};

use crate::{
    app::emit_to_webviews,
    error::Result,
    modules::monitors::{MonitorManager, MonitorManagerEvent},
    windows_api::MonitorEnumerator,
};

fn get_monitor_manager() -> &'static MonitorManager {
    static TAURI_EVENT_REGISTRATION: Once = Once::new();
    TAURI_EVENT_REGISTRATION.call_once(|| {
        let initial = MonitorManager::instance().get_cached_data();
        log::debug!("Initial monitors: {initial:#?}");

        MonitorManager::subscribe(|event| {
            if event != MonitorManagerEvent::ViewsChanged {
                return;
            }
            log::trace!("MonitorManagerEvent::{:?}", event);
            let monitors = MonitorManager::instance().get_cached_data();
            log::debug!("{monitors:#?}");
            emit_to_webviews(SeelenEvent::SystemMonitorsChanged, monitors);
            crate::widgets::webview::restore_webviews_after_display_change();
        });
    });
    MonitorManager::instance()
}

#[tauri::command(async)]
pub fn get_connected_monitors() -> Vec<PhysicalMonitor> {
    get_monitor_manager().get_cached_data()
}

#[tauri::command(async)]
pub fn set_monitor_hdr(id: MonitorId, state: bool) -> Result<()> {
    let monitor = MonitorEnumerator::enumerate_win32()?
        .into_iter()
        .find(|m| matches!(m.get_stable_info(), Ok((mid, _)) if mid == id))
        .ok_or("Monitor not found")?;
    monitor.set_hdr_state(state)
}
