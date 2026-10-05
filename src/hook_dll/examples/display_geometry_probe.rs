//! Read-only diagnostics: physical monitor and one application's HWND/client bounds.
//! No input injection, positioning, focus, power or display-setting mutations.
use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO},
    UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetClientRect, GetWindowLongW, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOPMOST,
    },
};
use windows::core::BOOL;

unsafe extern "system" fn monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, _: LPARAM) -> BOOL {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            println!("monitor rect={:?} work={:?}", info.rcMonitor, info.rcWork);
        }
    }
    BOOL(1)
}

unsafe extern "system" fn window(hwnd: HWND, target: LPARAM) -> BOOL {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != target.0 as u32 {
            return BOOL(1);
        }
        let mut title = [0; 256];
        let count = GetWindowTextW(hwnd, &mut title) as usize;
        let mut rect = RECT::default();
        let mut client = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() && GetClientRect(hwnd, &mut client).is_ok() {
            println!(
                "hwnd={} title={:?} visible={} topmost={} rect={rect:?} client={client:?}",
                hwnd.0 as isize,
                String::from_utf16_lossy(&title[..count]),
                IsWindowVisible(hwnd).as_bool(),
                GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
            );
        }
    }
    BOOL(1)
}

fn main() -> windows::core::Result<()> {
    let pid = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u32>().ok())
        .expect("Pass the verified MAC UI process ID");
    unsafe {
        EnumDisplayMonitors(None, None, Some(monitor), LPARAM(0)).ok()?;
        EnumWindows(Some(window), LPARAM(pid as isize))?;
    }
    Ok(())
}
