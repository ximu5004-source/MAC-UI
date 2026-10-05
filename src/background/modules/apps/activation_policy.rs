/// A Dock click is not a tray click: many tray icons deliberately open menus
/// even for the left button. Only known messaging clients use tray restoration.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RestoreRoute {
    Window,
    MessagingTray,
    AmbiguousMessagingTray,
    CheckMessagingProcess,
    Launch,
}

pub(crate) fn restore_route(executable: &str, has_window: bool, tray_icons: usize) -> RestoreRoute {
    if has_window {
        return RestoreRoute::Window;
    }
    let is_messaging = ["wechat.exe", "weixin.exe", "qq.exe", "wxwork.exe"]
        .iter()
        .any(|name| executable.eq_ignore_ascii_case(name));
    if !is_messaging {
        return RestoreRoute::Launch;
    }
    match tray_icons {
        0 => RestoreRoute::CheckMessagingProcess,
        1 => RestoreRoute::MessagingTray,
        _ => RestoreRoute::AmbiguousMessagingTray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_primary_click_never_becomes_an_arbitrary_tray_left_click() {
        for executable in ["ChatGPT.exe", "PixPin.exe", "OneDrive.exe", "unknown.exe"] {
            assert_eq!(restore_route(executable, false, 1), RestoreRoute::Launch);
            assert_eq!(restore_route(executable, false, 2), RestoreRoute::Launch);
        }
    }

    #[test]
    fn existing_window_always_wins_over_tray_callbacks() {
        for executable in ["ChatGPT.exe", "Weixin.exe", "QQ.exe"] {
            assert_eq!(restore_route(executable, true, 1), RestoreRoute::Window);
            assert_eq!(restore_route(executable, true, 2), RestoreRoute::Window);
        }
    }

    #[test]
    fn messaging_restore_preserves_multi_account_and_login_safety() {
        assert_eq!(
            restore_route("WECHAT.EXE", false, 1),
            RestoreRoute::MessagingTray
        );
        assert_eq!(
            restore_route("Weixin.exe", false, 2),
            RestoreRoute::AmbiguousMessagingTray
        );
        assert_eq!(
            restore_route("wxwork.exe", false, 0),
            RestoreRoute::CheckMessagingProcess
        );
    }
}
