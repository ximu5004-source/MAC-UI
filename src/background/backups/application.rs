use macui_core::system_state::BackupStatus;

/// Kept for compatible callers; local configuration saving never uploads data.
pub fn on_settings_saved() {}

pub fn get_backup_status() -> BackupStatus {
    BackupStatus { last_sync: None }
}
