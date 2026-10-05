use std::path::{Component, Path};

/// Read-only grouping metadata; never resolve, move or write a shortcut.
pub fn folder_segments(path: &Path, programs_root: &Path) -> Vec<String> {
    let Some(relative) = path
        .parent()
        .and_then(|parent| parent.strip_prefix(programs_root).ok())
    else {
        return Vec::new();
    };
    relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_nested_and_unicode_folders_without_programs_wrapper() {
        let root = Path::new("C:/Start/Programs");
        assert_eq!(
            folder_segments(&root.join("工具/绘图/App.lnk"), root),
            ["工具", "绘图"]
        );
        assert!(folder_segments(&root.join("App.lnk"), root).is_empty());
    }

    #[test]
    fn ignores_files_outside_the_known_programs_root() {
        assert!(
            folder_segments(
                Path::new("C:/Elsewhere/App.lnk"),
                Path::new("C:/Start/Programs")
            )
            .is_empty()
        );
    }
}
