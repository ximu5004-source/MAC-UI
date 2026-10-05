use std::{collections::HashSet, path::PathBuf, sync::Once};

use seelen_core::{
    handlers::SeelenEvent,
    system_state::{DesktopEntry, DesktopNewFileType, FolderChangedArgs, FolderType},
};
use tauri::Manager;

use crate::{
    app::{emit_to_webviews, get_app_handle},
    error::Result,
    windows_api::WindowsApi,
};

use super::application::{UserFoldersManager, UserFoldersManagerEvent};

fn register_folder_events() {
    static TAURI_EVENT_REGISTRATION: Once = Once::new();
    TAURI_EVENT_REGISTRATION.call_once(|| {
        UserFoldersManager::subscribe(|event| match event {
            UserFoldersManagerEvent::FolderChanged(folder) => {
                emit_to_webviews(
                    SeelenEvent::UserFolderChanged,
                    FolderChangedArgs {
                        of_folder: folder,
                        content: get_user_folder_content(folder),
                    },
                );
            }
        });
    });
}

/// Returns false and kicks off the initialization out of the caller thread when the
/// manager isn't built yet, see [`UserFoldersManager::init_in_background`].
fn ensure_folders_manager() -> bool {
    if UserFoldersManager::is_initialized() {
        return true;
    }
    register_folder_events();
    UserFoldersManager::init_in_background();
    false
}

#[tauri::command(async)]
pub fn get_user_folder_content(folder_type: FolderType) -> Vec<PathBuf> {
    // same as `get_user`, `SeelenEvent::UserFolderChanged` will bring the real content
    // once the background indexing finishes
    if !ensure_folders_manager() {
        return Vec::new();
    }
    let manager = UserFoldersManager::instance().lock();
    match manager.folders.get(&folder_type) {
        Some(details) => details.content.clone(),
        None => Vec::new(),
    }
}

pub(super) fn desktop_dir() -> Result<PathBuf> {
    Ok(get_app_handle().path().desktop_dir()?)
}

fn desktop_entry(path: PathBuf) -> Option<DesktopEntry> {
    let metadata = std::fs::metadata(&path).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    if name.eq_ignore_ascii_case("desktop.ini") {
        return None;
    }

    let is_application = crate::modules::apps::activation::is_application_file(&path);
    Some(DesktopEntry {
        is_application,
        extension: path
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase()),
        path,
        name,
        is_directory: metadata.is_dir(),
    })
}

/// Returns the direct children of the Windows desktop, including folders.
/// This deliberately differs from the recursive known-folder index used by search surfaces.
#[tauri::command(async)]
pub fn get_desktop_entries() -> Vec<DesktopEntry> {
    register_folder_events();
    UserFoldersManager::init_in_background();

    let Ok(desktop) = desktop_dir() else {
        return Vec::new();
    };

    let mut roots = vec![desktop];
    if let Ok(public) = WindowsApi::known_folder(windows::Win32::UI::Shell::FOLDERID_PublicDesktop)
    {
        roots.push(public);
    }
    let mut names = HashSet::new();
    roots
        .into_iter()
        .flat_map(|root| std::fs::read_dir(root).into_iter().flatten())
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| desktop_entry(entry.path()))
        .filter(|entry| names.insert(entry.name.to_lowercase()))
        .collect()
}

fn validate_desktop_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if !slu_utils::new_file::valid_name(name) {
        return Err("Invalid desktop entry name".into());
    }
    Ok(name)
}

#[tauri::command(async)]
pub fn get_desktop_new_file_types() -> Vec<DesktopNewFileType> {
    super::new_files::available_types()
}

#[tauri::command(async)]
pub fn create_desktop_file(extension: String, base_name: String) -> Result<PathBuf> {
    let base_name = validate_desktop_name(&base_name)?;
    super::new_files::create(&extension, base_name)
}

#[tauri::command(async)]
pub fn create_desktop_folder(base_name: String) -> Result<PathBuf> {
    let base_name = validate_desktop_name(&base_name)?;
    let desktop = desktop_dir()?;
    let mut candidate = desktop.join(base_name);
    let mut suffix = 2;
    while candidate.exists() {
        candidate = desktop.join(format!("{base_name} ({suffix})"));
        suffix += 1;
    }
    std::fs::create_dir(&candidate)?;
    Ok(candidate)
}

#[tauri::command(async)]
pub fn rename_desktop_entry(path: PathBuf, new_name: String) -> Result<PathBuf> {
    let new_name = validate_desktop_name(&new_name)?;
    let desktop = desktop_dir()?.canonicalize()?;
    let parent = path
        .parent()
        .ok_or("Desktop entry has no parent")?
        .canonicalize()?;
    if parent != desktop {
        return Err("Only direct desktop entries can be renamed".into());
    }

    let target = desktop.join(new_name);
    if target.exists() && target != path {
        return Err("A desktop entry with this name already exists".into());
    }
    std::fs::rename(&path, &target)?;
    Ok(target)
}
