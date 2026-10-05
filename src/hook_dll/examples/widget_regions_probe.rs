//! Read-only native region/composition diagnostic. Never sends input or messages.
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::{
            Dwm::{DWMWA_SYSTEMBACKDROP_TYPE, DwmGetWindowAttribute},
            Gdi::{CreateRectRgn, DeleteObject, GetRgnBox, GetWindowRgn, PtInRegion},
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetWindowLongW, GetWindowRect,
            GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        },
    },
    core::BOOL,
};

unsafe extern "system" fn inspect(hwnd: HWND, target: LPARAM) -> BOOL {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != target.0 as u32 {
            return BOOL(1);
        }
        let visible = IsWindowVisible(hwnd).as_bool();
        let mut title = [0; 256];
        let count = GetWindowTextW(hwnd, &mut title) as usize;
        let title = String::from_utf16_lossy(&title[..count]);
        let mut bounds = RECT::default();
        let _ = GetWindowRect(hwnd, &mut bounds);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let region = CreateRectRgn(0, 0, 0, 0);
        if region.is_invalid() {
            return BOOL(1);
        }
        let kind = GetWindowRgn(hwnd, region);
        let mut box_rect = RECT::default();
        let _ = GetRgnBox(region, &mut box_rect);
        let mut inside = 0;
        let mut total = 0;
        for x in (32..(bounds.right - bounds.left)).step_by(64) {
            for y in (32..(bounds.bottom - bounds.top)).step_by(64) {
                total += 1;
                inside += u32::from(PtInRegion(region, x, y).as_bool());
            }
        }
        let mut backdrop = 0u32;
        let result = DwmGetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &mut backdrop as *mut _ as _,
            4,
        );
        println!(
            "hwnd={} title={title:?} visible={visible} bounds={bounds:?} client={client:?} style={style:x} ex_style={ex_style:x} region_type={} box={box_rect:?} region_samples={inside}/{total} backdrop={backdrop} backdrop_read={}",
            hwnd.0 as isize,
            kind.0,
            result.is_ok()
        );
        let _ = DeleteObject(region.into());
        BOOL(1)
    }
}

fn main() -> windows::core::Result<()> {
    let pid = std::env::args()
        .nth(1)
        .and_then(|x| x.parse::<u32>().ok())
        .expect("Verified MAC UI PID required");
    unsafe {
        EnumWindows(Some(inspect), LPARAM(pid as isize))?;
    }
    Ok(())
}
