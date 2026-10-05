use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, IsWindowVisible, MSG,
        PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOWNORMAL, TranslateMessage, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_DESTROY, WM_ENDSESSION, WM_QUERYENDSESSION, WNDCLASSW,
    },
};

use crate::{
    error::Result,
    string_utils::WindowsString,
    windows_api::{
        WindowsApi,
        app_bar::{AppBarData, AppBarDataState},
        iterator::WindowEnumerator,
    },
};

/// Tracks whether we actually hid the native taskbar, so we only restore it
/// when it was hidden by us (avoids unnecessary restores on settings changes/shutdown).
static NATIVE_TASKBAR_HIDDEN: AtomicBool = AtomicBool::new(false);
static NATIVE_DESKTOP_ICONS_HIDDEN: AtomicBool = AtomicBool::new(false);
static HIDDEN_DESKTOP_ICON_WINDOWS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

pub fn get_taskbars_handles() -> Result<Vec<HWND>> {
    let mut founds = Vec::new();
    WindowEnumerator::new().for_each(|hwnd| {
        let class = WindowsApi::get_class(hwnd);
        if (class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd")
            && WindowsApi::get_title(hwnd).is_empty()
        {
            founds.push(hwnd);
        }
    })?;
    Ok(founds)
}

pub fn hide_native_taskbar() {
    NATIVE_TASKBAR_HIDDEN.store(true, Ordering::Release);
    std::thread::spawn(|| match get_taskbars_handles() {
        Ok(handles) => {
            let mut attempts = 0;
            while attempts < 10 && NATIVE_TASKBAR_HIDDEN.load(Ordering::Acquire) {
                for hwnd in &handles {
                    AppBarData::from_handle(*hwnd).set_state(AppBarDataState::AutoHide);
                    let _ = WindowsApi::show_window_async(hwnd.0 as isize, SW_HIDE.0);
                }
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        Err(err) => log::error!("Failed to get taskbars handles: {err:?}"),
    });
}

pub fn restore_native_taskbar() -> Result<()> {
    if NATIVE_TASKBAR_HIDDEN
        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    for hwnd in get_taskbars_handles()? {
        AppBarData::from_handle(hwnd).set_state(AppBarDataState::AlwaysOnTop);
        WindowsApi::show_window_async(hwnd.0 as isize, SW_SHOWNORMAL.0)?;
    }
    Ok(())
}

fn get_desktop_icon_handles() -> Result<Vec<HWND>> {
    let mut found = Vec::new();

    WindowEnumerator::new().for_each(|top_level| {
        if !matches!(
            WindowsApi::get_class(top_level).as_str(),
            "WorkerW" | "Progman"
        ) {
            return;
        }
        let _ = WindowEnumerator::new()
            .with_parent(top_level)
            .for_each(|child| {
                if WindowsApi::get_class(child) == "SysListView32" {
                    // The independent top-level desktop owns its own hit testing.
                    // Leave WorkerW / DefView and third-party wallpaper windows intact.
                    found.push(child);
                }
            });
    })?;

    Ok(found)
}

pub fn hide_native_desktop_icons() {
    let mut hidden = HIDDEN_DESKTOP_ICON_WINDOWS.lock();
    NATIVE_DESKTOP_ICONS_HIDDEN.store(true, Ordering::Release);
    match get_desktop_icon_handles() {
        Ok(handles) => {
            for hwnd in handles {
                // Restore only icons we actually hid; respect a user's already-hidden icons.
                if unsafe { IsWindowVisible(hwnd).as_bool() } {
                    let handle = hwnd.0 as isize;
                    if !hidden.contains(&handle) {
                        hidden.push(handle);
                    }
                    let _ = WindowsApi::show_window_async(handle, SW_HIDE.0);
                }
            }
        }
        Err(err) => log::error!("Failed to get desktop icon handles: {err:?}"),
    }
}

pub fn restore_native_desktop_icons() -> Result<()> {
    let mut hidden = HIDDEN_DESKTOP_ICON_WINDOWS.lock();
    if NATIVE_DESKTOP_ICONS_HIDDEN
        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    let live = get_desktop_icon_handles()?;
    for handle in hidden.drain(..) {
        if live.iter().any(|hwnd| hwnd.0 as isize == handle) {
            WindowsApi::show_window_async(handle, SW_SHOWNORMAL.0)?;
        }
    }
    Ok(())
}

unsafe extern "system" fn shutdown_window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            // sent by the OS when it wants to shut down/log off/quiesce the process
            // (system shutdown, user logoff, or a package update needing us to exit).
            // Reacting here lets us call `exit()` promptly instead of idling in the
            // exit channel wait until the OS times out and force-kills us.
            WM_QUERYENDSESSION | WM_ENDSESSION => {
                log::info!("Received shutdown/quiesce request (msg={msg}), exiting service");
                crate::exit(0);
            }
            WM_DESTROY => PostQuitMessage(0),
            _ => {}
        }
        DefWindowProcW(hwnd, msg, w_param, l_param)
    }
}

/// will lock until the window is closed
unsafe fn create_shutdown_window(done: &crossbeam_channel::Sender<()>) -> Result<()> {
    unsafe {
        let title = WindowsString::from_str("Seelen UI Service Shutdown Window");
        let class = WindowsString::from_str("SeelenServiceShutdownWindow");

        let h_module = GetModuleHandleW(None)?;

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(shutdown_window_proc),
            hInstance: h_module.into(),
            lpszClassName: class.as_pcwstr(),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class.as_pcwstr(),
            title.as_pcwstr(),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(wnd_class.hInstance),
            None,
        )?;

        done.send(())?;
        let mut msg = MSG::default();

        // GetMessageW will run until PostQuitMessage(0) is called
        while GetMessageW(&mut msg, Some(hwnd), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

/// Spawns a hidden top-level window on its own thread whose only purpose is receiving
/// `WM_QUERYENDSESSION`/`WM_ENDSESSION`, so the service can gracefully exit when the OS
/// tries to quiesce it (shutdown, logoff, package update) instead of being force-killed
/// while idling and reported as a hang.
pub fn start_shutdown_listener() -> Result<()> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("Shutdown Window".to_string())
        .spawn(move || {
            if let Err(err) = unsafe { create_shutdown_window(&tx) } {
                log::error!("Shutdown window thread failed: {err:?}");
            }
        })?;
    rx.recv()?;
    Ok(())
}
