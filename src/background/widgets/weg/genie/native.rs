use std::{
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    sync::{LazyLock, Once},
    time::{Duration, Instant},
};

use macui_core::rect::Rect;
use rayon::prelude::*;
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
        Graphics::{
            Dwm::DwmFlush,
            Gdi::{
                AlphaBlend, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, COLORONCOLOR,
                CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
                GdiFlush, HALFTONE, HBITMAP, HDC, HGDIOBJ, SRCCOPY, SelectObject, SetBrushOrgEx,
                SetStretchBltMode, StretchBlt,
            },
        },
        Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowPlacement,
            HTTRANSPARENT, HWND_TOPMOST, MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW,
            RegisterClassW, SHOW_WINDOW_CMD, SW_HIDE, SW_RESTORE, SW_SHOWMINNOACTIVE, SW_SHOWNA,
            SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPlacement, SetWindowPos, ShowWindow,
            ShowWindowAsync, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WINDOWPLACEMENT,
            WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST,
            WNDCLASSW, WPF_ASYNCWINDOWPLACEMENT, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::w,
};

use super::geometry::{self, Bounds};
use crate::{
    error::Result,
    windows_api::{WindowsApi, window::Window},
};

const DURATION: Duration = Duration::from_millis(420);
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const PREFLIGHT_BUDGET: Duration = Duration::from_millis(20);
const FRAME_BUDGET: Duration = Duration::from_millis(25);
const TEXTURE_PIXELS: i64 = 1_048_576;
const FRAME_PIXELS: i64 = 786_432;
const MIN_FRAME_PIXELS: i64 = 196_608;
const BLEND: BLENDFUNCTION = BLENDFUNCTION {
    BlendOp: 0,
    BlendFlags: 0,
    SourceConstantAlpha: 255,
    AlphaFormat: 1,
};

#[cfg(test)]
thread_local! {
    // Explicit self-owned playback tests exercise real HWND/GDI transitions
    // without putting synthetic artwork over the user's desktop or focus.
    static OWNED_PLAYBACK: std::cell::Cell<(bool, usize)> = const { std::cell::Cell::new((false,0)) };
}

fn playback_allowed() -> bool {
    #[cfg(test)]
    if OWNED_PLAYBACK.with(|state| state.get().0) {
        return true;
    }
    super::motion_enabled() && super::enabled()
}

fn should_focus_restored_window() -> bool {
    #[cfg(test)]
    if OWNED_PLAYBACK.with(|state| state.get().0) {
        return false;
    }
    true
}

struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    len: usize,
    width: i32,
    height: i32,
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
                width,
                height,
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

/// Preserve physical coordinates while capping only the internal texture/work.
fn scaled_size(width: i32, height: i32, max_pixels: i64) -> (i32, i32) {
    let pixels = i64::from(width) * i64::from(height);
    let scale = (max_pixels as f64 / pixels as f64).sqrt().min(1.0);
    (
        ((width as f64 * scale).floor() as i32).max(1),
        ((height as f64 * scale).floor() as i32).max(1),
    )
}

pub(super) struct CapturedTexture {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

pub(super) fn capture(window: Window, source: &Rect, outer: &Rect) -> Result<CapturedTexture> {
    if !super::valid_rect(source)
        || !super::valid_rect(outer)
        || source.left < outer.left
        || source.top < outer.top
        || source.right > outer.right
        || source.bottom > outer.bottom
    {
        return Err("Invalid Genie capture crop".into());
    }
    let mut original = Dib::new(outer.width(), outer.height())?;
    let (width, height) = scaled_size(source.width(), source.height(), TEXTURE_PIXELS);
    let texture = Dib::new(width, height)?;
    let mut pixels = Vec::new();
    // RENDERFULLCONTENT supports redirected Chromium windows; ordinary WM_PRINT
    // is a compatibility path for GDI apps whose redirected frame is empty.
    // Both are normal PrintWindow modes; neither bypasses display affinity.
    for flags in [2, 0] {
        original.clear();
        let printed =
            unsafe { PrintWindow(window.hwnd(), original.dc, PRINT_WINDOW_FLAGS(flags)).as_bool() };
        if !printed {
            continue;
        }
        unsafe {
            SetStretchBltMode(texture.dc, HALFTONE);
            let _ = SetBrushOrgEx(texture.dc, 0, 0, None);
            StretchBlt(
                texture.dc,
                0,
                0,
                width,
                height,
                Some(original.dc),
                source.left - outer.left,
                source.top - outer.top,
                source.width(),
                source.height(),
                SRCCOPY,
            )
            .ok()?;
            let _ = GdiFlush();
        }
        pixels = unsafe { std::slice::from_raw_parts(texture.pixels, texture.len) }.to_vec();
        if pixels
            .chunks_exact(4)
            .any(|p| p[0] > 3 || p[1] > 3 || p[2] > 3)
        {
            break;
        }
    }
    if pixels.is_empty() {
        return Err("PrintWindow capture failed".into());
    }
    // PrintWindow supplies RGB, not artwork alpha. Force opaque after native
    // downsampling; never swizzle/copy every physical 4K pixel in scalar Rust.
    pixels.par_chunks_mut(64 * 1024).for_each(|tile| {
        for pixel in tile.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    });
    Ok(CapturedTexture {
        width,
        height,
        pixels,
    })
}
struct FrameBuffers {
    internal: Dib,
    physical: Dib,
    extent: Rect,
    budget: i64,
}

// Probe actual alpha behavior rather than assuming every GDI implementation
// preserves the fourth byte of 32-bit SRCCOPY. DIBs only; no windows or settings.
static FAST_ALPHA_STRETCH: LazyLock<bool> = LazyLock::new(|| {
    let Ok(source) = Dib::new(2, 2) else {
        return false;
    };
    let Ok(target) = Dib::new(4, 4) else {
        return false;
    };
    let colors = [
        [0, 0, 0, 0],
        [13, 117, 245, 255],
        [13, 117, 245, 255],
        [0, 0, 0, 0],
    ];
    unsafe {
        let input = std::slice::from_raw_parts_mut(source.pixels, source.len);
        for (dest, color) in input.chunks_exact_mut(4).zip(colors) {
            dest.copy_from_slice(&color);
        }
        SetStretchBltMode(target.dc, COLORONCOLOR);
        if !StretchBlt(target.dc, 0, 0, 4, 4, Some(source.dc), 0, 0, 2, 2, SRCCOPY).as_bool() {
            return false;
        }
        let _ = GdiFlush();
        let output = std::slice::from_raw_parts(target.pixels, target.len);
        output
            .chunks_exact(4)
            .enumerate()
            .all(|(index, pixel)| pixel == colors[(index / 4 / 2) * 2 + (index % 4 / 2)])
    }
});

impl FrameBuffers {
    fn new(extent: &Rect) -> Result<Self> {
        let (width, height) = scaled_size(extent.width(), extent.height(), FRAME_PIXELS);
        Ok(Self {
            internal: Dib::new(width, height)?,
            physical: Dib::new(extent.width(), extent.height())?,
            extent: extent.clone(),
            budget: FRAME_PIXELS,
        })
    }

    fn reduce_quality(&mut self) -> Result<bool> {
        if self.budget <= MIN_FRAME_PIXELS {
            return Ok(false);
        }
        self.budget = (self.budget / 2).max(MIN_FRAME_PIXELS);
        let (width, height) = scaled_size(self.extent.width(), self.extent.height(), self.budget);
        self.internal = Dib::new(width, height)?;
        Ok(true)
    }

    fn draw(&mut self, texture: &Dib, source: &Rect, target: &Rect, t: f64) -> Result<()> {
        draw(&mut self.internal, texture, source, target, &self.extent, t)?;
        unsafe {
            if *FAST_ALPHA_STRETCH {
                SetStretchBltMode(self.physical.dc, COLORONCOLOR);
                StretchBlt(
                    self.physical.dc,
                    0,
                    0,
                    self.physical.width,
                    self.physical.height,
                    Some(self.internal.dc),
                    0,
                    0,
                    self.internal.width,
                    self.internal.height,
                    SRCCOPY,
                )
                .ok()?;
            } else {
                // Documented alpha composition path when the fast copy probe
                // fails. A 4K surface is still animated, at a lower frame rate.
                self.physical.clear();
                AlphaBlend(
                    self.physical.dc,
                    0,
                    0,
                    self.physical.width,
                    self.physical.height,
                    self.internal.dc,
                    0,
                    0,
                    self.internal.width,
                    self.internal.height,
                    BLEND,
                )
                .ok()?;
            }
            let _ = GdiFlush();
        }
        Ok(())
    }
}

/// The window owns a tiny transparent replacement surface for every exit path.
/// Keeping that allocation out of Drop makes cleanup independent of a failed
/// frame allocation or an animation canceled halfway through the screenshot.
struct Overlay(HWND, Dib);

/// At the icon, the screenshot must disappear rather than finish as an opaque
/// thumbnail on top of (or sampled by) the translucent Dock. Restore follows the
/// same phase in reverse, so its screenshot emerges from a transparent endpoint.
fn screenshot_opacity(phase: f64) -> u8 {
    let fade = ((phase - 0.75) / 0.25).clamp(0.0, 1.0);
    (255.0 * (1.0 + (std::f64::consts::PI * fade).cos()) * 0.5).round() as u8
}

impl Overlay {
    fn new(bounds: &Rect) -> Result<Self> {
        let mut transparent = Dib::new(1, 1)?;
        transparent.clear();
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
        Ok(Self(hwnd, transparent))
    }

    fn present(&self, surface: &Dib, bounds: &Rect, opacity: u8) -> Result<()> {
        let blend = BLENDFUNCTION {
            SourceConstantAlpha: opacity,
            ..BLEND
        };
        #[cfg(test)]
        let blend = if OWNED_PLAYBACK.with(|state| {
            let (enabled, count) = state.get();
            if enabled {
                state.set((true, count + 1));
            }
            enabled
        }) {
            BLENDFUNCTION {
                SourceConstantAlpha: 0,
                ..BLEND
            }
        } else {
            blend
        };
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
                Some(&blend),
                ULW_ALPHA,
            )?;
        }
        Ok(())
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            // Submit transparent content before detaching the HWND so an opaque
            // screenshot is never its final queued composition near the Dock.
            // A 1x1 transparent surface also releases the full-size retained
            // bitmap without allocating or walking 4K pixels during cleanup.
            let _ = UpdateLayeredWindow(
                self.0,
                None,
                None,
                Some(&SIZE { cx: 1, cy: 1 }),
                Some(self.1.dc),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&BLEND),
                ULW_ALPHA,
            );
            let _ = ShowWindow(self.0, SW_HIDE);
            if let Err(error) = DestroyWindow(self.0) {
                log::warn!("Could not destroy Genie transient window: {error}");
            }
            // Only flush our queued cleanup, never repaint other applications
            // or rebuild the Dock's live backdrop effect tree. A display-off or
            // locked session may have no upcoming Present to wait for.
            if super::IS_DISPLAY_ON.load(std::sync::atomic::Ordering::Acquire)
                && super::IS_INTERACTIVE_SESSION.load(std::sync::atomic::Ordering::Acquire)
            {
                let _ = DwmFlush();
            }
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

/// Readable placement, unlike the setter-only DWM transitions attribute.
pub(super) fn placement(window: Window) -> Result<WINDOWPLACEMENT> {
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe {
        GetWindowPlacement(window.hwnd(), &mut placement)?;
    }
    Ok(placement)
}

pub(super) fn same_monitor(window: Window, source: &Rect, target: &Rect) -> bool {
    let Ok(monitor) = window.monitor().info() else {
        return false;
    };
    let screen = monitor.monitorInfo.rcMonitor;
    [source, target].iter().all(|rect| {
        super::valid_rect(rect)
            && rect.left >= screen.left
            && rect.top >= screen.top
            && rect.right <= screen.right
            && rect.bottom <= screen.bottom
    })
}

/// Exists only during a short hide/commit, never for the 420ms animation.
/// Drop queues a non-activating reveal if an error leaves the app hidden.
/// The caller holds ACTION_LOCK so a newer action is queued after this repair.
struct HiddenWindowGuard {
    window: Window,
    pid: u32,
    recovery: SHOW_WINDOW_CMD,
    generation: u64,
    created_at: Instant,
}

impl HiddenWindowGuard {
    fn new(window: Window, recovery: SHOW_WINDOW_CMD, generation: u64) -> Result<Self> {
        let guard = Self {
            window,
            pid: window.process().id(),
            recovery,
            generation,
            created_at: Instant::now(),
        };
        *super::TRANSIENT_VISIBILITY.lock() = Some((window.address(), guard.pid, generation));
        unsafe {
            ShowWindowAsync(window.hwnd(), SW_HIDE).ok()?;
        }
        wait_for(window, guard.pid, || !window.is_visible())?;
        Ok(guard)
    }
}

impl Drop for HiddenWindowGuard {
    fn drop(&mut self) {
        if self.window.is_window()
            && self.window.process().id() == self.pid
            && !self.window.is_visible()
        {
            unsafe {
                // A new native minimize can arrive during a short state commit.
                // Reveal it iconic instead of undoing that input.
                let minimize =
                    super::system_events::minimized_intent_after(self.window, self.created_at)
                        .unwrap_or_else(|| self.window.is_minimized());
                let recovery = if self.recovery == SW_SHOWNA && minimize {
                    SW_SHOWMINNOACTIVE
                } else {
                    self.recovery
                };
                let _ = ShowWindowAsync(self.window.hwnd(), recovery);
            }
            let _ = wait_for(self.window, self.pid, || self.window.is_visible());
        }
        let mut lease = super::TRANSIENT_VISIBILITY.lock();
        if *lease == Some((self.window.address(), self.pid, self.generation)) {
            *lease = None;
        }
    }
}

fn wait_for(window: Window, pid: u32, state: impl Fn() -> bool) -> Result<()> {
    let started = Instant::now();
    loop {
        if !window.is_window() || window.process().id() != pid {
            return Err("Genie source window closed".into());
        }
        if state() {
            return Ok(());
        }
        if started.elapsed() >= Duration::from_millis(80) {
            return Err("Genie window state deadline elapsed".into());
        }
        pump();
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn pump() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn commit_minimize(window: Window, snapshot: &super::Snapshot, generation: u64) -> Result<()> {
    let _action = super::ACTION_LOCK.lock();
    if !super::active(generation) {
        return Err("Genie action superseded".into());
    }
    if !window.is_window()
        || window.process().id() != snapshot.pid
        || window.class() != snapshot.class
        || window.is_minimized()
        || !window.is_visible()
        || window.inner_rect().ok().as_ref() != Some(&snapshot.source)
    {
        return Err("Genie source changed before minimize".into());
    }
    let guard = HiddenWindowGuard::new(window, SW_SHOWNA, generation)?;
    // Minimizing an already hidden window avoids mutating any unreadable DWM
    // setting. It remains a real, visible/iconic, Alt+Tab-reachable application.
    unsafe {
        super::system_events::expect_owned(window, false);
        ShowWindowAsync(window.hwnd(), SW_SHOWMINNOACTIVE).ok()?;
    }
    wait_for(window, guard.pid, || {
        window.is_visible() && window.is_minimized()
    })?;
    Ok(())
}

fn commit_restore(window: Window, snapshot: &super::Snapshot, generation: u64) -> Result<bool> {
    let _action = super::ACTION_LOCK.lock();
    if !super::active(generation) {
        return Err("Genie restore superseded".into());
    }
    if !window.is_window()
        || window.process().id() != snapshot.pid
        || window.class() != snapshot.class
    {
        return Err("Genie restore source changed".into());
    }
    if !window.is_minimized() {
        return Ok(false);
    }
    let guard = HiddenWindowGuard::new(window, SW_RESTORE, generation)?;
    let mut original = snapshot.placement;
    // Preserve maximized state AND normal placement; never use SW_SHOWNORMAL
    // for a window which was maximized when it entered the Dock.
    original.flags |= WPF_ASYNCWINDOWPLACEMENT;
    unsafe {
        super::system_events::expect_owned(window, true);
        SetWindowPlacement(window.hwnd(), &original)?;
    }
    wait_for(window, guard.pid, || {
        window.is_visible() && !window.is_minimized()
    })?;
    Ok(true)
}

/// Observation never hides another application's HWND across playback. A
/// terminated animator leaves a visible/iconic window reachable in Alt+Tab.
fn park_observed_restore(
    window: Window,
    snapshot: &super::Snapshot,
    generation: u64,
) -> Result<()> {
    let _action = super::ACTION_LOCK.lock();
    if !super::active(generation)
        || !window.is_window()
        || window.process().id() != snapshot.pid
        || window.class() != snapshot.class
        || !window.is_visible()
        || window.is_minimized()
    {
        return Err("Observed restore changed before playback".into());
    }
    super::system_events::expect_owned(window, false);
    unsafe {
        ShowWindowAsync(window.hwnd(), SW_SHOWMINNOACTIVE).ok()?;
    }
    wait_for(window, snapshot.pid, || {
        window.is_visible() && window.is_minimized()
    })
}

fn restore_observed_window(
    window: Window,
    snapshot: &super::Snapshot,
    generation: u64,
) -> Result<bool> {
    let _action = super::ACTION_LOCK.lock();
    if !super::active(generation)
        || !window.is_window()
        || window.process().id() != snapshot.pid
        || window.class() != snapshot.class
    {
        return Err("Observed restore superseded or source closed".into());
    }
    if !window.is_minimized() {
        return Ok(false);
    }
    let mut original = snapshot.placement;
    original.flags |= WPF_ASYNCWINDOWPLACEMENT;
    super::system_events::expect_owned(window, true);
    unsafe {
        SetWindowPlacement(window.hwnd(), &original)?;
    }
    wait_for(window, snapshot.pid, || {
        window.is_visible() && !window.is_minimized()
    })?;
    Ok(true)
}

fn bounds(rect: &Rect) -> Bounds {
    Bounds {
        x: rect.left as f64,
        y: rect.top as f64,
        width: rect.width() as f64,
        height: rect.height() as f64,
    }
}

/// Scan-convert the 1,250 textured triangles. Adjacent cells share vertices;
/// pixel centers and half-open edges avoid cracks and overlapping alpha seams.
/// Each span increments UV instead of recalculating barycentrics per pixel.
fn triangle(
    surface: &mut [u8],
    width: usize,
    height: usize,
    top: usize,
    texture: &[u8],
    texture_width: usize,
    texture_height: usize,
    vertices: [geometry::Vertex; 3],
) {
    let min_y = vertices.iter().map(|v| v.y).fold(f64::INFINITY, f64::min);
    let max_y = vertices
        .iter()
        .map(|v| v.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let first_y = ((min_y - 0.5).ceil() as i32).max(top as i32) as usize;
    let last_y = ((max_y - 0.5).ceil() as i32).clamp(top as i32, (top + height) as i32) as usize;
    for y in first_y..last_y {
        let scan = y as f64 + 0.5;
        let mut intersections = [geometry::Vertex::default(); 3];
        let mut count = 0;
        for edge in 0..3 {
            let a = vertices[edge];
            let b = vertices[(edge + 1) % 3];
            if scan < a.y.min(b.y) || scan >= a.y.max(b.y) {
                continue;
            }
            let f = (scan - a.y) / (b.y - a.y);
            intersections[count] = geometry::Vertex {
                x: a.x + (b.x - a.x) * f,
                y: scan,
                u: a.u + (b.u - a.u) * f,
                v: a.v + (b.v - a.v) * f,
            };
            count += 1;
        }
        if count != 2 {
            continue;
        }
        let (left, right) = if intersections[0].x < intersections[1].x {
            (intersections[0], intersections[1])
        } else {
            (intersections[1], intersections[0])
        };
        if right.x - left.x <= 1e-10 {
            continue;
        }
        let first_x = ((left.x - 0.5).ceil() as i32).clamp(0, width as i32) as usize;
        let last_x = ((right.x - 0.5).ceil() as i32).clamp(0, width as i32) as usize;
        let du = (right.u - left.u) / (right.x - left.x);
        let dv = (right.v - left.v) / (right.x - left.x);
        let offset = first_x as f64 + 0.5 - left.x;
        let mut u = left.u + offset * du;
        let mut v = left.v + offset * dv;
        for x in first_x..last_x {
            let sx = ((u * texture_width as f64) as usize).min(texture_width - 1);
            let sy = ((v * texture_height as f64) as usize).min(texture_height - 1);
            let src = (sy * texture_width + sx) * 4;
            let dest = ((y - top) * width + x) * 4;
            surface[dest..dest + 4].copy_from_slice(&texture[src..src + 4]);
            u += du;
            v += dv;
        }
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
    let source_bounds = bounds(source);
    let target_bounds = bounds(target);
    let mut vertices = geometry::mesh(
        source_bounds,
        target_bounds,
        geometry::direction(source_bounds, target_bounds),
        t,
    );
    for vertex in &mut vertices {
        vertex.x = (vertex.x - extent.left as f64) * surface.width as f64 / extent.width() as f64;
        vertex.y = (vertex.y - extent.top as f64) * surface.height as f64 / extent.height() as f64;
    }
    surface.clear();
    let pixels = unsafe { std::slice::from_raw_parts_mut(surface.pixels, surface.len) };
    let texture = unsafe { std::slice::from_raw_parts(snapshot.pixels, snapshot.len) };
    // Exact endpoint: copy scanlines without sampling two million pixels in
    // scalar Rust. This also prevents a blurry handover to the real window.
    if t <= 0.0
        && snapshot.width == source.width()
        && snapshot.height == source.height()
        && surface.width == extent.width()
        && surface.height == extent.height()
    {
        let width = source.width() as usize * 4;
        let x = (source.left - extent.left) as usize * 4;
        let y = (source.top - extent.top) as usize;
        for (row, bytes) in texture.chunks_exact(width).enumerate() {
            let start = (y + row) * extent.width() as usize * 4 + x;
            pixels[start..start + width].copy_from_slice(bytes);
        }
        return Ok(());
    }
    if t <= 0.0 {
        let sx = surface.width as f64 / extent.width() as f64;
        let sy = surface.height as f64 / extent.height() as f64;
        let left = ((source.left - extent.left) as f64 * sx).round() as i32;
        let top = ((source.top - extent.top) as f64 * sy).round() as i32;
        let right = ((source.right - extent.left) as f64 * sx).round() as i32;
        let bottom = ((source.bottom - extent.top) as f64 * sy).round() as i32;
        unsafe {
            AlphaBlend(
                surface.dc,
                left,
                top,
                right - left,
                bottom - top,
                snapshot.dc,
                0,
                0,
                snapshot.width,
                snapshot.height,
                BLEND,
            )
            .ok()?;
            let _ = GdiFlush();
        }
        return Ok(());
    }
    let n = geometry::MESH_SIDE;
    // Independent 64-row tiles avoid races at shared triangle edges. Reuse the
    // application's existing Rayon pool instead of spawning threads per frame.
    let width = surface.width as usize;
    let texture_width = snapshot.width as usize;
    let texture_height = snapshot.height as usize;
    pixels
        .par_chunks_mut(width * 4 * 64)
        .enumerate()
        .for_each(|(tile_index, tile)| {
            let height = tile.len() / (width * 4);
            for row in 0..n - 1 {
                for column in 0..n - 1 {
                    let a = row * n + column;
                    for indices in [[a, a + 1, a + n + 1], [a, a + n + 1, a + n]] {
                        triangle(
                            tile,
                            width,
                            height,
                            tile_index * 64,
                            texture,
                            texture_width,
                            texture_height,
                            indices.map(|index| vertices[index]),
                        );
                    }
                }
            }
        });
    Ok(())
}

pub(super) fn animate(
    window: Window,
    snapshot: &super::Snapshot,
    target: Rect,
    generation: u64,
    restore: bool,
) -> Result<()> {
    animate_impl(window, snapshot, target, generation, restore, false)
}

/// The OS command is complete already. Restore is temporarily parked iconic,
/// never hidden for the effect duration. Owned event expectations prevent the
/// parking/reveal commands from recursively starting another transition.
pub(super) fn animate_observed(
    window: Window,
    snapshot: &super::Snapshot,
    target: Rect,
    generation: u64,
    restore: bool,
) -> Result<()> {
    let result = animate_impl(window, snapshot, target, generation, restore, true);
    if restore && result.is_err() && !super::superseded(generation) {
        // A render failure must not consume the user's restore. This recovery
        // does not hide or wait on a foreign UI thread without a deadline.
        let _ = restore_observed_window(window, snapshot, generation);
    }
    result
}

fn animate_impl(
    window: Window,
    snapshot: &super::Snapshot,
    target: Rect,
    generation: u64,
    restore: bool,
    observed: bool,
) -> Result<()> {
    let initial_foreground = Window::get_foregrounded().address();
    let source = &snapshot.source;
    let extent = Rect {
        left: source.left.min(target.left),
        top: source.top.min(target.top),
        right: source.right.max(target.right),
        bottom: source.bottom.max(target.bottom),
    };
    if !super::valid_rect(&extent) {
        return Err("Genie overlay extent is excessive".into());
    }
    if snapshot.texture_width <= 0
        || snapshot.texture_height <= 0
        || snapshot.pixels.len()
            != snapshot.texture_width as usize * snapshot.texture_height as usize * 4
    {
        return Err("Invalid Genie capture texture".into());
    }
    let texture = Dib::new(snapshot.texture_width, snapshot.texture_height)?;
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(texture.pixels, texture.len);
        bytes.copy_from_slice(&snapshot.pixels);
    }
    let mut frames = FrameBuffers::new(&extent)?;
    // Probe the expensive wide-neck phase before showing or hiding anything.
    // Do not force CPU animation on a busy or low-power machine.
    loop {
        let probe = Instant::now();
        draw(
            &mut frames.internal,
            &texture,
            source,
            &target,
            &extent,
            0.15,
        )?;
        if probe.elapsed() <= PREFLIGHT_BUDGET || !frames.reduce_quality()? {
            break;
        }
        if super::superseded(generation) {
            return Ok(());
        }
    }
    let overlay = Overlay::new(&extent)?;
    let initial_phase = if restore { 1.0 } else { 0.0 };
    frames.draw(&texture, source, &target, initial_phase)?;
    overlay.present(&frames.physical, &extent, screenshot_opacity(initial_phase))?;
    if super::superseded(generation) {
        return Ok(());
    }
    if !super::active(generation) || !playback_allowed() {
        return Err("Genie motion unavailable".into());
    }
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
    if observed && restore {
        park_observed_restore(window, snapshot, generation)?;
        if initial_foreground == window.address() {
            // Minimize/activation may settle separately on a foreign UI
            // thread. Do not mistake that expected handoff for newer input.
            let _ = wait_for(window, snapshot.pid, || {
                Window::get_foregrounded().address() != window.address()
            });
        }
    }
    let playback_foreground = Window::get_foregrounded().address();
    if !restore && !observed {
        commit_minimize(window, snapshot, generation)?;
    }
    let started = Instant::now();
    while started.elapsed() < DURATION {
        let frame = Instant::now();
        if super::superseded(generation) {
            return Ok(());
        }
        if !super::active(generation) {
            return Err("Genie display/session changed".into());
        }
        if !window.is_window() || window.process().id() != snapshot.pid || !window.is_minimized() {
            return Ok(());
        }
        if restore && Window::get_foregrounded().address() != playback_foreground {
            // An intervening Alt+Tab/app activation is newer focus intent.
            return Ok(());
        }
        if !playback_allowed() {
            break;
        }
        pump();
        let elapsed = (started.elapsed().as_secs_f64() / DURATION.as_secs_f64()).min(1.0);
        let phase = if restore { 1.0 - elapsed } else { elapsed };
        frames.draw(&texture, source, &target, phase)?;
        overlay.present(&frames.physical, &extent, screenshot_opacity(phase))?;
        if frame.elapsed() > FRAME_BUDGET {
            // Keep 4K animation active: lower internal detail under contention
            // and skip late frames, instead of abandoning the user's effect.
            let _ = frames.reduce_quality()?;
        }
        std::thread::sleep(FRAME_INTERVAL.saturating_sub(frame.elapsed()));
    }
    if restore && !super::active(generation) {
        return if super::superseded(generation) {
            Ok(())
        } else {
            Err("Genie restore display/session changed".into())
        };
    }
    if restore && Window::get_foregrounded().address() != playback_foreground {
        return Ok(());
    }
    if restore {
        // Cover the native window until it is completely revealed. Any failure
        // bubbles to the handler's native restore; Drop removes the overlay.
        frames.draw(&texture, source, &target, 0.0)?;
        overlay.present(&frames.physical, &extent, screenshot_opacity(0.0))?;
        let committed = if observed {
            restore_observed_window(window, snapshot, generation)?
        } else {
            commit_restore(window, snapshot, generation)?
        };
        if committed {
            let _action = super::ACTION_LOCK.lock();
            if !super::superseded(generation)
                && should_focus_restored_window()
                && (!observed || initial_foreground == window.address())
            {
                window.focus()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_disappears_at_dock_and_returns_with_reverse_transition() {
        // The window remains fully visible during the funnel's main movement,
        // then vanishes into the icon instead of ending as a colored thumbnail.
        for phase in [0.0, 0.15, 0.42, 0.75] {
            assert_eq!(screenshot_opacity(phase), 255);
        }
        let phases = [0.75, 0.80, 0.85, 0.90, 0.95, 1.0];
        let minimize = phases.map(screenshot_opacity);
        assert!(minimize.windows(2).all(|pair| pair[0] > pair[1]));
        assert_eq!(minimize[5], 0);
        let restore = phases.map(|phase| screenshot_opacity(1.75 - phase));
        assert_eq!(restore[0], 0);
        assert_eq!(restore[5], 255);
        assert!(restore.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    #[ignore = "explicit invisible self-owned transient HWND lifecycle probe"]
    fn hidden_overlay_is_destroyed_after_success_and_early_return() {
        use windows::Win32::UI::WindowsAndMessaging::{IsWindow, IsWindowVisible};
        fn create_and_leave(early: bool) -> HWND {
            let overlay = Overlay::new(&Rect {
                left: -10000,
                top: -10000,
                right: -9872,
                bottom: -9872,
            })
            .unwrap();
            let handle = overlay.0;
            assert!(unsafe { IsWindow(Some(handle)) }.as_bool());
            assert!(!unsafe { IsWindowVisible(handle) }.as_bool());
            if early {
                return handle;
            }
            drop(overlay);
            handle
        }
        for early in [false, true] {
            let handle = create_and_leave(early);
            assert!(!unsafe { IsWindow(Some(handle)) }.as_bool());
        }
    }

    #[test]
    fn four_k_surface_preserves_transparency_opaque_pixels_and_physical_target() {
        let source = Rect {
            left: -3840,
            top: 0,
            right: 0,
            bottom: 2160,
        };
        let target = Rect {
            left: -1990,
            top: 2050,
            right: -1850,
            bottom: 2120,
        };
        let (width, height) = scaled_size(source.width(), source.height(), TEXTURE_PIXELS);
        let texture = Dib::new(width, height).unwrap();
        unsafe {
            for pixel in
                std::slice::from_raw_parts_mut(texture.pixels, texture.len).chunks_exact_mut(4)
            {
                pixel.copy_from_slice(&[13, 117, 245, 255]);
            }
        }
        let mut frames = FrameBuffers::new(&source).unwrap();
        assert_eq!(
            (frames.physical.width, frames.physical.height),
            (3840, 2160)
        );
        assert!(
            i64::from(frames.internal.width) * i64::from(frames.internal.height) <= FRAME_PIXELS
        );
        for t in [0.0, 0.15, 0.35, 0.65, 0.85, 1.0] {
            let started = Instant::now();
            frames.draw(&texture, &source, &target, t).unwrap();
            let elapsed = started.elapsed();
            let bytes =
                unsafe { std::slice::from_raw_parts(frames.physical.pixels, frames.physical.len) };
            let mut opaque = 0usize;
            for pixel in bytes.chunks_exact(4) {
                assert!(pixel == [0, 0, 0, 0] || pixel == [13, 117, 245, 255]);
                if pixel[3] != 0 {
                    opaque += 1;
                }
            }
            assert!(opaque > 0);
            if t == 0.0 {
                assert_eq!(opaque, 3840 * 2160);
            }
            if t == 1.0 {
                let x = ((target.left + target.right) / 2 - source.left) as usize;
                let y = ((target.top + target.bottom) / 2 - source.top) as usize;
                assert_eq!(
                    &bytes[(y * 3840 + x) * 4..(y * 3840 + x) * 4 + 4],
                    &[13, 117, 245, 255]
                );
                assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
            }
            println!(
                "4K physical 3840x2160 internal={}x{} texture={}x{} phase={t} draw+upscale={}us opaque={opaque}",
                frames.internal.width,
                frames.internal.height,
                width,
                height,
                elapsed.as_micros()
            );
        }
        while frames.reduce_quality().unwrap() {
            frames.draw(&texture, &source, &target, 0.35).unwrap();
            assert_eq!(
                (frames.physical.width, frames.physical.height),
                (3840, 2160)
            );
            let center = (1080usize * 3840 + 1920) * 4;
            let bytes =
                unsafe { std::slice::from_raw_parts(frames.physical.pixels, frames.physical.len) };
            assert_eq!(&bytes[center..center + 4], &[13, 117, 245, 255]);
        }
        assert_eq!(frames.budget, MIN_FRAME_PIXELS);
    }

    #[test]
    #[ignore = "explicit 4K self-owned transparent window PrintWindow/state probe"]
    fn owned_four_k_capture_minimize_restore_probe() {
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GWL_STYLE, GetWindowLongW, LWA_ALPHA, SWP_FRAMECHANGED, SWP_NOZORDER,
            SetLayeredWindowAttributes, SetWindowLongW,
        };
        let owned = super::super::hide_probe::OwnedWindow::new();
        let window = Window::from(owned.0);
        let offscreen = window.outer_rect().unwrap();
        unsafe {
            SetWindowLongW(
                owned.0,
                GWL_EXSTYLE,
                GetWindowLongW(owned.0, GWL_EXSTYLE) | WS_EX_TRANSPARENT.0 as i32,
            );
            SetWindowLongW(
                owned.0,
                GWL_STYLE,
                (WS_POPUP | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE).0 as i32,
            );
            SetWindowPos(
                owned.0,
                None,
                offscreen.left,
                offscreen.top,
                3840,
                2160,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            )
            .unwrap();
        }
        pump();
        let source = window.inner_rect().unwrap();
        let outer = window.outer_rect().unwrap();
        assert_eq!((source.width(), source.height()), (3840, 2160));
        let started = Instant::now();
        let image = capture(window, &source, &outer).unwrap();
        println!(
            "owned 4K PrintWindow+downsample={}us texture={}x{} cache_bytes={}",
            started.elapsed().as_micros(),
            image.width,
            image.height,
            image.pixels.len()
        );
        assert!(image.pixels.len() <= TEXTURE_PIXELS as usize * 4);
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [245, 117, 13, 255])
        );
        // Capture a normal opaque HWND off-screen, then make the fixture fully
        // transparent before moving it on-screen for placement/playback tests.
        // Capturing an alpha=0 layered HWND legitimately yields black pixels.
        unsafe {
            SetWindowLongW(
                owned.0,
                GWL_EXSTYLE,
                GetWindowLongW(owned.0, GWL_EXSTYLE) | WS_EX_LAYERED.0 as i32,
            );
            SetLayeredWindowAttributes(owned.0, COLORREF(0), 0, LWA_ALPHA).unwrap();
            SetWindowPos(
                owned.0,
                None,
                0,
                0,
                3840,
                2160,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW,
            )
            .unwrap();
        }
        pump();
        let source = window.inner_rect().unwrap();
        let original = placement(window).unwrap();
        let snapshot = super::super::Snapshot {
            hwnd: window.address(),
            pid: window.process().id(),
            class: window.class(),
            source,
            placement: original,
            texture_width: image.width,
            texture_height: image.height,
            pixels: image.pixels.into(),
            captured_at: Instant::now(),
        };
        let generation = super::super::cancel();
        commit_minimize(window, &snapshot, generation).unwrap();
        assert!(window.is_visible() && window.is_minimized());
        assert!(commit_restore(window, &snapshot, generation).unwrap());
        assert!(window.is_visible() && !window.is_minimized());
        assert_eq!(
            placement(window).unwrap().rcNormalPosition,
            original.rcNormalPosition
        );
        struct OwnedPlayback;
        impl Drop for OwnedPlayback {
            fn drop(&mut self) {
                OWNED_PLAYBACK.with(|state| state.set((false, 0)));
            }
        }
        OWNED_PLAYBACK.with(|state| state.set((true, 0)));
        let _playback = OwnedPlayback;
        let target = Rect {
            left: 1850,
            top: 2050,
            right: 1990,
            bottom: 2120,
        };
        let started = Instant::now();
        let generation = super::super::cancel();
        animate(window, &snapshot, target.clone(), generation, false).unwrap();
        assert!(window.is_visible() && window.is_minimized());
        let generation = super::super::cancel();
        animate(window, &snapshot, target, generation, true).unwrap();
        assert!(window.is_visible() && !window.is_minimized());
        assert_eq!(
            placement(window).unwrap().rcNormalPosition,
            original.rcNormalPosition
        );
        let frames = OWNED_PLAYBACK.with(|state| state.get().1);
        assert!(
            frames >= 10,
            "4K playback must present multiple frames, not merely fall back: {frames}"
        );
        println!(
            "owned 4K native animate minimize+restore: presented_frames={frames} elapsed_ms={} alpha_preserving_fast_stretch={}",
            started.elapsed().as_millis(),
            *FAST_ALPHA_STRETCH
        );
    }

    #[test]
    #[ignore = "explicit CPU-only renderer timing; no window is created"]
    fn render_budget_probe() {
        let source = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let target = Rect {
            left: 900,
            top: 1160,
            right: 964,
            bottom: 1224,
        };
        let extent = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1224,
        };
        let snapshot = Dib::new(source.width(), source.height()).unwrap();
        let mut surface = Dib::new(extent.width(), extent.height()).unwrap();
        for t in [0.0, 0.15, 0.35, 0.65, 0.85] {
            let started = Instant::now();
            draw(&mut surface, &snapshot, &source, &target, &extent, t).unwrap();
            println!("genie CPU 1080p t={t}: {}us", started.elapsed().as_micros());
        }
    }

    #[test]
    #[ignore = "explicit self-owned window state validation; no user application is touched"]
    fn owned_window_commit_cancel_close_and_maximized_restore() {
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongW, LWA_ALPHA, SW_SHOWMAXIMIZED, SW_SHOWNOACTIVATE,
            SetLayeredWindowAttributes, SetWindowLongW, ShowWindow,
        };
        let owned = super::super::hide_probe::OwnedWindow::new();
        let window = Window::from(owned.0);
        // Zero-alpha self-owned fixture; maximizing cannot flash on the desktop.
        unsafe {
            let style = GetWindowLongW(owned.0, GWL_EXSTYLE);
            SetWindowLongW(owned.0, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as i32);
            SetLayeredWindowAttributes(owned.0, COLORREF(0), 0, LWA_ALPHA).unwrap();
            // WINDOWPLACEMENT intentionally clamps wholly off-screen normal
            // rectangles. Production rejects those; keep this invisible test
            // fixture inside the primary monitor before saving placement.
            SetWindowPos(
                owned.0,
                None,
                120,
                120,
                320,
                240,
                SWP_NOACTIVATE | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
            .unwrap();
        }
        for maximized in [false, true] {
            unsafe {
                let _ = ShowWindow(
                    owned.0,
                    if maximized {
                        SW_SHOWMAXIMIZED
                    } else {
                        SW_SHOWNOACTIVATE
                    },
                );
            }
            let original = placement(window).unwrap();
            let generation = super::super::cancel();
            {
                let _action = super::super::ACTION_LOCK.lock();
                let hidden = HiddenWindowGuard::new(window, SW_SHOWNA, generation).unwrap();
                assert!(!window.is_visible());
                assert!(super::super::is_transient_visibility(window));
                super::super::cancel();
                drop(hidden);
            }
            assert!(window.is_visible());
            assert!(!window.is_minimized());
            assert_eq!(window.is_maximized(), maximized);
            assert!(!super::super::is_transient_visibility(window));
            let snapshot = super::super::Snapshot {
                hwnd: window.address(),
                pid: window.process().id(),
                class: window.class(),
                source: window.inner_rect().unwrap(),
                placement: original,
                texture_width: 0,
                texture_height: 0,
                pixels: Vec::new().into(),
                captured_at: Instant::now(),
            };
            let generation = super::super::cancel();
            commit_minimize(window, &snapshot, generation).unwrap();
            assert!(window.is_visible() && window.is_minimized());
            commit_restore(window, &snapshot, generation).unwrap();
            assert!(window.is_visible() && !window.is_minimized());
            assert_eq!(window.is_maximized(), maximized);
            assert_eq!(
                placement(window).unwrap().rcNormalPosition,
                original.rcNormalPosition
            );
        }
        let generation = super::super::cancel();
        let hidden = HiddenWindowGuard::new(window, SW_SHOWNA, generation).unwrap();
        drop(owned);
        drop(hidden);
        assert!(!window.is_window());
        assert!(!super::super::is_transient_visibility(window));
        println!(
            "owned Genie state commits: normal/maximized minimize+restore, hide cancellation, source destruction passed; no persistent DWM mutation"
        );
    }

    #[test]
    #[ignore = "explicit transparent self-owned HWND observed event playback; no user app or preference"]
    fn owned_observed_transitions_preserve_real_state_and_reveal_source() {
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongW, LWA_ALPHA, SW_SHOWMAXIMIZED, SW_SHOWNOACTIVATE,
            SetLayeredWindowAttributes, SetWindowLongW,
        };
        struct Playback;
        impl Drop for Playback {
            fn drop(&mut self) {
                OWNED_PLAYBACK.with(|state| state.set((false, 0)));
            }
        }
        OWNED_PLAYBACK.with(|state| state.set((true, 0)));
        let _playback = Playback;
        let owned = super::super::hide_probe::OwnedWindow::new();
        let window = Window::from(owned.0);
        unsafe {
            SetWindowLongW(
                owned.0,
                GWL_EXSTYLE,
                GetWindowLongW(owned.0, GWL_EXSTYLE) | WS_EX_LAYERED.0 as i32,
            );
            SetLayeredWindowAttributes(owned.0, COLORREF(0), 0, LWA_ALPHA).unwrap();
            SetWindowPos(
                owned.0,
                None,
                120,
                120,
                320,
                240,
                SWP_NOACTIVATE | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
            .unwrap();
        }
        for maximized in [false, true] {
            unsafe {
                let _ = ShowWindow(
                    owned.0,
                    if maximized {
                        SW_SHOWMAXIMIZED
                    } else {
                        SW_SHOWNOACTIVATE
                    },
                );
            }
            pump();
            let original = placement(window).unwrap();
            let snapshot = super::super::Snapshot {
                hwnd: window.address(),
                pid: window.process().id(),
                class: window.class(),
                source: window.inner_rect().unwrap(),
                placement: original,
                texture_width: 32,
                texture_height: 24,
                pixels: [13, 117, 245, 255].repeat(32 * 24).into(),
                captured_at: Instant::now(),
            };
            let target = Rect {
                left: 500,
                top: 700,
                right: 550,
                bottom: 750,
            };
            // State changes belong to the application/OS, before observation.
            unsafe {
                let _ = ShowWindow(owned.0, SW_SHOWMINNOACTIVE);
            }
            assert!(window.is_minimized());
            animate_observed(
                window,
                &snapshot,
                target.clone(),
                super::super::cancel(),
                false,
            )
            .unwrap();
            assert!(window.is_visible() && window.is_minimized());
            unsafe {
                SetWindowPlacement(owned.0, &original).unwrap();
            }
            assert!(!window.is_minimized());
            animate_observed(window, &snapshot, target, super::super::cancel(), true).unwrap();
            assert!(window.is_visible() && !window.is_minimized());
            assert_eq!(window.is_maximized(), maximized);
            assert_eq!(
                placement(window).unwrap().rcNormalPosition,
                original.rcNormalPosition
            );
            assert!(!super::super::is_transient_visibility(window));
        }
        assert!(OWNED_PLAYBACK.with(|state| state.get().1) >= 20);
    }

    #[test]
    #[ignore = "explicit transparent self-owned HWND accessible parking/cancellation probe"]
    fn owned_observed_restore_cancellation_leaves_reachable_iconic_window() {
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongW, LWA_ALPHA, SetLayeredWindowAttributes, SetWindowLongW,
        };
        let owned = super::super::hide_probe::OwnedWindow::new();
        let window = Window::from(owned.0);
        unsafe {
            SetWindowLongW(
                owned.0,
                GWL_EXSTYLE,
                GetWindowLongW(owned.0, GWL_EXSTYLE) | WS_EX_LAYERED.0 as i32,
            );
            SetLayeredWindowAttributes(owned.0, COLORREF(0), 0, LWA_ALPHA).unwrap();
        }
        let snapshot = super::super::Snapshot {
            hwnd: window.address(),
            pid: window.process().id(),
            class: window.class(),
            source: window.inner_rect().unwrap(),
            placement: placement(window).unwrap(),
            texture_width: 0,
            texture_height: 0,
            pixels: Vec::new().into(),
            captured_at: Instant::now(),
        };
        let generation = super::super::cancel();
        park_observed_restore(window, &snapshot, generation).unwrap();
        assert!(window.is_visible() && window.is_minimized());
        super::super::cancel();
        assert!(restore_observed_window(window, &snapshot, generation).is_err());
        // No Drop or compositor cleanup is required to make the app reachable
        // after the animator has stopped. The current command can restore it.
        assert!(window.is_visible() && window.is_minimized());
        restore_observed_window(window, &snapshot, super::super::cancel()).unwrap();
        assert!(window.is_visible() && !window.is_minimized());
        assert!(!super::super::is_transient_visibility(window));
    }

    #[test]
    fn offscreen_mesh_keeps_transparent_exterior_and_opaque_snapshot() {
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
            let bytes = unsafe { std::slice::from_raw_parts(surface.pixels, surface.len) };
            assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
            let painted: Vec<&[u8]> = bytes.chunks_exact(4).filter(|pixel| pixel[3] > 0).collect();
            assert!(!painted.is_empty());
            assert!(painted.iter().all(|pixel| **pixel == [13, 117, 245, 255]));
            if t == 0.0 {
                assert_eq!(
                    painted.len(),
                    source.width() as usize * source.height() as usize
                );
            }
            if t == 1.0 {
                assert_eq!(
                    painted.len(),
                    target.width() as usize * target.height() as usize
                );
            }
        }
    }
}
