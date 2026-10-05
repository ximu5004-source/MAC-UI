//! Read-only support for Windows ShellNew file templates. Command/COM handlers
//! require their own creation wizard and must never be replaced by empty files.
use std::path::PathBuf;

use seelen_core::system_state::DesktopNewFileType;
use slu_utils::new_file::{create_unique_file, valid_extension};
use windows::Win32::UI::Shell::{FOLDERID_CommonTemplates, FOLDERID_Templates, FOLDERID_Windows};
use winreg::{
    RegKey,
    enums::{HKEY_CLASSES_ROOT, REG_BINARY},
};

use crate::{
    error::Result,
    windows_api::{WindowsApi, string_utils::WindowsString},
};

enum Template {
    Empty,
    Data(Vec<u8>),
    File(PathBuf),
}

fn template_file(name: &str) -> Option<PathBuf> {
    let expanded =
        WindowsApi::resolve_environment_variables(&WindowsString::from_str(name)).ok()?;
    let path = PathBuf::from(expanded.to_string());
    if path.is_absolute() {
        return path.is_file().then_some(path);
    }
    // ShellNew also commonly registers a template basename, not a full path.
    if path.components().count() != 1 {
        return None;
    }
    for folder in [
        FOLDERID_Templates,
        FOLDERID_CommonTemplates,
        FOLDERID_Windows,
    ] {
        if let Ok(root) = WindowsApi::known_folder(folder) {
            for candidate in [root.join(&path), root.join("ShellNew").join(&path)] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn read_template(key: &RegKey) -> Option<Template> {
    if ["Command", "Handler", "Directory"]
        .iter()
        .any(|name| key.get_raw_value(name).is_ok())
        || key
            .open_subkey("Config")
            .is_ok_and(|config| config.get_raw_value("NoExtension").is_ok())
    {
        return None;
    }
    // Documented Windows precedence: NullFile > FileName > Data.
    if key.get_raw_value("NullFile").is_ok() {
        return Some(Template::Empty);
    }
    if let Ok(name) = key.get_value::<String, _>("FileName") {
        return template_file(&name).map(Template::File);
    }
    key.get_raw_value("Data")
        .ok()
        .filter(|value| value.vtype == REG_BINARY)
        .map(|value| Template::Data(value.bytes))
}

fn registered_template(extension: &str) -> Option<(DesktopNewFileType, Template)> {
    if !valid_extension(extension) {
        return None;
    }
    let classes = RegKey::predef(HKEY_CLASSES_ROOT);
    let association = classes.open_subkey(extension).ok()?;
    let prog_id = association.get_value::<String, _>("").unwrap_or_default();
    let mut keys = vec![format!("{extension}\\ShellNew")];
    if !prog_id.is_empty() {
        keys.push(format!("{extension}\\{prog_id}\\ShellNew"));
    }
    for path in keys {
        let Ok(key) = classes.open_subkey(path) else {
            continue;
        };
        let Some(template) = read_template(&key) else {
            continue;
        };
        let program = classes.open_subkey(&prog_id).ok();
        let label = key
            .get_value::<String, _>("ItemName")
            .ok()
            .or_else(|| {
                program
                    .as_ref()
                    .and_then(|key| key.get_value::<String, _>("FriendlyTypeName").ok())
            })
            .or_else(|| {
                program
                    .as_ref()
                    .and_then(|key| key.get_value::<String, _>("").ok())
            })
            .and_then(|text| {
                if text.starts_with('@') {
                    WindowsApi::resolve_indirect_string(&text).ok()
                } else {
                    Some(text)
                }
            })
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| extension.to_uppercase());
        return Some((
            DesktopNewFileType {
                extension: extension.to_owned(),
                label,
            },
            template,
        ));
    }
    None
}

pub fn available_types() -> Vec<DesktopNewFileType> {
    let classes = RegKey::predef(HKEY_CLASSES_ROOT);
    let mut types: Vec<_> = classes
        .enum_keys()
        .filter_map(|key| key.ok())
        .filter(|extension| {
            valid_extension(extension)
                && ![".txt", ".md"].contains(&extension.to_lowercase().as_str())
        })
        .filter_map(|extension| registered_template(&extension).map(|(item, _)| item))
        .collect();
    types.sort_by(|a, b| a.label.cmp(&b.label).then(a.extension.cmp(&b.extension)));
    types
}

pub fn create(extension: &str, base_name: &str) -> Result<PathBuf> {
    let extension = extension.to_lowercase();
    let template = match extension.as_str() {
        ".txt" | ".md" => Template::Empty,
        _ => {
            registered_template(&extension)
                .ok_or("This file template is no longer available")?
                .1
        }
    };
    // Load before creating the destination: a missing/unreadable template leaves no broken file.
    let data = match template {
        Template::Empty => Vec::new(),
        Template::Data(data) => data,
        Template::File(path) => std::fs::read(path)?,
    };
    Ok(create_unique_file(
        &super::infrastructure::desktop_dir()?,
        base_name,
        &extension,
        &data,
    )?)
}
