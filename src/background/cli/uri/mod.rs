use crate::{
    app::get_app_handle, error::Result, resources::RESOURCES, state::application::FULL_STATE,
    utils::constants::MACUI_COMMON, widgets::trigger_dialog_backend,
};
use macui_core::{
    resource::{MacUiPackage, ResourceId, ResourceKind, ResourceMetadata},
    state::{CssStyles, Dialog, DialogContent},
};
use std::path::{Path, PathBuf};
use tauri::Listener;
use uuid::Uuid;

pub const URI: &str = "mac-ui:";
// Import compatibility only: old online URIs are explicitly rejected, never fetched.
pub const LEGACY_URI: &str = "seelen.uri:";

pub async fn process_uri(uri: &str) -> Result<()> {
    // Old online links are rejected before parsing or logging authentication data.
    if uri.starts_with(URI) || uri.starts_with(LEGACY_URI) {
        return Err(
            "Online resource links are unavailable. Import a local resource file instead.".into(),
        );
    }
    let path = PathBuf::from(uri);
    if !path.is_file() || !is_local_resource_path(&path) {
        return Err("Invalid local resource file".into());
    }
    let file = MacUiPackage::load(&path).await?;
    let kind = file.kind();
    let used_id = file.resource_id()?;
    if matches!(kind, ResourceKind::Theme | ResourceKind::IconPack) {
        return Err("MAC UI uses built-in Liquid Glass and system icons".into());
    }
    let metadata = match &file {
        MacUiPackage::Legacy(file) => file.resource.metadata.clone(),
        MacUiPackage::Local(file) => serde_json::from_value::<ResourceMetadata>(
            file.data
                .0
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        )?,
    };
    let stored = store_file_on_respective_user_folder(&file).await?;
    // Resolve the imported file before showing Enable; don't rely on watcher timing.
    let load_path = if kind == ResourceKind::Wallpaper {
        stored.parent().unwrap_or(&stored)
    } else {
        &stored
    };
    RESOURCES.load(&kind, load_path).await?;
    RESOURCES.emit_kind_changed(&kind)?;
    let dialog_id = Uuid::new_v4();
    trigger_dialog_backend(Dialog {
        identifier: dialog_id,
        ..Default::default()
    })?;
    update_dialog_to_added_resource(dialog_id, kind, used_id, &metadata)
}

fn is_local_resource_path(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_string_lossy();
        ext.eq_ignore_ascii_case("maui") || ext.eq_ignore_ascii_case("slu")
    })
}

fn path_by_resource_kind(kind: &ResourceKind) -> &Path {
    match kind {
        ResourceKind::Theme => MACUI_COMMON.user_themes_path(),
        ResourceKind::IconPack => MACUI_COMMON.user_icons_path(),
        ResourceKind::Widget => MACUI_COMMON.user_widgets_path(),
        ResourceKind::Plugin => MACUI_COMMON.user_plugins_path(),
        ResourceKind::Wallpaper => MACUI_COMMON.user_wallpapers_path(),
        ResourceKind::SoundPack => MACUI_COMMON.user_sounds_path(),
    }
}

async fn store_file_on_respective_user_folder(file: &MacUiPackage) -> Result<PathBuf> {
    let kind = file.kind();
    let id = file.resource_id()?;
    let key = resource_storage_key(&id);
    let mut path_to_store = path_by_resource_kind(&kind).to_path_buf();
    if kind == ResourceKind::Wallpaper {
        path_to_store.push(key);
        tokio::fs::create_dir_all(&path_to_store).await?;
        path_to_store.push("metadata.maui");
    } else {
        path_to_store.push(format!("{key}.maui"));
    }
    file.store(&path_to_store).await?;
    Ok(path_to_store)
}

fn resource_storage_key(id: &ResourceId) -> String {
    match id {
        ResourceId::Remote(id) => id.to_string(),
        // Avoid path separators, collisions and Windows path-length limits while
        // preserving the original resource ID inside the package.
        ResourceId::Local(id) => format!(
            "macui-local-{}",
            macui_utils::checksums::calculate_sha256(id.as_bytes())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_maui_and_legacy_imports_but_rejects_other_extensions() {
        for path in ["widget.maui", "widget.MAUI", "legacy.slu"] {
            assert!(is_local_resource_path(Path::new(path)));
        }
        for path in ["widget.exe", "widget.maui.exe", "no-extension"] {
            assert!(!is_local_resource_path(Path::new(path)));
        }
    }

    #[test]
    fn local_import_storage_keys_are_safe_stable_and_distinct() {
        let first = resource_storage_key(&"@creator/my-widget".into());
        assert!(first.starts_with("macui-local-"));
        assert!(!first.contains(['/', '\\', ':']));
        assert_eq!(first, resource_storage_key(&"@creator/my-widget".into()));
        assert_ne!(first, resource_storage_key(&"@creator-my/widget".into()));
    }
}

fn update_dialog_to_added_resource(
    dialog_id: Uuid,
    kind: ResourceKind,
    used_id: ResourceId,
    metadata: &ResourceMetadata,
) -> Result<()> {
    let event = format!("resource::{dialog_id}::enable");
    let config = resource_to_dialog(dialog_id, metadata, &event)?;
    trigger_dialog_backend(config)?;

    get_app_handle().once(event, move |_| {
        RESOURCES.enable_resource(kind, used_id.clone());
    });

    Ok(())
}

fn resource_to_dialog(id: Uuid, metadata: &ResourceMetadata, enable_event: &str) -> Result<Dialog> {
    let image_styles = CssStyles::new()
        .add("width", "90px")
        .add("minWidth", "90px")
        .add("height", "90px")
        .add("borderRadius", "14px")
        .add("backgroundColor", "var(--color-gray-200)")
        .add("display", "flex")
        .add("alignItems", "center")
        .add("justifyContent", "center");

    let image = if let Some(url) = metadata
        .portrait
        .as_ref()
        .filter(|url| !matches!(url.scheme(), "http" | "https"))
    {
        DialogContent::Image {
            href: url.clone(),
            styles: Some(image_styles),
        }
    } else {
        DialogContent::Icon {
            name: "GrStatusUnknown".to_string(),
            styles: Some(image_styles),
        }
    };

    let state = FULL_STATE.load();
    let locale = state.locale();

    Ok(Dialog {
        identifier: id,
        width: 500.0,
        height: 280.0,
        title: vec![DialogContent::Group {
            items: vec![
                DialogContent::Icon {
                    name: "GrCircleInformation".to_string(),
                    styles: Some(
                        CssStyles::new()
                            .add("color", "var(--color-blue-900)")
                            .add("height", "1.2rem"),
                    ),
                },
                DialogContent::Text {
                    value: t!("resource.added").to_string(),
                    styles: None,
                },
            ],
            styles: Some(CssStyles::new().add("alignItems", "center")),
        }],
        content: vec![DialogContent::Group {
            items: vec![
                image,
                DialogContent::Group {
                    items: vec![
                        DialogContent::Text {
                            value: metadata.display_name.get(locale).to_owned(),
                            styles: Some(
                                CssStyles::new()
                                    .add("fontWeight", "bold")
                                    .add("fontSize", "2rem")
                                    .add("lineHeight", "1.2em"),
                            ),
                        },
                        DialogContent::Text {
                            value: metadata.description.get(locale).to_owned(),
                            styles: None,
                        },
                    ],
                    styles: Some(CssStyles::new().add("flexDirection", "column")),
                },
            ],
            styles: None,
        }],
        footer: vec![DialogContent::Button {
            skin: Some("solid".to_string()),
            inner: vec![DialogContent::Text {
                value: t!("resource.enable").to_string(),
                styles: None,
            }],
            on_click: enable_event.to_owned(),
            styles: None,
        }],
    })
}
