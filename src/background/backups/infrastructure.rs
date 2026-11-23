use macui_core::system_state::BackupStatus;

#[tauri::command(async)]
pub fn get_backup_status() -> BackupStatus {
    super::application::get_backup_status()
}
