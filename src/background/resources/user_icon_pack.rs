//! Compatibility handlers for old clients. Their saved files are retained, but
//! MAC UI no longer supports overriding its system application icons with packs.
use macui_core::state::IconPackEntry;

use crate::error::Result;

#[tauri::command(async)]
pub async fn register_user_custom_app_icon(
    icon_base64: String,
    entry: IconPackEntry,
) -> Result<()> {
    let _ = (icon_base64, entry);
    Err("Custom icon packs are unavailable; MAC UI uses system icons".into())
}

#[tauri::command(async)]
pub async fn delete_user_custom_app_icon(entry: IconPackEntry) -> Result<()> {
    let _ = entry;
    Err("Custom icon packs are unavailable; existing data has been retained".into())
}
