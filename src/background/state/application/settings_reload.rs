use std::path::Path;

use seelen_core::{SeelenLibError, state::Settings};

/// Only the three files that form a settings snapshot can refresh runtime settings.
/// Backups and resource files must never accidentally replace the active snapshot.
pub fn is_settings_file(changed: &Path, settings_path: &Path) -> bool {
    if changed == settings_path {
        return true;
    }
    let (Some(parent), Some(stem)) = (settings_path.parent(), settings_path.file_stem()) else {
        return false;
    };
    let stem = stem.to_string_lossy();
    changed == parent.join(format!("{stem}_shortcuts.json"))
        || changed == parent.join(format!("{stem}_by_app.yml"))
}

/// A failed/partial write is an error, not a request to reset the running app.
/// Comparing serialized values also suppresses the events caused by our own saves.
pub fn load_changed_settings(
    settings_path: &Path,
    current: &Settings,
) -> Result<Option<Settings>, SeelenLibError> {
    let loaded = Settings::load(settings_path)?;
    if serde_json::to_value(&loaded)? == serde_json::to_value(current)? {
        Ok(None)
    } else {
        Ok(Some(loaded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watches_main_and_both_sidecars_but_not_backups_or_other_widgets() {
        let main = Path::new("profile/settings.json");
        for name in [
            "settings.json",
            "settings_shortcuts.json",
            "settings_by_app.yml",
        ] {
            assert!(is_settings_file(&Path::new("profile").join(name), main));
        }
        for other in [
            "profile/settings.before-win-launchpad.json",
            "profile/settings_shortcuts.json.tmp",
            "profile/widgets/settings.json",
            "other/settings.json",
        ] {
            assert!(!is_settings_file(Path::new(other), main));
        }
    }

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "mac-ui-settings-reload-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn main(&self) -> std::path::PathBuf {
            self.0.join("settings.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // This directory was freshly created by this test; it holds no user files.
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn disk_enable_and_disable_are_loaded_without_changing_other_preferences() {
        let fixture = Fixture::new();
        let id = "@seelen/apps-menu".into();
        let mut current = Settings::default();
        current.sanitize().unwrap();
        current.streaming_mode = true;
        let mut enabled = current.clone();
        enabled.set_widget_enabled(&id, true);
        enabled.save(fixture.main()).unwrap();
        let updated = load_changed_settings(&fixture.main(), &current)
            .unwrap()
            .unwrap();
        assert!(updated.is_widget_enabled(&id));
        assert!(updated.streaming_mode);
        assert!(
            load_changed_settings(&fixture.main(), &updated)
                .unwrap()
                .is_none()
        );

        current.save(fixture.main()).unwrap();
        let disabled = load_changed_settings(&fixture.main(), &updated)
            .unwrap()
            .unwrap();
        assert!(!disabled.is_widget_enabled(&id));
        assert!(disabled.streaming_mode);
    }

    #[test]
    fn shortcut_sidecar_updates_are_part_of_the_same_snapshot() {
        let fixture = Fixture::new();
        let mut current = Settings::default();
        current.sanitize().unwrap();
        current.save(fixture.main()).unwrap();
        let mut disabled = current.clone();
        disabled.shortcuts.enabled = false;
        disabled.save(fixture.main()).unwrap();
        let updated = load_changed_settings(&fixture.main(), &current)
            .unwrap()
            .unwrap();
        assert!(!updated.shortcuts.enabled);
    }

    #[test]
    fn incomplete_or_missing_files_do_not_reset_the_last_good_runtime_snapshot() {
        let fixture = Fixture::new();
        let id = "@seelen/apps-menu".into();
        let mut current = Settings::default();
        current.set_widget_enabled(&id, true);
        current.sanitize().unwrap();
        current.save(fixture.main()).unwrap();
        std::fs::write(fixture.0.join("settings_shortcuts.json"), b"{").unwrap();
        assert!(load_changed_settings(&fixture.main(), &current).is_err());
        assert!(current.is_widget_enabled(&id));
        assert!(load_changed_settings(&fixture.0.join("missing.json"), &current).is_err());
    }
}
