//! One-time, copy-only migration to MAC UI's independent application identity.
//! Legacy files are backed up before adopting MAC UI's independent resource IDs.
use std::{fs, io, path::Path};
use tauri::Manager;

pub fn migrate_legacy_data(app: &tauri::AppHandle<tauri::Wry>) -> crate::error::Result<()> {
    let destination = app.path().app_data_dir()?;
    copy_legacy_installation(&destination)?;
    migrate_macui_identity(&destination)?;
    Ok(())
}

fn copy_legacy_installation(destination: &Path) -> crate::error::Result<()> {
    let parent = destination
        .parent()
        .ok_or("Invalid application data directory")?;
    let source = parent.join("com.seelen.seelen-ui");
    if !source.join("settings.json").is_file() || destination.join(".mac-ui-migrated").exists() {
        return Ok(());
    }
    // A configured new installation always wins. Never merge or overwrite its data.
    if destination.join("settings.json").exists() {
        return Ok(());
    }
    let stage = parent.join(format!("mac-ui-migration-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage)?;
    copy_tree(&source, &stage)?;
    rewrite_paths(&stage, &source, destination)?;
    if destination.exists() {
        // Only remove an empty directory created by the runtime. Nonempty data is preserved.
        if fs::read_dir(&destination)?.next().is_some() {
            return Err(format!(
                "MAC UI data directory is not empty. Migration copy retained at {}",
                stage.display()
            )
            .into());
        }
        fs::remove_dir(&destination)?;
    }
    fs::write(
        stage.join(".mac-ui-migrated"),
        b"Copied from previous installation; original data retained.\n",
    )?;
    fs::rename(&stage, destination)?;
    log::info!("Copied previous settings into MAC UI application data; original files retained.");
    Ok(())
}

const IDENTITY_MARKER: &str = ".macui-identity-v1";

/// The prior MAC UI build already copied the legacy namespace, so this is a
/// separate one-time upgrade. Work on a full copy, keep the previous directory
/// as a sibling backup, and only replace it after the new settings validate.
fn migrate_macui_identity(destination: &Path) -> crate::error::Result<()> {
    if !destination.is_dir() || destination.join(IDENTITY_MARKER).exists() {
        return Ok(());
    }
    let parent = destination
        .parent()
        .ok_or("Invalid application data directory")?;
    let suffix = uuid::Uuid::new_v4();
    let stage = parent.join(format!("macui-identity-stage-{suffix}"));
    let backup = parent.join(format!("macui-before-identity-{suffix}"));
    copy_tree(destination, &stage)?;
    migrate_widget_data_directories(&stage.join("data"))?;
    rewrite_identity_files(&stage, destination)?;
    let settings_path = stage.join("settings.json");
    if settings_path.exists() {
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&settings_path)?)?;
        let object = value.as_object_mut().ok_or("Invalid settings object")?;
        let widgets = object
            .entry("byWidget")
            .or_insert_with(|| serde_json::json!({}));
        let widgets = widgets
            .as_object_mut()
            .ok_or("Invalid widget settings object")?;
        let settings = widgets
            .entry("@macui/settings")
            .or_insert_with(|| serde_json::json!({}));
        let settings = settings
            .as_object_mut()
            .ok_or("Invalid settings widget object")?;
        settings.insert("colorScheme".into(), serde_json::json!("light"));
        fs::write(&settings_path, serde_json::to_vec_pretty(&value)?)?;
        // Validate and persist fixed glass/system icons without losing shortcuts.
        let settings = macui_core::state::Settings::load(&settings_path)?;
        settings.save(&settings_path)?;
    }
    fs::write(
        stage.join(IDENTITY_MARKER),
        b"MAC UI resource identity and default light UI adopted.\n",
    )?;
    fs::rename(destination, &backup)?;
    if let Err(error) = fs::rename(&stage, destination) {
        if let Err(restore) = fs::rename(&backup, destination) {
            return Err(format!("Identity migration failed: {error}; restore failed: {restore}; previous files remain at {}", backup.display()).into());
        }
        return Err(error.into());
    }
    log::info!(
        "MAC UI resource identity migrated; previous files retained at {}",
        backup.display()
    );
    Ok(())
}

fn migrate_widget_data_directories(data: &Path) -> io::Result<()> {
    if !data.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(data)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if !kind.is_dir() || kind.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(suffix) = name.strip_prefix("seelen-") else {
            continue;
        };
        let target = data.join(format!("macui-{suffix}"));
        if target.exists() {
            // Current-namespace files win, with missing legacy files copied in.
            copy_missing_tree(&entry.path(), &target)?;
            fs::remove_dir_all(entry.path())?; // only our fully backed-up staging copy
        } else {
            fs::rename(entry.path(), target)?;
        }
    }
    Ok(())
}

fn copy_missing_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_missing_tree(&entry.path(), &target)?;
        } else if kind.is_file() && !target.exists() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn rewrite_identity_files(directory: &Path, owned_root: &Path) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            rewrite_identity_files(&entry.path(), owned_root)?;
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        // Keep snapshots/backups byte-for-byte, so they remain useful for recovery.
        if path
            .file_name()
            .is_some_and(|v| v.to_string_lossy().contains(".before-"))
        {
            continue;
        }
        let Ok(data) = fs::read_to_string(&path) else {
            continue;
        };
        match extension {
            "json" => {
                let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&data) else {
                    continue;
                };
                if remap_identity_json(&mut value, owned_root) {
                    fs::write(path, serde_json::to_vec_pretty(&value)?)?;
                }
            }
            "yml" | "yaml" => {
                let Ok(mut value) = serde_yaml::from_str::<serde_yaml::Value>(&data) else {
                    continue;
                };
                if remap_identity_yaml(&mut value, owned_root) {
                    fs::write(
                        path,
                        serde_yaml::to_string(&value).map_err(io::Error::other)?,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn remap_identity_text(text: &str, owned_root: &Path) -> String {
    let mut renamed = text.replace("@seelen/", "@macui/");
    // Only application-owned paths change. User documents or external paths do not.
    if let Ok(relative) = Path::new(&renamed).strip_prefix(owned_root) {
        let relative = relative.to_string_lossy().replace("seelen-", "macui-");
        renamed = owned_root.join(relative).to_string_lossy().into_owned();
    }
    renamed
}

fn remap_identity_json(value: &mut serde_json::Value, owned_root: &Path) -> bool {
    match value {
        serde_json::Value::String(text) => {
            let new = remap_identity_text(text, owned_root);
            let changed = *text != new;
            *text = new;
            changed
        }
        serde_json::Value::Array(values) => values.iter_mut().fold(false, |changed, value| {
            remap_identity_json(value, owned_root) || changed
        }),
        serde_json::Value::Object(values) => {
            let keys: Vec<_> = values.keys().cloned().collect();
            let mut changed = false;
            for key in keys {
                let renamed = key.replace("@seelen/", "@macui/");
                if key != renamed {
                    let old = values.remove(&key).unwrap();
                    if let Some(current) = values.get_mut(&renamed) {
                        merge_missing_json(current, old);
                    } else {
                        values.insert(renamed, old);
                    }
                    changed = true;
                }
            }
            for value in values.values_mut() {
                changed |= remap_identity_json(value, owned_root);
            }
            changed
        }
        _ => false,
    }
}

fn merge_missing_json(current: &mut serde_json::Value, legacy: serde_json::Value) {
    if let (Some(current), serde_json::Value::Object(legacy)) = (current.as_object_mut(), legacy) {
        for (key, old) in legacy {
            match current.get_mut(&key) {
                Some(new) => merge_missing_json(new, old),
                None => {
                    current.insert(key, old);
                }
            }
        }
    }
}

fn remap_identity_yaml(value: &mut serde_yaml::Value, owned_root: &Path) -> bool {
    match value {
        serde_yaml::Value::String(text) => {
            let new = remap_identity_text(text, owned_root);
            let changed = *text != new;
            *text = new;
            changed
        }
        serde_yaml::Value::Sequence(values) => values.iter_mut().fold(false, |changed, value| {
            remap_identity_yaml(value, owned_root) || changed
        }),
        serde_yaml::Value::Mapping(values) => {
            let keys: Vec<_> = values.keys().cloned().collect();
            let mut changed = false;
            for key in keys {
                let mut new = key.clone();
                if remap_identity_yaml(&mut new, owned_root) {
                    let old = values.remove(&key).unwrap();
                    if !values.contains_key(&new) {
                        values.insert(new, old);
                    }
                    changed = true;
                }
            }
            for value in values.values_mut() {
                changed |= remap_identity_yaml(value, owned_root);
            }
            changed
        }
        serde_yaml::Value::Tagged(tag) => remap_identity_yaml(&mut tag.value, owned_root),
        _ => false,
    }
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        // Do not follow junctions/symlinks out of the application data directory.
        if kind.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            let copied = fs::copy(entry.path(), &target)?;
            if copied != entry.metadata()?.len() {
                return Err(io::Error::other("Incomplete data migration copy"));
            }
        }
    }
    Ok(())
}

fn rewrite_paths(directory: &Path, source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            rewrite_paths(&entry.path(), source, destination)?;
            continue;
        }
        if entry.path().extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let Ok(data) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&data) else {
            continue;
        };
        if remap_json(&mut json, source, destination) {
            fs::write(entry.path(), serde_json::to_vec_pretty(&json)?)?;
        }
    }
    Ok(())
}

fn remap_json(value: &mut serde_json::Value, source: &Path, destination: &Path) -> bool {
    match value {
        serde_json::Value::String(text) => {
            let path = Path::new(text);
            if let Ok(suffix) = path.strip_prefix(source) {
                *text = destination.join(suffix).to_string_lossy().into_owned();
                true
            } else {
                false
            }
        }
        serde_json::Value::Array(values) => values.iter_mut().fold(false, |changed, v| {
            remap_json(v, source, destination) || changed
        }),
        serde_json::Value::Object(values) => values.values_mut().fold(false, |changed, v| {
            remap_json(v, source, destination) || changed
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remaps_owned_paths_and_preserves_resource_ids_and_external_paths() {
        let mut value = serde_json::json!({"icon": "C:/old/icon.png", "id": "@seelen/weg", "external": "D:/Photos/a.png", "similar": "C:/older/file", "nested": ["C:/old/widgets/state.json"]});
        assert!(remap_json(
            &mut value,
            Path::new("C:/old"),
            Path::new("C:/new")
        ));
        assert_eq!(
            Path::new(value["icon"].as_str().unwrap()),
            Path::new("C:/new/icon.png")
        );
        assert_eq!(value["id"], "@seelen/weg");
        assert_eq!(value["external"], "D:/Photos/a.png");
        assert_eq!(value["similar"], "C:/older/file");
    }
    #[test]
    fn copies_without_removing_the_source() {
        let root = std::env::temp_dir().join(format!("mac-ui-copy-test-{}", uuid::Uuid::new_v4()));
        let source = root.join("old");
        let dest = root.join("new");
        fs::create_dir_all(source.join("widgets")).unwrap();
        fs::write(source.join("widgets/state.json"), b"{\"pin\":true}").unwrap();
        copy_tree(&source, &dest).unwrap();
        assert_eq!(
            fs::read(source.join("widgets/state.json")).unwrap(),
            fs::read(dest.join("widgets/state.json")).unwrap()
        );
        fs::remove_file(source.join("widgets/state.json")).unwrap();
        fs::remove_file(dest.join("widgets/state.json")).unwrap();
        fs::remove_dir(source.join("widgets")).unwrap();
        fs::remove_dir(dest.join("widgets")).unwrap();
        fs::remove_dir(source).unwrap();
        fs::remove_dir(dest).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn identity_remaps_config_and_shortcuts_without_changing_external_documents() {
        let mut value = serde_json::json!({
            "byWidget": {
                "@seelen/weg": {"size": 40, "$shortcuts": {"@seelen/weg::toggle": ["Win", "D"]}},
                "@macui/weg": {"size": 48}
            },
            "command": ["widget", "trigger", "@seelen/apps-menu"],
            "owned": "C:/profile/data/seelen-weg/state.yml",
            "external": "D:/notes/seelen-my-document.txt"
        });
        assert!(remap_identity_json(&mut value, Path::new("C:/profile")));
        assert_eq!(value["byWidget"]["@macui/weg"]["size"], 48);
        assert_eq!(
            value["byWidget"]["@macui/weg"]["$shortcuts"]["@macui/weg::toggle"],
            serde_json::json!(["Win", "D"])
        );
        assert!(value["byWidget"]["@seelen/weg"].is_null());
        assert_eq!(value["command"][2], "@macui/apps-menu");
        assert_eq!(
            Path::new(value["owned"].as_str().unwrap()),
            Path::new("C:/profile/data/macui-weg/state.yml")
        );
        assert_eq!(value["external"], "D:/notes/seelen-my-document.txt");
        assert!(!remap_identity_json(&mut value, Path::new("C:/profile")));
    }

    #[test]
    fn identity_migration_backs_up_existing_data_and_runs_only_once() {
        let root =
            std::env::temp_dir().join(format!("macui-identity-test-{}", uuid::Uuid::new_v4()));
        let destination = root.join("com.jona.mac-ui");
        fs::create_dir_all(destination.join("data/seelen-apps-menu")).unwrap();
        let original = serde_json::json!({
            "byWidget": {
                "@seelen/apps-menu": {"enabled": true},
                "@seelen/desktop-shell": {"enabled": true},
                "@seelen/settings": {"colorScheme": "dark", "custom": "retained"}
            },
            "activeThemes": ["@custom/old"], "activeIconPacks": ["@custom/icons"]
        });
        fs::write(
            destination.join("settings.json"),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
        fs::write(
            destination.join("data/seelen-apps-menu/launchpad-v1.json"),
            b"{\"items\":[\"example.exe\"]}",
        )
        .unwrap();
        fs::write(
            destination.join("settings_shortcuts.json"),
            b"{\"enabled\":false,\"shortcuts\":{}}",
        )
        .unwrap();
        migrate_macui_identity(&destination).unwrap();
        assert!(destination.join(IDENTITY_MARKER).exists());
        assert!(
            destination
                .join("data/macui-apps-menu/launchpad-v1.json")
                .exists()
        );
        assert!(!destination.join("data/seelen-apps-menu").exists());
        let migrated =
            macui_core::state::Settings::load(destination.join("settings.json")).unwrap();
        assert!(migrated.is_widget_enabled(&"@macui/apps-menu".into()));
        assert!(migrated.is_widget_enabled(&"@macui/desktop-shell".into()));
        assert!(!migrated.shortcuts.enabled);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(destination.join("settings.json")).unwrap()).unwrap();
        assert_eq!(value["byWidget"]["@macui/settings"]["colorScheme"], "light");
        assert_eq!(value["byWidget"]["@macui/settings"]["custom"], "retained");
        let backups: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("macui-before-identity-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(backups[0].join("settings.json")).unwrap()
            )
            .unwrap(),
            original
        );
        assert!(
            backups[0]
                .join("data/seelen-apps-menu/launchpad-v1.json")
                .exists()
        );
        value["byWidget"]["@macui/settings"]["colorScheme"] = serde_json::json!("dark");
        fs::write(
            destination.join("settings.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        migrate_macui_identity(&destination).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(destination.join("settings.json")).unwrap()
            )
            .unwrap()["byWidget"]["@macui/settings"]["colorScheme"],
            "dark"
        );
        fs::remove_dir_all(root).unwrap(); // freshly allocated test fixture, no user data
    }

    #[test]
    fn yaml_widget_states_adopt_resource_ids() {
        let mut value: serde_yaml::Value = serde_yaml::from_str(
            "plugin: '@seelen/toolbar-media'\n'@seelen/weg':\n  enabled: true\n",
        )
        .unwrap();
        assert!(remap_identity_yaml(&mut value, Path::new("C:/profile")));
        assert_eq!(value["plugin"].as_str(), Some("@macui/toolbar-media"));
        assert_eq!(value["@macui/weg"]["enabled"].as_bool(), Some(true));
    }
}
