//! Bring back a tray-resident application before launching another process.
use crate::{
    error::Result,
    modules::system_tray::SystemTrayManager,
    windows_api::{WindowEnumerator, WindowsApi, window::Window},
};
use seelen_core::system_state::SystrayIconAction;
use std::path::{Path, PathBuf};
use windows::Win32::UI::WindowsAndMessaging::{SW_RESTORE, SW_SHOW, WS_CAPTION, WS_EX_TOOLWINDOW};

#[path = "activation_policy.rs"]
mod policy;
use policy::{RestoreRoute, restore_route};

fn executable(path: &Path) -> Option<PathBuf> {
    let path = if path.extension()?.eq_ignore_ascii_case("lnk") {
        WindowsApi::resolve_lnk_target(path).ok()?.0
    } else {
        path.to_owned()
    };
    path.extension()?
        .eq_ignore_ascii_case("exe")
        .then_some(path)
}

/// Read-only classification shared by desktop and Dock. A folder/document
/// shortcut must not acquire application-only artwork styling.
pub fn is_application_file(path: &Path) -> bool {
    path.is_file()
        && (path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("appref-ms"))
            || executable(path).is_some_and(|target| target.is_file()))
}

fn same_executable(left: &Path, right: &Path) -> bool {
    fn normalized(path: &Path) -> String {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .to_lowercase()
    }
    normalized(left) == normalized(right)
}

pub fn activate_existing(path: &Path, relaunch: Option<&Path>) -> Result<bool> {
    let Some(target) = relaunch.and_then(executable).or_else(|| executable(path)) else {
        return Ok(false);
    };
    let tray = SystemTrayManager::instance();
    let icons: Vec<_> = tray
        .icons()
        .into_iter()
        .filter(|icon| {
            icon.executable
                .as_ref()
                .is_some_and(|exe| same_executable(exe, &target))
                && icon.window_handle.is_some_and(|handle| {
                    let window = Window::from(handle);
                    window.is_window()
                        && window
                            .process()
                            .program_path()
                            .is_ok_and(|exe| same_executable(&exe, &target))
                })
                && icon.callback_message.is_some()
        })
        .collect();
    let mut running = false;
    let mut candidates = Vec::new();
    WindowEnumerator::new().for_each(|window| {
        if !window
            .process()
            .program_path()
            .is_ok_and(|exe| same_executable(&exe, &target))
        {
            return;
        }
        running = true;
        let class = window.class();
        let ex = WindowsApi::get_ex_styles(window.hwnd());
        let style = WindowsApi::get_styles(window.hwnd());
        // Do not expose message-only windows, tray bridges or hidden login/helper dialogs.
        let known_main = matches!(class.as_str(), "WeChatMainWndForPC" | "WeixinMainWndForPC");
        if !window.is_cloaked()
            && !ex.contains(WS_EX_TOOLWINDOW)
            && ((window.is_visible() && style.contains(WS_CAPTION)) || known_main)
        {
            candidates.push(window);
        }
    })?;
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    match restore_route(&name, !candidates.is_empty(), icons.len()) {
        RestoreRoute::Window => {
            let window = &candidates[0];
            window.show_window(if window.is_minimized() {
                SW_RESTORE
            } else {
                SW_SHOW
            })?;
            window.focus()?;
            return Ok(true);
        }
        RestoreRoute::MessagingTray => {
            let icon = &icons[0];
            tray.send_action(&icon.stable_id, &SystrayIconAction::LeftDoubleClick)?;
            tray.acknowledge_notification(&icon.stable_id);
            return Ok(true);
        }
        RestoreRoute::AmbiguousMessagingTray => {
            return Err(
                "Multiple background instances exist; select the account in the system tray".into(),
            );
        }
        // Use the original app entry point so its own single-instance handling
        // restores it. Synthesizing a generic tray left click often opens a menu.
        RestoreRoute::Launch => return Ok(false),
        RestoreRoute::CheckMessagingProcess => {}
    }
    // Message-only windows of background apps are excluded from EnumWindows.
    if !running {
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always),
        );
        running = system.processes().values().any(|process| {
            process
                .exe()
                .is_some_and(|exe| same_executable(exe, &target))
        });
    }
    if running {
        return Err(
            "Application is already running in the background; use its system tray icon to open it"
                .into(),
        );
    }
    Ok(false)
}
