use super::{Settings, ThemeSettings};

#[test]
fn default_appearance_is_fixed_liquid_glass_with_system_icons() {
    let settings = Settings::default();
    assert_eq!(settings.active_themes, Settings::fixed_glass_themes());
    assert_eq!(settings.active_icon_packs, vec!["@system/icon-pack".into()]);
}

#[test]
fn old_theme_and_icon_pack_selections_cannot_change_fixed_appearance() {
    let mut settings = Settings::default();
    settings.active_themes = vec!["@custom/red-theme".into()];
    settings.old_active_themes = vec!["old-theme.yml".into()];
    settings
        .by_theme
        .insert("@custom/red-theme".into(), ThemeSettings::default());
    settings
        .active_icon_packs
        .push("@custom/replacement-icons".into());
    settings.backup_sync_enabled = true;
    settings.drpc = true;
    settings.sanitize().unwrap();
    assert_eq!(settings.active_themes, Settings::fixed_glass_themes());
    assert_eq!(settings.active_icon_packs, vec!["@system/icon-pack".into()]);
    assert!(settings.old_active_themes.is_empty());
    assert!(settings.by_theme.is_empty());
    assert!(!settings.backup_sync_enabled);
    assert!(!settings.drpc);
    assert!(settings.shortcuts.enabled);
}
