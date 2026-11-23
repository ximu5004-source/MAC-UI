use std::path::Path;

/// Legacy service names are accepted only within this non-MSIX installation.
/// WindowsApps is a shared alias directory, so aliases require an exact match.
pub(super) fn is_owned_service_action(
    action_path: &Path,
    current_service: &Path,
    allow_legacy_names: bool,
) -> bool {
    let equal = |left: &Path, right: &Path| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    };
    if equal(action_path, current_service) {
        return true;
    }
    if !allow_legacy_names {
        return false;
    }
    let Some(name) = action_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !["slu-service.exe", "mac-ui-service.exe"]
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
    {
        return false;
    }
    match (action_path.parent(), current_service.parent()) {
        (Some(old), Some(current)) => equal(old, current),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_same_installation_rename_but_rejects_another_installation() {
        let current = Path::new("C:/Apps/MAC UI/macui-service.exe");
        assert!(is_owned_service_action(current, current, true));
        assert!(is_owned_service_action(
            Path::new("C:/Apps/MAC UI/slu-service.exe"),
            current,
            true
        ));
        assert!(!is_owned_service_action(
            Path::new("C:/Apps/Other/slu-service.exe"),
            current,
            true
        ));
        assert!(!is_owned_service_action(
            Path::new("C:/Apps/MAC UI/other-service.exe"),
            current,
            true
        ));
    }

    #[test]
    fn shared_windowsapps_aliases_are_never_adopted_by_filename_only() {
        let current = Path::new("C:/Local/Microsoft/WindowsApps/macui-service.exe");
        assert!(!is_owned_service_action(
            Path::new("C:/Local/Microsoft/WindowsApps/slu-service.exe"),
            current,
            false
        ));
        assert!(is_owned_service_action(current, current, false));
    }
}
