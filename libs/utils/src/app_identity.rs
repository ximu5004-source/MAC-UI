//! Public executable naming, with a fallback for existing development installs.
use std::path::{Path, PathBuf};

pub const APP_EXECUTABLE: &str = "macui.exe";
// Import/upgrade fallback only; newly built and installed binaries use macui.exe.
const LEGACY_EXECUTABLES: &[&str] = &["mac-ui.exe", "seelen-ui.exe"];

pub fn is_app_executable(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(APP_EXECUTABLE)
                || LEGACY_EXECUTABLES
                    .iter()
                    .any(|legacy| name.eq_ignore_ascii_case(legacy))
        })
}

/// Never stop an unrelated installation merely because its executable has the same name.
pub fn is_app_executable_in(path: &Path, directory: &Path) -> bool {
    is_app_executable(path)
        && path.parent().is_some_and(|parent| {
            let parent = parent
                .canonicalize()
                .unwrap_or_else(|_| parent.to_path_buf());
            let directory = directory
                .canonicalize()
                .unwrap_or_else(|_| directory.to_path_buf());
            parent
                .to_string_lossy()
                .eq_ignore_ascii_case(&directory.to_string_lossy())
        })
}

pub fn app_executable_in(directory: &Path) -> PathBuf {
    select_executable(directory, |path| path.is_file())
}

fn select_executable(directory: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let branded = directory.join(APP_EXECUTABLE);
    if exists(&branded) {
        return branded;
    }
    LEGACY_EXECUTABLES
        .iter()
        .map(|name| directory.join(name))
        .find(|legacy| exists(legacy))
        .unwrap_or(branded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_current_and_legacy_names_without_matching_other_applications() {
        assert!(is_app_executable(Path::new("mac-ui.exe")));
        assert!(is_app_executable(Path::new("MAC-UI.EXE")));
        assert!(is_app_executable(Path::new("macui.exe")));
        assert!(is_app_executable(Path::new("seelen-ui.exe")));
        assert!(!is_app_executable(Path::new("not-mac-ui.exe")));
        assert!(!is_app_executable(Path::new("macui-service.exe")));
    }

    #[test]
    fn process_ownership_requires_the_same_installation_directory() {
        assert!(is_app_executable_in(
            Path::new("C:/Apps/MAC UI/mac-ui.exe"),
            Path::new("C:/Apps/MAC UI")
        ));
        assert!(is_app_executable_in(
            Path::new("C:/Apps/MAC UI/macui.exe"),
            Path::new("C:/Apps/MAC UI")
        ));
        assert!(!is_app_executable_in(
            Path::new("C:/Apps/Other/macui.exe"),
            Path::new("C:/Apps/MAC UI")
        ));
    }

    #[test]
    fn branded_binary_wins_and_legacy_development_build_still_launches() {
        let directory = Path::new("installation");
        assert_eq!(
            select_executable(directory, |_| true),
            directory.join(APP_EXECUTABLE)
        );
        assert_eq!(
            select_executable(directory, |_| false),
            directory.join(APP_EXECUTABLE)
        );
        assert_eq!(
            select_executable(directory, |p| p.file_name().unwrap() == "mac-ui.exe"),
            directory.join("mac-ui.exe"),
        );
        assert_eq!(
            select_executable(directory, |p| p.file_name().unwrap() == "seelen-ui.exe"),
            directory.join("seelen-ui.exe"),
        );
    }
}
