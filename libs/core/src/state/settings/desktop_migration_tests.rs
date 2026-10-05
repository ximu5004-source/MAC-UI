use super::{Settings, WallpaperCollection};

#[test]
fn legacy_desktop_becomes_independent_without_losing_wallpapers() {
    let mut settings = Settings::default();
    settings.by_widget.wall.desktop_shell_enabled = true;
    settings.by_widget.wall.enabled = true;
    let collection = WallpaperCollection {
        id: uuid::Uuid::new_v4(),
        name: "Keep me".into(),
        wallpapers: vec![],
        hidden: false,
    };
    settings.by_widget.wall.default_collection = Some(collection.id);
    settings.wallpaper_collections.push(collection.clone());
    settings.migrate().unwrap();
    assert!(settings.is_widget_enabled(&"@seelen/desktop-shell".into()));
    assert!(!settings.by_widget.wall.enabled);
    assert!(!settings.by_widget.wall.desktop_shell_enabled);
    assert_eq!(
        settings.by_widget.wall.default_collection,
        Some(collection.id)
    );
    assert_eq!(settings.wallpaper_collections.len(), 1);
    settings.migrate().unwrap();
    assert!(settings.is_widget_enabled(&"@seelen/desktop-shell".into()));
}

#[test]
fn independent_switches_never_override_each_other() {
    let mut settings = Settings::default();
    settings.by_widget.wall.desktop_shell_enabled = true;
    settings.set_widget_enabled(&"@seelen/desktop-shell".into(), false);
    settings.migrate().unwrap();
    assert!(!settings.is_widget_enabled(&"@seelen/desktop-shell".into()));
    assert!(settings.by_widget.wall.enabled);
    for desktop in [false, true] {
        for wallpaper in [false, true] {
            settings.set_widget_enabled(&"@seelen/desktop-shell".into(), desktop);
            settings.by_widget.wall.enabled = wallpaper;
            settings.migrate().unwrap();
            settings.sanitize().unwrap();
            assert_eq!(
                settings.is_widget_enabled(&"@seelen/desktop-shell".into()),
                desktop
            );
            assert_eq!(settings.by_widget.wall.enabled, wallpaper);
        }
    }
}

#[test]
fn fresh_users_do_not_lose_native_desktop() {
    let mut settings = Settings::default();
    assert!(!settings.is_widget_enabled(&"@seelen/desktop-shell".into()));
    settings.migrate().unwrap();
    assert!(!settings.is_widget_enabled(&"@seelen/desktop-shell".into()));
}

#[test]
fn desktop_glass_preferences_survive_save_load_without_enabling_desktop() {
    let mut source = serde_json::to_value(Settings::default()).unwrap();
    let appearance = serde_json::json!({
        "dockTransparency": 42, "dockGloss": 80,
        "iconEdgeTransparency": 73, "iconEdgeGloss": 56
    });
    source["byWidget"]["@seelen/desktop-shell"] = serde_json::json!({
        "enabled": false, "glassAppearance": appearance, "unrelatedPreference": 123
    });
    let mut settings: Settings = serde_json::from_value(source).unwrap();
    settings.sanitize().unwrap();
    let saved = serde_json::to_value(settings).unwrap();
    assert_eq!(
        saved["byWidget"]["@seelen/desktop-shell"]["glassAppearance"],
        appearance
    );
    assert_eq!(
        saved["byWidget"]["@seelen/desktop-shell"]["unrelatedPreference"],
        123
    );
    let reloaded: Settings = serde_json::from_value(saved).unwrap();
    assert!(!reloaded.is_widget_enabled(&"@seelen/desktop-shell".into()));
}
