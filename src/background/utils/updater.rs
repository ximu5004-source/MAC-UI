use crate::error::Result;
use tauri_plugin_updater::Update;

/// Updates are installed explicitly from a MAC UI installer.
pub async fn check_for_updates() -> Result<Option<Update>> {
    Ok(None)
}

pub async fn trace_update_intallation(_update: Update) -> Result<()> {
    Err("Automatic online updates are not available in MAC UI.".into())
}
