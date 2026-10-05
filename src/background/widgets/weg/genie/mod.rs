//! Dock-initiated Genie transition. Does not intercept title-bar/Win+D actions.
//! A failed/late capture never prevents the real Windows minimize operation.

mod geometry;
mod native;

#[cfg(test)]
mod cloak_probe;
#[cfg(test)]
mod preview_fixture;
#[cfg(test)]
mod hide_probe;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use seelen_core::rect::Rect;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowDisplayAffinity, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};

use crate::windows_api::{
    event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
    window::Window,
};

static GENERATION: AtomicU64 = AtomicU64::new(0);
static ANIMATION_BUSY: AtomicBool = AtomicBool::new(false);
static CAPTURE_BUSY: AtomicBool = AtomicBool::new(false);
const MAX_PIXELS: i64 = 16_777_216;

struct BusyGuard(&'static AtomicBool);

impl BusyGuard {
    fn acquire(flag: &'static AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(flag))
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn cancel() -> u64 {
    GENERATION.fetch_add(1, Ordering::AcqRel) + 1
}

fn active(generation: u64) -> bool {
    !superseded(generation)
        && IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
        && IS_DISPLAY_ON.load(Ordering::Acquire)
}

fn superseded(generation: u64) -> bool {
    GENERATION.load(Ordering::Acquire) != generation
}

fn motion_enabled() -> bool {
    let mut enabled = windows_core::BOOL(0);
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut windows_core::BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && enabled.as_bool()
    }
}

fn valid_rect(rect: &Rect) -> bool {
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    width > 0 && height > 0 && width <= 8192 && height <= 8192 && width * height <= MAX_PIXELS
}

/// true: minimize was submitted, or a newer user action superseded this one.
/// false: caller should execute the ordinary native minimize immediately.
pub fn try_minimize(window: Window, target: Rect) -> bool {
    // NOT an accepted feature. This host cannot read the old DWM transition
    // flag, so do not add capture cost or touch application state by default.
    if std::env::var_os("MAC_UI_EXPERIMENTAL_DOCK_GENIE").is_none_or(|value| value != "1") {
        return false;
    }
    let generation = cancel();
    let Some(_animation) = BusyGuard::acquire(&ANIMATION_BUSY) else {
        return false;
    };
    if !active(generation)
        || !motion_enabled()
        || !window.is_manageable_from_unelevated()
        || window.is_minimized()
        || !valid_rect(&target)
        || target.width() > 512
        || target.height() > 512
    {
        return false;
    }
    let mut affinity = 0;
    if unsafe { GetWindowDisplayAffinity(window.hwnd(), &mut affinity) }.is_ok() && affinity != 0 {
        return false;
    }
    let Ok(source) = window.inner_rect() else {
        return false;
    };
    let Ok(outer) = window.outer_rect() else {
        return false;
    };
    let Ok(monitor) = window.monitor().info() else {
        return false;
    };
    let screen = monitor.monitorInfo.rcMonitor;
    // Cross-monitor, partly off-screen and oversized surfaces use the OS path.
    // No giant virtual-desktop overlay or mixed-DPI coordinate guesses.
    if !valid_rect(&source)
        || !valid_rect(&outer)
        || source.left < screen.left
        || source.top < screen.top
        || source.right > screen.right
        || source.bottom > screen.bottom
        || target.left < screen.left
        || target.top < screen.top
        || target.right > screen.right
        || target.bottom > screen.bottom
    {
        return false;
    }
    let Some(capture_guard) = BusyGuard::acquire(&CAPTURE_BUSY) else {
        return false;
    };
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let address = window.address();
    let capture_xy = [source.left - outer.left, source.top - outer.top];
    let capture_wh = [source.width(), source.height()];
    if capture_xy[0] < 0
        || capture_xy[1] < 0
        || source.right > outer.right
        || source.bottom > outer.bottom
    {
        return false;
    }
    // PrintWindow can block in another application. At most ONE capture worker
    // can remain blocked; subsequent actions take the OS path, not spawn more.
    let worker = std::thread::Builder::new()
        .name("dock-genie-capture".into())
        .spawn(move || {
            let _guard = capture_guard;
            let image = win_screenshot::capture::capture_window_ex(
                address,
                win_screenshot::capture::Using::PrintWindow,
                win_screenshot::capture::Area::Full,
                Some(capture_xy),
                Some(capture_wh),
            )
            .ok();
            let _ = tx.send(image);
        });
    if worker.is_err() {
        return false;
    }
    let Ok(Some(image)) = rx.recv_timeout(Duration::from_millis(80)) else {
        return superseded(generation);
    };
    if superseded(generation) {
        return true;
    }
    if !active(generation) {
        return false;
    }
    if !window.is_window()
        || window.is_minimized()
        || window.inner_rect().ok().as_ref() != Some(&source)
        || image.width != source.width() as u32
        || image.height != source.height() as u32
        || image.pixels.len() != image.width as usize * image.height as usize * 4
        || !image
            .pixels
            .chunks_exact(4)
            .any(|p| p[0] > 3 || p[1] > 3 || p[2] > 3)
    {
        return false;
    }
    match native::animate(window, source, target, image.pixels, generation) {
        Ok(()) => true,
        Err(error) => {
            log::trace!("Dock Genie skipped; native minimize fallback: {error}");
            superseded(generation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_guard_bounds_concurrency_and_releases_on_drop() {
        static TEST_BUSY: AtomicBool = AtomicBool::new(false);
        let lease = BusyGuard::acquire(&TEST_BUSY).unwrap();
        assert!(BusyGuard::acquire(&TEST_BUSY).is_none());
        drop(lease);
        assert!(BusyGuard::acquire(&TEST_BUSY).is_some());
    }

    #[test]
    fn rejects_invalid_and_excessive_allocation_before_capture() {
        assert!(!valid_rect(&Rect {
            left: i32::MIN,
            right: i32::MAX,
            top: 0,
            bottom: 5
        }));
        assert!(!valid_rect(&Rect {
            left: 0,
            right: 0,
            top: 0,
            bottom: 5
        }));
        assert!(!valid_rect(&Rect {
            left: 0,
            right: 8192,
            top: 0,
            bottom: 8192
        }));
        assert!(valid_rect(&Rect {
            left: -3840,
            right: 0,
            top: 0,
            bottom: 2160
        }));
    }
}
