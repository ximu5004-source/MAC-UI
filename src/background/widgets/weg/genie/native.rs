use std::{
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    sync::Once,
    time::{Duration, Instant},
};

use seelen_core::rect::Rect;
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
        Graphics::{
            Dwm::DWMWA_TRANSITIONS_FORCEDISABLED,
            Gdi::{
                AlphaBlend, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
                CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
                GdiFlush, HBITMAP, HDC, HGDIOBJ, SelectObject,
            },
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, HTTRANSPARENT,
            HWND_TOPMOST, MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SW_MINIMIZE,
            SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, TranslateMessage, ULW_ALPHA,
            UpdateLayeredWindow, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_MOUSEACTIVATE,
            WM_NCHITTEST, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::w,
};

use super::geometry::{self, Bounds, Direction};
use crate::{
    error::Result,
    windows_api::{WindowsApi, window::Window},
};

const DURATION: Duration = Duration::from_millis(420);
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const BLEND: BLENDFUNCTION = BLENDFUNCTION {
    BlendOp: 0,
    BlendFlags: 0,
    SourceConstantAlpha: 255,
    AlphaFormat: 1,
};

struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    len: usize,
}

impl Dib {
    fn new(width: i32, height: i32) -> Result<Self> {
        if width <= 0 || height <= 0 || i64::from(width) * i64::from(height) > super::MAX_PIXELS {
            return Err("Invalid Genie surface dimensions".into());
        }
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err("Could not allocate Genie DC".into());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels: *mut c_void = null_mut();
            let bitmap =
                match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
                    Ok(bitmap) => bitmap,
                    Err(error) => {
                        let _ = DeleteDC(dc);
                        return Err(error.into());
                    }
                };
            let previous = SelectObject(dc, bitmap.into());
            if previous.is_invalid() || pixels.is_null() {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(dc);
                return Err("Could not select Genie surface".into());
            }
            Ok(Self {
                dc,
                bitmap,
                previous,
                pixels: pixels.cast(),
                len: width as usize * height as usize * 4,
            })
        }
    }

    fn clear(&mut self) {
        unsafe {
            let _ = GdiFlush();
            std::ptr::write_bytes(self.pixels, 0, self.len);
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            let _ = GdiFlush();
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

struct Overlay(HWND);

impl Overlay {
    fn new(bounds: &Rect) -> Result<Self> {
        static REGISTER: Once = Once::new();
        let instance = WindowsApi::module_handle_w()?.into();
        REGISTER.call_once(|| unsafe {
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("MAC UI Genie Transient"),
                lpfnWndProc: Some(window_proc),
                ..Default::default()
            });
        });
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("MAC UI Genie Transient"),
                w!("MAC UI minimize transition"),
                WS_POPUP,
                bounds.left,
                bounds.top,
                bounds.width(),
                bounds.height(),
                None,
                None,
                Some(instance),
                None,
            )?
        };
        Ok(Self(hwnd))
    }

    fn present(&self, surface: &Dib, bounds: &Rect) -> Result<()> {
        unsafe {
            UpdateLayeredWindow(
                self.0,
                None,
                Some(&POINT {
                    x: bounds.left,
                    y: bounds.top,
                }),
                Some(&SIZE {
                    cx: bounds.width(),
                    cy: bounds.height(),
                }),
                Some(surface.dc),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&BLEND),
                ULW_ALPHA,
            )?;
        }
        Ok(())
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            super::cancel();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Never alter a global Windows setting; preserve this window's exact old value.
struct TransitionGuard {
    window: Window,
    previous: windows_core::BOOL,
}

impl TransitionGuard {
    fn new(window: Window) -> Result<Self> {
        let mut previous = windows_core::BOOL(0);
        WindowsApi::dwm_get_window_attribute(
            window.hwnd(),
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &mut previous,
        )?;
        WindowsApi::set_system_transitions_disabled(window.hwnd(), true)?;
        Ok(Self { window, previous })
    }
}

impl Drop for TransitionGuard {
    fn drop(&mut self) {
        if self.window.is_window() {
            if let Err(error) = WindowsApi::dwm_set_window_attribute(
                self.window.hwnd(),
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &self.previous,
            ) {
                log::warn!(
                    "Could not restore this window's DWM transition flag after Genie: {error}"
                );
            }
        }
    }
}

fn bounds(rect: &Rect) -> Bounds {
    Bounds {
        x: rect.left as f64,
        y: rect.top as f64,
        width: rect.width() as f64,
        height: rect.height() as f64,
    }
}

fn draw(
    surface: &mut Dib,
    snapshot: &Dib,
    source: &Rect,
    target: &Rect,
    extent: &Rect,
    t: f64,
) -> Result<()> {
    surface.clear();
    let side = geometry::direction(bounds(source), bounds(target));
    let horizontal = matches!(side, Direction::Left | Direction::Right);
    let axis_length = if horizontal {
        source.width()
    } else {
        source.height()
    };
    let count = (axis_length as usize).min(256);
    for index in 0..count {
        let begin = index as i32 * axis_length / count as i32;
        let end = (index + 1) as i32 * axis_length / count as i32;
        let dest = geometry::strip(bounds(source), bounds(target), side, t, index, count);
        let x = dest.x.round() as i32 - extent.left;
        let y = dest.y.round() as i32 - extent.top;
        let width = ((dest.x + dest.width).round() as i32 - extent.left - x).max(1);
        let height = ((dest.y + dest.height).round() as i32 - extent.top - y).max(1);
        let (sx, sy, sw, sh) = if horizontal {
            (begin, 0, end - begin, source.height())
        } else {
            (0, begin, source.width(), end - begin)
        };
        unsafe {
            AlphaBlend(
                surface.dc,
                x,
                y,
                width,
                height,
                snapshot.dc,
                sx,
                sy,
                sw,
                sh,
                BLEND,
            )
            .ok()?;
        }
    }
    Ok(())
}

pub(super) fn animate(
    window: Window,
    source: Rect,
    target: Rect,
    rgba: Vec<u8>,
    generation: u64,
) -> Result<()> {
    let extent = Rect {
        left: source.left.min(target.left),
        top: source.top.min(target.top),
        right: source.right.max(target.right),
        bottom: source.bottom.max(target.bottom),
    };
    let snapshot = Dib::new(source.width(), source.height())?;
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(snapshot.pixels, snapshot.len);
        for (dest, src) in bytes.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
            // PrintWindow's capture is a composited window, not alpha artwork.
            dest.copy_from_slice(&[src[2], src[1], src[0], 255]);
        }
    }
    let mut surface = Dib::new(extent.width(), extent.height())?;
    let overlay = Overlay::new(&extent)?;
    draw(&mut surface, &snapshot, &source, &target, &extent, 0.0)?;
    overlay.present(&surface, &extent)?;
    if super::superseded(generation) {
        return Ok(());
    }
    if !super::active(generation) || !super::motion_enabled() {
        return Err("Genie display or motion preference changed".into());
    }
    let transition = TransitionGuard::new(window)?;
    unsafe {
        SetWindowPos(
            overlay.0,
            Some(HWND_TOPMOST),
            extent.left,
            extent.top,
            extent.width(),
            extent.height(),
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }
    window.show_window_async(SW_MINIMIZE)?;
    let submitted = Instant::now();
    while !window.is_minimized() {
        if !super::active(generation)
            || !window.is_window()
            || submitted.elapsed() > Duration::from_millis(80)
        {
            // The native minimize has already been submitted. Never block on a
            // hung application's response or leave a topmost overlay behind.
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    drop(transition);
    let started = Instant::now();
    while started.elapsed() < DURATION {
        let frame = Instant::now();
        if !super::active(generation) || !window.is_window() || !window.is_minimized() {
            break;
        }
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, Some(overlay.0), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        let t = started.elapsed().as_secs_f64() / DURATION.as_secs_f64();
        draw(&mut surface, &snapshot, &source, &target, &extent, t)?;
        overlay.present(&surface, &extent)?;
        // A wall-clock deadline skips frames under load, never stretches motion.
        std::thread::sleep(FRAME_INTERVAL.saturating_sub(frame.elapsed()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_owned_window_transition_attribute_probe() {
        // Never shown or activated. Probe only an owned temporary window, not a
        // user's app: some Windows builds expose this attribute as set-only.
        let overlay = Overlay::new(&Rect {
            left: 0,
            top: 0,
            right: 32,
            bottom: 32,
        })
        .unwrap();
        let window = Window::from(overlay.0);
        let mut before = windows_core::BOOL(0);
        if WindowsApi::dwm_get_window_attribute(
            window.hwnd(),
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &mut before,
        )
        .is_err()
        {
            assert!(TransitionGuard::new(window).is_err());
            println!(
                "DWM transition flag is not readable here; Genie safely falls back without mutating it."
            );
            return;
        }
        let guard = TransitionGuard::new(window).unwrap();
        drop(guard);
        let mut after = windows_core::BOOL(0);
        WindowsApi::dwm_get_window_attribute(
            window.hwnd(),
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &mut after,
        )
        .unwrap();
        assert_eq!(before, after);
        println!("DWM transition flag read/restore supported on an owned hidden window.");
    }

    #[test]
    fn offscreen_mesh_keeps_transparent_exterior_and_opaque_snapshot() {
        // GDI memory surfaces only: this test never creates/shows a window.
        let source = Rect {
            left: 20,
            top: 10,
            right: 180,
            bottom: 130,
        };
        let target = Rect {
            left: 120,
            top: 160,
            right: 140,
            bottom: 180,
        };
        let extent = Rect {
            left: 0,
            top: 0,
            right: 200,
            bottom: 200,
        };
        let snapshot = Dib::new(source.width(), source.height()).unwrap();
        unsafe {
            for pixel in
                std::slice::from_raw_parts_mut(snapshot.pixels, snapshot.len).chunks_exact_mut(4)
            {
                pixel.copy_from_slice(&[13, 117, 245, 255]);
            }
        }
        let mut surface = Dib::new(extent.width(), extent.height()).unwrap();
        for t in [0.0, 0.35, 0.75, 1.0] {
            draw(&mut surface, &snapshot, &source, &target, &extent, t).unwrap();
            unsafe {
                let _ = GdiFlush();
                let bytes = std::slice::from_raw_parts(surface.pixels, surface.len);
                assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
                let painted: Vec<&[u8]> = bytes.chunks_exact(4).filter(|p| p[3] > 0).collect();
                assert!(!painted.is_empty());
                assert!(painted.iter().all(|p| **p == [13, 117, 245, 255]));
            }
        }
    }
}
