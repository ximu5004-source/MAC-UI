//! Manual smoke test for the native tray bridge. No network or account access.
//! Appears as a clearly labeled test tray icon and exits automatically in 5 minutes.
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::*,
        },
    },
    core::w,
};

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_APP + 1 && lp.0 as u32 == WM_LBUTTONDBLCLK {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(hwnd);
        }
        if msg == WM_TIMER {
            if wp.0 == 1 {
                let _ = KillTimer(Some(hwnd), 1);
                let mut data = NOTIFYICONDATAW {
                    cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    uFlags: NIF_INFO,
                    ..Default::default()
                };
                let title: Vec<_> = "Seelen notification test".encode_utf16().collect();
                let body: Vec<_> = "Native Windows tray notification received successfully."
                    .encode_utf16()
                    .collect();
                data.szInfoTitle[..title.len()].copy_from_slice(&title);
                data.szInfo[..body.len()].copy_from_slice(&body);
                let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
            } else {
                let _ = DestroyWindow(hwnd);
            }
        }
        if msg == WM_DESTROY {
            PostQuitMessage(0);
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?.into();
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            lpszClassName: w!("SeelenNativeTrayProbe"),
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("SeelenNativeTrayProbe"),
            w!("Seelen notification test - native activation OK"),
            WS_OVERLAPPEDWINDOW,
            200,
            200,
            540,
            180,
            None,
            None,
            Some(instance),
            None,
        )?;
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE,
            uCallbackMessage: WM_APP + 1,
            hIcon: LoadIconW(None, IDI_INFORMATION)?,
            ..Default::default()
        };
        let tip: Vec<_> = "Telegram - Seelen TEST (no real messages)"
            .encode_utf16()
            .collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(NIM_ADD, &data).ok()?;
        SetTimer(Some(hwnd), 1, 12000, None);
        SetTimer(Some(hwnd), 2, 300000, None);
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
    Ok(())
}
