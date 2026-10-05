//! Public executable naming, with a fallback for existing development installs.
use std::path::{Path, PathBuf};

pub const APP_EXECUTABLE: &str = "mac-ui.exe";
pub const LEGACY_EXECUTABLE: &str = "seelen-ui.exe";

pub fn is_app_executable(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(APP_EXECUTABLE)
                || name.eq_ignore_ascii_case(LEGACY_EXECUTABLE)
        })
}

pub fn app_executable_in(directory: &Path) -> PathBuf {
    select_executable(directory, |path| path.is_file())
}

fn select_executable(directory: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let branded = directory.join(APP_EXECUTABLE);
    let legacy = directory.join(LEGACY_EXECUTABLE);
    if exists(&branded) || !exists(&legacy) {
        branded
    } else {
        legacy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_both_names_without_matching_other_applications() {
        assert!(is_app_executable(Path::new("mac-ui.exe")));
        assert!(is_app_executable(Path::new("MAC-UI.EXE")));
        assert!(is_app_executable(Path::new("seelen-ui.exe")));
        assert!(!is_app_executable(Path::new("not-mac-ui.exe")));
        assert!(!is_app_executable(Path::new("slu-service.exe")));
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
            select_executable(directory, |p| p.file_name().unwrap() == LEGACY_EXECUTABLE),
            directory.join(LEGACY_EXECUTABLE),
        );
    }
}
