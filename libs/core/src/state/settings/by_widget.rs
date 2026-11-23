use std::collections::HashMap;

use schemars::JsonSchema;
use uuid::Uuid;

use crate::{resource::WidgetId, utils::TsUnknown};

use super::{FancyToolbarSettings, MacUiWallSettings, MacUiWegSettings, WindowManagerSettings};

const SYSTEM_WIDGET_IDS: &[&str] = &[
    "@macui/settings",
    "@macui/dialog",
    "@macui/context-menu",
    "@macui/tooltip",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default)]
pub struct SettingsByWidget {
    #[serde(rename = "@macui/weg")]
    pub weg: MacUiWegSettings,
    #[serde(rename = "@macui/fancy-toolbar")]
    pub fancy_toolbar: FancyToolbarSettings,
    #[serde(rename = "@macui/window-manager")]
    pub wm: WindowManagerSettings,
    #[serde(rename = "@macui/wallpaper-manager")]
    pub wall: MacUiWallSettings,
    #[serde(flatten)]
    pub others: HashMap<WidgetId, GenericWidgetSettings>,
}

impl SettingsByWidget {
    /// Returns the user-configured key overrides (`shortcut_id -> keys`) for the given widget.
    pub fn get_shortcut_overrides(&self, widget_id: &WidgetId) -> &HashMap<String, Vec<String>> {
        static EMPTY_SHORTCUTS: std::sync::LazyLock<HashMap<String, Vec<String>>> =
            std::sync::LazyLock::new(HashMap::new);

        let shortcuts = match widget_id.as_str() {
            "@macui/weg" => self.weg.shortcuts.as_ref(),
            "@macui/fancy-toolbar" => self.fancy_toolbar.shortcuts.as_ref(),
            "@macui/window-manager" => self.wm.shortcuts.as_ref(),
            "@macui/wallpaper-manager" => self.wall.shortcuts.as_ref(),
            _ => self
                .others
                .get(widget_id)
                .and_then(|s| s.shortcuts.as_ref()),
        };

        shortcuts.unwrap_or(&EMPTY_SHORTCUTS)
    }

    pub fn is_enabled(&self, widget_id: &WidgetId) -> bool {
        match widget_id.as_str() {
            "@macui/weg" => self.weg.enabled,
            "@macui/fancy-toolbar" => self.fancy_toolbar.enabled,
            "@macui/window-manager" => self.wm.enabled,
            "@macui/wallpaper-manager" => self.wall.enabled,
            // Shell replacements are opt-in; an absent entry must preserve Windows behavior.
            "@macui/desktop-shell" | "@macui/apps-menu" => {
                self.others.get(widget_id).is_some_and(|s| s.enabled)
            }
            _ => match self.others.get(widget_id) {
                Some(settings) => settings.enabled,
                // only official widgets are enabled by default
                None => widget_id.starts_with("@macui"),
            },
        }
    }

    pub fn set_enabled(&mut self, widget_id: &WidgetId, enabled: bool) {
        match widget_id.as_str() {
            "@macui/weg" => self.weg.enabled = enabled,
            "@macui/fancy-toolbar" => self.fancy_toolbar.enabled = enabled,
            "@macui/window-manager" => self.wm.enabled = enabled,
            "@macui/wallpaper-manager" => self.wall.enabled = enabled,
            _ => match self.others.entry(widget_id.clone()) {
                std::collections::hash_map::Entry::Occupied(mut o) => {
                    o.get_mut().enabled = enabled;
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(GenericWidgetSettings {
                        enabled,
                        ..Default::default()
                    });
                }
            },
        }
    }

    /// System widgets can not be disabled. If a system widget is present in `others`
    /// (e.g. leftover from an older config), force it back to enabled; otherwise do nothing.
    pub fn sanitize(&mut self) {
        // Unlike passive bundled widgets, a desktop replacement must be explicitly enabled.
        self.others
            .entry("@macui/desktop-shell".into())
            .or_default();
        self.others.entry("@macui/apps-menu".into()).or_default();
        for id in SYSTEM_WIDGET_IDS {
            if let Some(settings) = self.others.get_mut(&WidgetId::from(*id)) {
                settings.enabled = true;
            }
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(
    all(feature = "gen-binds", not(feature = "salvo")),
    derive(ts_rs::TS),
    ts(export)
)]
#[serde(default)]
pub struct GenericWidgetSettings {
    /// Enable or disable the widget
    pub enabled: bool,
    /// By intance will be used to store settings in case of multiple instances allowed on widget.\
    /// The map values will be merged with the root object and default values on settings declaration.
    #[serde(rename = "$instances")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(optional = nullable))]
    pub instances: Option<HashMap<Uuid, HashMap<String, TsUnknown>>>,
    /// Key overrides for widget-declared shortcuts (`shortcut_id -> keys`).
    /// When present, replaces the shortcut's `defaultKeys` for this widget instance.
    #[serde(rename = "$shortcuts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(optional = nullable))]
    pub shortcuts: Option<HashMap<String, Vec<String>>>,
    #[serde(flatten)]
    pub rest: HashMap<String, TsUnknown>,
}
