use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Widget;

/// Declaration for a system-level shortcut (not attached to any specific widget definition).
/// Hardcoded in Rust; exposed to the frontend via the `StateGetSystemShortcuts` command.
/// The user can override keys via `SluShortcutsSettings.shortcuts`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct SystemShortcutDeclaration {
    pub id: String,
    pub command: Vec<String>,
    pub label: String,
    pub default_keys: Vec<String>,
    /// If true, user cannot change the keys for this shortcut.
    pub readonly: bool,
}

/// Minimal struct sent to the service after the background has resolved all shortcut overrides.
/// Contains only what the service needs to register hotkeys — no widget info, no enums.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct ResolvedShortcut {
    pub command: Vec<String>,
    pub keys: Vec<String>,
}

/// User-facing shortcut settings.
/// - `enabled`: global on/off toggle.
/// - `shortcuts`: key overrides for **system-level** shortcut declarations (`id -> keys`).
///   Widget shortcut overrides live inside each widget's `$shortcuts` in `SettingsByWidget`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct SluShortcutsSettings {
    pub enabled: bool,
    pub shortcuts: HashMap<String, Vec<String>>,
}

impl Default for SluShortcutsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: HashMap::new(),
        }
    }
}

// =====================================================================
// Helpers
// =====================================================================

macro_rules! cmd {
    ($($arg:expr),+ $(,)?) => {
        vec![$($arg.to_string()),+]
    };
}

macro_rules! decl {
    ($id:expr, $label:expr, $command:expr, $default_keys:expr, readonly) => {
        SystemShortcutDeclaration {
            id: $id.to_string(),
            label: $label.to_string(),
            command: $command,
            default_keys: $default_keys,
            readonly: true,
        }
    };
    ($id:expr, $label:expr, $command:expr, $default_keys:expr) => {
        SystemShortcutDeclaration {
            id: $id.to_string(),
            label: $label.to_string(),
            command: $command,
            default_keys: $default_keys,
            readonly: false,
        }
    };
}

// =====================================================================
// System shortcut declarations (hardcoded defaults)
// =====================================================================

/// Returns all hardcoded system-level shortcut declarations.
/// These are shortcuts that are either:
/// - Not tied to any single widget (VD system, misc), or
/// - System overrides that must not be freely editable (dock Win+N).
pub fn system_shortcut_declarations() -> Vec<SystemShortcutDeclaration> {
    // ---- Virtual Desktop system overrides (readonly) ----
    let mut decls = vec![
        decl!(
            "vd-create-workspace",
            "t:shortcuts.labels.create_new_workspace",
            cmd!["vd", "create-new-workspace"],
            cmd!["Ctrl", "Win", "D"],
            readonly
        ),
        decl!(
            "vd-create-workspace-row",
            "t:shortcuts.labels.create_new_workspace_row",
            cmd!["vd", "create-new-workspace-row"],
            cmd!["Ctrl", "Win", "Shift", "D"],
            readonly
        ),
        decl!(
            "vd-destroy-workspace",
            "t:shortcuts.labels.destroy_current_workspace",
            cmd!["vd", "destroy-current-workspace"],
            cmd!["Ctrl", "Win", "F4"],
            readonly
        ),
        decl!(
            "vd-switch-to-next",
            "t:shortcuts.labels.switch_to_next_workspace",
            cmd!["vd", "switch-to", "right"],
            cmd!["Ctrl", "Win", "Right"],
            readonly
        ),
        decl!(
            "vd-switch-to-prev",
            "t:shortcuts.labels.switch_to_previous_workspace",
            cmd!["vd", "switch-to", "left"],
            cmd!["Ctrl", "Win", "Left"],
            readonly
        ),
        decl!(
            "vd-switch-to-up",
            "t:shortcuts.labels.switch_to_workspace_up",
            cmd!["vd", "switch-to", "up"],
            cmd!["Ctrl", "Win", "Up"],
            readonly
        ),
        decl!(
            "vd-switch-to-down",
            "t:shortcuts.labels.switch_to_workspace_down",
            cmd!["vd", "switch-to", "down"],
            cmd!["Ctrl", "Win", "Down"],
            readonly
        ),
        decl!(
            "vd-send-to-next",
            "t:shortcuts.labels.send_to_next_workspace",
            cmd!["vd", "send-to", "right"],
            cmd!["Shift", "Win", "Right"],
            readonly
        ),
        decl!(
            "vd-send-to-prev",
            "t:shortcuts.labels.send_to_previous_workspace",
            cmd!["vd", "send-to", "left"],
            cmd!["Shift", "Win", "Left"],
            readonly
        ),
        decl!(
            "vd-send-to-up",
            "t:shortcuts.labels.send_to_workspace_up",
            cmd!["vd", "send-to", "up"],
            cmd!["Shift", "Win", "Up"],
            readonly
        ),
        decl!(
            "vd-send-to-down",
            "t:shortcuts.labels.send_to_workspace_down",
            cmd!["vd", "send-to", "down"],
            cmd!["Shift", "Win", "Down"],
            readonly
        ),
    ];

    // ---- Workspace switch/move/send (user-configurable) ----
    for index in 0..10usize {
        let digit = if index == 9 {
            "0".to_string()
        } else {
            format!("{}", index + 1)
        };
        decls.push(SystemShortcutDeclaration {
            id: format!("vd-switch-to-{}", index),
            label: format!("t:shortcuts.labels.switch_workspace:{}", index + 1),
            command: cmd!["vd", "switch-to", index],
            default_keys: vec!["Alt".to_string(), digit.clone()],
            readonly: false,
        });
        decls.push(SystemShortcutDeclaration {
            id: format!("vd-move-to-{}", index),
            label: format!("t:shortcuts.labels.move_to_workspace:{}", index + 1),
            command: cmd!["vd", "move-to", index],
            default_keys: vec!["Alt".to_string(), "Shift".to_string(), digit.clone()],
            readonly: false,
        });
        decls.push(SystemShortcutDeclaration {
            id: format!("vd-send-to-{}", index),
            label: format!("t:shortcuts.labels.send_to_workspace:{}", index + 1),
            command: cmd!["vd", "send-to", index],
            default_keys: vec!["Win".to_string(), "Shift".to_string(), digit],
            readonly: false,
        });
    }

    // ---- Misc (readonly) ----
    decls.push(decl!(
        "service-force-restart",
        "t:shortcuts.labels.misc_force_restart",
        cmd!["service", "force-restart"],
        cmd!["Ctrl", "Win", "Alt", "R"],
        readonly
    ));
    decls.push(decl!(
        "service-force-quit",
        "t:shortcuts.labels.misc_force_quit",
        cmd!["service", "force-quit"],
        cmd!["Ctrl", "Win", "Alt", "K"],
        readonly
    ));
    decls.push(decl!(
        "shortcuts-pause-toggle",
        "t:shortcuts.labels.pause_toggle",
        cmd!["toggle-shortcuts-pause"],
        cmd!["Ctrl", "Win", "Alt", "P"]
    ));

    decls
}

// =====================================================================
// Resolution
// =====================================================================

use super::Settings;

/// Resolves all shortcut declarations (widget-declared + system-hardcoded) against
/// user-configured overrides, respecting widget enabled state.
///
/// Returns a flat list of `ResolvedShortcut` ready to be sent to the service, and a
/// boolean that is `true` when at least two entries share the same key combination.
/// Returns an empty `Vec` (and `false`) if `settings.shortcuts.enabled` is false.
pub fn resolve_shortcuts(
    settings: &Settings,
    widgets: &[&Widget],
) -> (Vec<ResolvedShortcut>, bool) {
    if !settings.shortcuts.enabled {
        return (vec![], false);
    }

    let mut resolved = Vec::new();

    // 1. Widget-declared shortcuts
    for widget in widgets {
        if !settings.is_widget_enabled(&widget.id) {
            continue;
        }

        let overrides = settings.by_widget.get_shortcut_overrides(&widget.id);
        for decl in &widget.shortcuts {
            let keys = if decl.readonly {
                decl.default_keys.clone()
            } else {
                overrides
                    .get(&decl.id)
                    .cloned()
                    .unwrap_or_else(|| decl.default_keys.clone())
            };
            if keys.is_empty() {
                continue;
            }
            resolved.push(ResolvedShortcut {
                command: decl.command.clone(),
                keys,
            });
        }
    }

    // 2. System-level declarations
    let system_overrides = &settings.shortcuts.shortcuts;
    for decl in system_shortcut_declarations() {
        let keys = system_overrides
            .get(&decl.id)
            .cloned()
            .unwrap_or_else(|| decl.default_keys.clone());

        if keys.is_empty() {
            continue;
        }

        resolved.push(ResolvedShortcut {
            command: decl.command.clone(),
            keys,
        });
    }

    // Detect duplicate key combinations across all resolved entries.
    let has_conflicts = {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        resolved.iter().any(|r| {
            let normalized = r
                .keys
                .iter()
                .map(|k| k.to_lowercase())
                .collect::<Vec<_>>()
                .join("+");
            !seen.insert(normalized)
        })
    };

    (resolved, has_conflicts)
}

#[cfg(test)]
mod native_start_tests {
    use super::*;
    use crate::state::widget::WidgetShortcutDeclaration;

    fn launchpad() -> Widget {
        Widget {
            id: "@seelen/apps-menu".into(),
            shortcuts: vec![WidgetShortcutDeclaration {
                id: "apps-menu-toggle".into(),
                command: cmd!["widget", "trigger", "@seelen/apps-menu"],
                default_keys: cmd!["Win"],
                readonly: true,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn absent_or_disabled_launchpad_leaves_windows_key_native() {
        let widget = launchpad();
        let mut settings = Settings::default();
        for sanitize in [false, true] {
            if sanitize {
                settings.sanitize().unwrap();
            }
            assert!(!settings.is_widget_enabled(&widget.id));
            let (resolved, _) = resolve_shortcuts(&settings, &[&widget]);
            assert!(resolved.iter().all(|s| s.keys != cmd!["Win"]));
            // Other shortcuts remain available; do not disable the whole service.
            assert!(
                resolved
                    .iter()
                    .any(|s| s.keys == cmd!["Ctrl", "Win", "Alt", "P"])
            );
        }
    }

    #[test]
    fn disabling_launchpad_releases_only_its_shortcuts() {
        let widget = launchpad();
        let mut settings = Settings::default();
        settings.set_widget_enabled(&widget.id, true);
        settings.sanitize().unwrap();
        let (enabled, _) = resolve_shortcuts(&settings, &[&widget]);
        assert_eq!(enabled.iter().filter(|s| s.keys == cmd!["Win"]).count(), 1);
        settings.set_widget_enabled(&widget.id, false);
        let (disabled, _) = resolve_shortcuts(&settings, &[&widget]);
        assert_eq!(disabled.len() + 1, enabled.len());
        assert!(
            disabled
                .iter()
                .all(|s| !s.command.contains(&widget.id.to_string()))
        );
    }

    #[test]
    fn editable_launchpad_win_default_and_user_override_are_respected() {
        let mut widget = launchpad();
        widget.shortcuts[0].readonly = false;
        let mut settings = Settings::default();
        settings.set_widget_enabled(&widget.id, true);
        settings.sanitize().unwrap();
        let (resolved, _) = resolve_shortcuts(&settings, &[&widget]);
        assert_eq!(resolved.iter().filter(|s| s.keys == cmd!["Win"]).count(), 1);
        let mut source = serde_json::to_value(settings).unwrap();
        source["byWidget"]["@seelen/apps-menu"]["$shortcuts"] =
            serde_json::json!({ "apps-menu-toggle": [] });
        let settings: Settings = serde_json::from_value(source).unwrap();
        let (resolved, _) = resolve_shortcuts(&settings, &[&widget]);
        assert!(resolved.iter().all(|s| s.keys != cmd!["Win"]));
        assert!(resolved.iter().all(|s| !s.command.contains(&widget.id.to_string())));
        assert!(resolved.iter().any(|s| s.keys == cmd!["Ctrl", "Win", "Alt", "P"]));
    }

    #[test]
    fn native_start_preference_survives_roundtrip_and_sanitize() {
        let mut source = serde_json::to_value(Settings::default()).unwrap();
        source["byWidget"]["@seelen/apps-menu"] = serde_json::json!({
            "enabled": false, "savedPreference": "keep", "$shortcuts": {"apps-menu-toggle": ["Win"]}
        });
        let mut settings: Settings = serde_json::from_value(source).unwrap();
        settings.migrate().unwrap();
        settings.sanitize().unwrap();
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            saved["byWidget"]["@seelen/apps-menu"]["savedPreference"],
            "keep"
        );
        assert!(!settings.is_widget_enabled(&launchpad().id));
        let (resolved, _) = resolve_shortcuts(&settings, &[&launchpad()]);
        assert!(resolved.iter().all(|s| s.keys != cmd!["Win"]));
    }
}
