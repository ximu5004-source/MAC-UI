//! Genie transitions from Dock items and Windows minimize/restore events.
//! A failed/late capture never prevents the real Windows minimize operation.

mod cache;
mod geometry;
mod native;
mod system_animation;
mod system_events;
pub(super) mod targets;
mod titlebar;

pub(crate) fn start_titlebar_hook() {
    system_events::start();
    titlebar::start();
}

pub(crate) fn shutdown() {
    system_events::shutdown();
    cancel();
    system_animation::restore();
}

pub(crate) fn service_recovered() {
    // The capture worker re-evaluates the current preference; never replay an
    // old suppression request or an old native window command after reconnect.
    system_animation::service_recovered();
}

#[cfg(test)]
mod cloak_probe;
#[cfg(test)]
mod hide_probe;
#[cfg(test)]
mod preview_fixture;

use std::sync::{
    LazyLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use macui_core::rect::Rect;
use windows::Win32::UI::WindowsAndMessaging::{GetWindowDisplayAffinity, SW_MINIMIZE, SW_RESTORE};

use crate::windows_api::{
    event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
    window::Window,
};
use crate::{
    error::Result,
    state::application::{FULL_STATE, performance::PERFORMANCE_MODE},
};
use cache::{Snapshot, SnapshotCache};
use parking_lot::Mutex;

static GENERATION: AtomicU64 = AtomicU64::new(0);
static ANIMATION_BUSY: AtomicBool = AtomicBool::new(false);
static CAPTURE_BUSY: AtomicBool = AtomicBool::new(false);
const MAX_PIXELS: i64 = 16_777_216;
static CACHE: LazyLock<Mutex<SnapshotCache>> =
    LazyLock::new(|| Mutex::new(SnapshotCache::default()));
// Only short window-state mutations are serialized, never a capture or frame loop.
// A newer Dock action cancels first, then queues after any in-flight hide/commit.
static ACTION_LOCK: Mutex<()> = Mutex::new(());
static TRANSIENT_VISIBILITY: Mutex<Option<(isize, u32, u64)>> = Mutex::new(None);

/// Only the current synchronous hide/commit lease, never a timed suppression.
pub(crate) fn is_transient_visibility(window: Window) -> bool {
    let lease = *TRANSIENT_VISIBILITY.lock();
    lease.is_some_and(|(hwnd, pid, _generation)| {
        hwnd == window.address() && window.is_window() && window.process().id() == pid
    })
}

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

fn native_action(generation: u64, action: impl FnOnce() -> Result<()>) -> Result<()> {
    let _action = ACTION_LOCK.lock();
    if superseded(generation) {
        Ok(())
    } else {
        action()
    }
}

pub fn toggle(window: Window, minimize: bool, target: Option<Rect>) -> Result<()> {
    let generation = cancel();
    let handled = target.is_some_and(|target| {
        if minimize {
            try_minimize(window, target, generation)
        } else {
            try_restore(window, target, generation)
        }
    });
    if handled {
        return Ok(());
    }
    native_action(generation, || {
        if minimize {
            system_events::expect_owned(window, false);
            window.show_window_async(SW_MINIMIZE)
        } else {
            if window.is_minimized() || !window.is_visible() {
                // Never synchronously wait on a hung application's UI thread.
                // SW_RESTORE also queues activation on the owning UI thread.
                system_events::expect_owned(window, true);
                window.show_window_async(SW_RESTORE)?;
            }
            window.focus()
        }
    })
}

pub(super) fn enabled() -> bool {
    FULL_STATE.load().settings.by_widget.weg.window_animation
        == macui_core::state::WegWindowAnimation::Genie
        && PERFORMANCE_MODE.load() == macui_core::state::PerformanceMode::Disabled
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
    // Selecting Genie is an explicit app-level motion preference. Suppressing
    // native Windows motion must not feed back into disabling this effect.
    // Performance mode and the Native option still disable Genie via enabled().
    enabled() && system_animation::confirmed()
}

fn valid_rect(rect: &Rect) -> bool {
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    width > 0 && height > 0 && width <= 8192 && height <= 8192 && width * height <= MAX_PIXELS
}

/// true: minimize was submitted, or a newer user action superseded this one.
/// false: caller should execute the ordinary native minimize immediately.
fn try_minimize(window: Window, target: Rect, generation: u64) -> bool {
    if !enabled() {
        return false;
    }
    let Some(_animation) = BusyGuard::acquire(&ANIMATION_BUSY) else {
        return false;
    };
    if !active(generation)
        || !motion_enabled()
        || !window.is_manageable_from_unelevated()
        || window.is_minimized()
        || !window.is_visible()
        || window.is_cloaked()
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
    let pid = window.process().id();
    let class = window.class();
    if source.left < outer.left
        || source.top < outer.top
        || source.right > outer.right
        || source.bottom > outer.bottom
    {
        return false;
    }
    // PrintWindow can block in another application. At most ONE capture worker
    // can remain blocked; subsequent actions take the OS path, not spawn more.
    let capture_source = source.clone();
    let worker = std::thread::Builder::new()
        .name("dock-genie-capture".into())
        .spawn(move || {
            let _guard = capture_guard;
            let image = native::capture(window, &capture_source, &outer).ok();
            let _ = tx.send(image);
        });
    if worker.is_err() {
        return false;
    }
    // 4K PrintWindow + native downsampling needs a larger bounded allowance.
    // Poll cancellation instead of making a newer click wait the full budget.
    let started = std::time::Instant::now();
    let image = loop {
        if superseded(generation) {
            return true;
        }
        if !active(generation) || started.elapsed() >= Duration::from_millis(250) {
            return false;
        }
        match rx.recv_timeout(Duration::from_millis(8)) {
            Ok(Some(image)) => break image,
            Ok(None) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return false,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    if superseded(generation) {
        return true;
    }
    if !active(generation) {
        return false;
    }
    if !window.is_window()
        || window.process().id() != pid
        || window.class() != class
        || window.is_minimized()
        || window.inner_rect().ok().as_ref() != Some(&source)
        || image.width <= 0
        || image.height <= 0
        || image.pixels.len() != image.width as usize * image.height as usize * 4
        || !image
            .pixels
            .chunks_exact(4)
            .any(|p| p[0] > 3 || p[1] > 3 || p[2] > 3)
    {
        return false;
    }
    let Ok(placement) = native::placement(window) else {
        return false;
    };
    let snapshot = Snapshot {
        hwnd: window.address(),
        pid,
        class,
        source,
        placement,
        texture_width: image.width,
        texture_height: image.height,
        pixels: image.pixels.into(),
        captured_at: std::time::Instant::now(),
    };
    CACHE.lock().insert(snapshot.clone());
    match native::animate(window, &snapshot, target, generation, false) {
        Ok(()) => true,
        Err(error) => {
            log::trace!("Dock Genie skipped; native minimize fallback: {error}");
            let submitted = window.is_minimized();
            if !submitted {
                CACHE.lock().remove(window.address());
            }
            submitted || superseded(generation)
        }
    }
}

/// Restore only an exact, bounded, recent screenshot captured by our minimize.
/// Windows-native minimizations and changed placements use the ordinary path.
fn try_restore(window: Window, target: Rect, generation: u64) -> bool {
    if !enabled() {
        return false;
    }
    let Some(_animation) = BusyGuard::acquire(&ANIMATION_BUSY) else {
        return false;
    };
    if !active(generation)
        || !motion_enabled()
        || !window.is_minimized()
        || !window.is_manageable_from_unelevated()
        || window.is_cloaked()
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
    let pid = window.process().id();
    let class = window.class();
    let Some(snapshot) = CACHE.lock().get(window.address(), pid, &class) else {
        return false;
    };
    let Ok(current) = native::placement(window) else {
        return false;
    };
    let a = current.rcNormalPosition;
    let b = snapshot.placement.rcNormalPosition;
    if !valid_rect(&snapshot.source)
        || (a.left, a.top, a.right, a.bottom) != (b.left, b.top, b.right, b.bottom)
        || !native::same_monitor(window, &snapshot.source, &target)
    {
        CACHE.lock().remove(window.address());
        return false;
    }
    match native::animate(window, &snapshot, target, generation, true) {
        Ok(()) => true,
        Err(error) => {
            log::trace!("Dock Genie restore skipped; native fallback: {error}");
            superseded(generation)
        }
    }
}

/// Samples are taken off the hook/UI threads. At most one PrintWindow worker
/// exists, including a worker stuck inside an unresponsive application.
fn capture_live_snapshot(window: Window) -> Option<Snapshot> {
    if !enabled()
        || ANIMATION_BUSY.load(Ordering::Acquire)
        || !window.is_window()
        || window.is_minimized()
        || !window.is_visible()
        || window.is_cloaked()
        || !window.is_manageable_from_unelevated()
    {
        return None;
    }
    let mut affinity = 0;
    if unsafe { GetWindowDisplayAffinity(window.hwnd(), &mut affinity) }.is_ok() && affinity != 0 {
        return None;
    }
    let source = window.inner_rect().ok()?;
    let outer = window.outer_rect().ok()?;
    if !valid_rect(&source) || !valid_rect(&outer) {
        return None;
    }
    let placement = native::placement(window).ok()?;
    let pid = window.process().id();
    let class = window.class();
    let capture_guard = BusyGuard::acquire(&CAPTURE_BUSY)?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let capture_source = source.clone();
    std::thread::Builder::new()
        .name("system-genie-capture".into())
        .spawn(move || {
            let _guard = capture_guard;
            let _ = tx.send(native::capture(window, &capture_source, &outer).ok());
        })
        .ok()?;
    let image = rx.recv_timeout(Duration::from_millis(180)).ok()??;
    if !window.is_window()
        || window.process().id() != pid
        || window.is_minimized()
        || window.inner_rect().ok().as_ref() != Some(&source)
        || !image
            .pixels
            .chunks_exact(4)
            .any(|p| p[0] > 3 || p[1] > 3 || p[2] > 3)
    {
        return None;
    }
    Some(Snapshot {
        hwnd: window.address(),
        pid,
        class,
        source,
        placement,
        texture_width: image.width,
        texture_height: image.height,
        pixels: image.pixels.into(),
        captured_at: std::time::Instant::now(),
    })
}

/// The Windows operation has already happened. A cached frame preserves its
/// original geometry; restore is parked iconic only during reverse playback.
fn animate_system_event(window: Window, restore: bool) -> Result<bool> {
    if !enabled() || !motion_enabled() || !window.is_manageable_from_unelevated() {
        return Ok(false);
    }
    let Some(_animation) = BusyGuard::acquire(&ANIMATION_BUSY) else {
        return Ok(false);
    };
    let Some(target) = targets::target_for(window) else {
        return Ok(false);
    };
    let pid = window.process().id();
    let class = window.class();
    let Some(snapshot) = CACHE.lock().get(window.address(), pid, &class) else {
        log::debug!(
            "System Genie skipped: no captured frame, hwnd={}, restore={restore}",
            window.address()
        );
        return Ok(false);
    };
    let mut affinity = 0;
    if unsafe { GetWindowDisplayAffinity(window.hwnd(), &mut affinity) }.is_ok() && affinity != 0 {
        CACHE.lock().remove(window.address());
        return Ok(false);
    }
    if !native::same_monitor(window, &snapshot.source, &target)
        || !valid_rect(&target)
        || target.width() > 512
        || target.height() > 512
    {
        return Ok(false);
    }
    let current = native::placement(window)?;
    let a = current.rcNormalPosition;
    let b = snapshot.placement.rcNormalPosition;
    if (a.left, a.top, a.right, a.bottom) != (b.left, b.top, b.right, b.bottom)
        || (restore && window.inner_rect().ok().as_ref() != Some(&snapshot.source))
    {
        CACHE.lock().remove(window.address());
        return Ok(false);
    }
    let generation = cancel();
    if !active(generation) {
        return Ok(false);
    }
    log::debug!(
        "System Genie animation: hwnd={}, restore={restore}, capture_age_ms={}",
        window.address(),
        snapshot.captured_at.elapsed().as_millis()
    );
    native::animate_observed(window, &snapshot, target, generation, restore)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superseded_native_fallback_cannot_overwrite_a_newer_action() {
        let old = cancel();
        let current = cancel();
        native_action(old, || panic!("stale fallback ran")).unwrap();
        let invoked = AtomicBool::new(false);
        native_action(current, || {
            invoked.store(true, Ordering::Relaxed);
            Ok(())
        })
        .unwrap();
        assert!(invoked.load(Ordering::Relaxed));
    }

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
        assert!(valid_rect(&Rect {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2160
        }));
        assert!(valid_rect(&Rect {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080
        }));
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
