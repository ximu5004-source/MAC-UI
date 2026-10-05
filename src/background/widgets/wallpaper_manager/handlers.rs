use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::SWP_ASYNCWINDOWPOS;

use crate::{error::Result, windows_api::WindowsApi};

use super::SeelenWall;

#[tauri::command(async)]
pub async fn set_as_wallpaper(webview: tauri::WebviewWindow) -> Result<()> {
    let handle = webview.hwnd()?.0 as isize;
    let rect = WindowsApi::virtual_screen_rect()?;

    let hwnd = HWND(handle as _);
    match SeelenWall::try_set_under_desktop_items(hwnd) {
        Ok(_) => {
            let relative_rect = RECT {
                left: 0,
                top: 0,
                right: rect.right - rect.left,
                bottom: rect.bottom - rect.top,
            };
            WindowsApi::move_window(hwnd, &relative_rect)?;
            WindowsApi::set_position(hwnd, None, &relative_rect, SWP_ASYNCWINDOWPOS)?;
        }
        Err(e) => {
            log::warn!(
                "Failed to attach to desktop hierarchy: {}, using absolute positioning",
                e
            );
            WindowsApi::move_window(hwnd, &rect)?;
            WindowsApi::set_position(hwnd, None, &rect, SWP_ASYNCWINDOWPOS)?;
        }
    }

    // refresh_desktop uses SPI_SETDESKWALLPAPER which on MSIX can cause the shell to
    // rebuild the WorkerW hierarchy, evicting our window from its parent. Log instead
    // of propagating so positioning already applied above is preserved.
    if let Err(e) = SeelenWall::refresh_desktop() {
        log::warn!("Failed to refresh desktop after wallpaper attach: {e}");
    }
    Ok(())
}
