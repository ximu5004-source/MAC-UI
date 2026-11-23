//! Observe OS minimize/restore, including shortcuts and custom title bars.
//! Hooks enqueue only; capture and animation never run on a hook/UI thread.
use std::{
    collections::HashMap,
    sync::{
        LazyLock, Once, OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            DispatchMessageW, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GWL_EXSTYLE,
            GetMessageW, GetWindowLongPtrW, GetWindowThreadProcessId, MSG, OBJID_WINDOW,
            PostThreadMessageW, TranslateMessage, WINEVENT_OUTOFCONTEXT, WM_QUIT, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW,
        },
    },
};

use crate::windows_api::{
    WindowEnumerator,
    event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
    window::Window,
};

static START: Once = Once::new();
static STOPPED: AtomicBool = AtomicBool::new(false);
static EVENTS_READY: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static ACTIONS: OnceLock<SyncSender<Request>> = OnceLock::new();
static OWNED: LazyLock<Mutex<ExpectedTransitions>> =
    LazyLock::new(|| Mutex::new(ExpectedTransitions::default()));
static INTENTS: LazyLock<Mutex<HashMap<Identity, (bool, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const EVENT_MAX_AGE: Duration = Duration::from_millis(250);
const CAPTURE_INTERVAL: Duration = Duration::from_millis(600);
const BACKGROUND_CAPTURE_INTERVAL: Duration = Duration::from_secs(10);
const MAX_CANDIDATES: usize = 32;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Identity {
    hwnd: isize,
    pid: u32,
}

impl Identity {
    fn of(window: Window) -> Option<Self> {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(window.hwnd(), Some(&mut pid)) };
        (pid != 0).then_some(Self {
            hwnd: window.address(),
            pid,
        })
    }

    fn window(self) -> Option<Window> {
        let window = Window::from(self.hwnd);
        (Self::of(window) == Some(self) && window.is_window()).then_some(window)
    }
}

#[derive(Default)]
struct ExpectedTransitions(HashMap<Identity, Vec<(bool, Instant)>>);

impl ExpectedTransitions {
    fn insert(&mut self, identity: Identity, restore: bool, now: Instant) {
        self.0.retain(|_, values| {
            values.retain(|(_, until)| *until > now);
            !values.is_empty()
        });
        // Only commands actually submitted by our native state commit arrive
        // here. Bounded expectations cannot suppress unrelated later input.
        if self.0.len() >= MAX_CANDIDATES && !self.0.contains_key(&identity) {
            return;
        }
        let values = self.0.entry(identity).or_default();
        if values.len() < 4 {
            values.push((restore, now + Duration::from_secs(2)));
        }
    }

    fn consume(&mut self, identity: Identity, restore: bool, now: Instant) -> bool {
        let Some(values) = self.0.get_mut(&identity) else {
            return false;
        };
        values.retain(|(_, until)| *until > now);
        let found = values.iter().position(|(kind, _)| *kind == restore);
        if let Some(index) = found {
            values.remove(index);
        }
        if values.is_empty() {
            self.0.remove(&identity);
        }
        found.is_some()
    }
}

pub(super) fn expect_owned(window: Window, restore: bool) {
    if let Some(identity) = Identity::of(window) {
        let now = Instant::now();
        record_intent(identity, restore, now);
        OWNED.lock().insert(identity, restore, now);
    }
}

fn record_intent(identity: Identity, restore: bool, now: Instant) {
    let mut intents = INTENTS.lock();
    intents.retain(|_, (_, time)| now.duration_since(*time) < Duration::from_secs(2));
    if intents.len() >= MAX_CANDIDATES
        && !intents.contains_key(&identity)
        && let Some(oldest) = intents
            .iter()
            .min_by_key(|(_, (_, time))| *time)
            .map(|(identity, _)| *identity)
    {
        intents.remove(&oldest);
    }
    intents.insert(identity, (!restore, now));
}

/// Submission intent, rather than the current IsIconic bit, handles an async
/// minimize which is queued but has not executed on the source UI thread yet.
pub(super) fn minimized_intent_after(window: Window, since: Instant) -> Option<bool> {
    let identity = Identity::of(window)?;
    INTENTS
        .lock()
        .get(&identity)
        .and_then(|(minimized, time)| (*time >= since).then_some(*minimized))
}

struct Request {
    identity: Identity,
    restore: bool,
    received: Instant,
}

extern "system" fn event_proc(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if object != OBJID_WINDOW.0 || STOPPED.load(Ordering::Acquire) || !playback_available() {
        return;
    }
    let restore = event == EVENT_SYSTEM_MINIMIZEEND;
    let window = Window::from(hwnd);
    let Some(identity) = Identity::of(window) else {
        return;
    };
    let now = Instant::now();
    if OWNED.lock().consume(identity, restore, now) {
        return;
    }
    record_intent(identity, restore, now);
    // A new external OS action outranks any frame loop already in flight.
    // Owned commands were consumed above, so they cannot cancel themselves.
    super::cancel();
    if let Some(sender) = ACTIONS.get() {
        let _ = sender.try_send(Request {
            identity,
            restore,
            received: now,
        });
    }
}

fn desired_available() -> bool {
    EVENTS_READY.load(Ordering::Acquire)
        && super::enabled()
        && crate::state::application::FULL_STATE
            .load()
            .is_weg_enabled()
        && IS_DISPLAY_ON.load(Ordering::Acquire)
        && IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
}

fn playback_available() -> bool {
    desired_available() && super::system_animation::confirmed()
}

fn sample_candidate(window: Window) -> bool {
    if !window.is_window() || !window.is_visible() || window.is_minimized() || window.is_cloaked() {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(window.hwnd(), GWL_EXSTYLE) } as u32;
    style & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) == 0
        && window.owner().is_none()
        && super::targets::target_for(window).is_some()
}

fn capture_loop() {
    let mut sampled = HashMap::<Identity, Instant>::new();
    let mut last_sweep = Instant::now() - Duration::from_secs(1);
    let mut candidates = Vec::new();
    while !STOPPED.load(Ordering::Acquire) {
        // Acquiring custody must depend on desired policy, not confirmed
        // custody; otherwise the initial false confirmation prevents startup.
        let available = desired_available();
        super::system_animation::sync(available);
        if available && !super::ANIMATION_BUSY.load(Ordering::Acquire) {
            let now = Instant::now();
            if last_sweep.elapsed() >= Duration::from_secs(1) {
                candidates.clear();
                let _ = WindowEnumerator::new().for_each(|window| {
                    if candidates.len() < MAX_CANDIDATES && sample_candidate(window) {
                        candidates.push(window);
                    }
                });
                sampled.retain(|identity, _| identity.window().is_some());
                last_sweep = now;
            }
            let foreground = Window::get_foregrounded();
            let foreground_due = Identity::of(foreground).filter(|id| {
                sampled
                    .get(id)
                    .is_none_or(|time| now.duration_since(*time) >= CAPTURE_INTERVAL)
                    && sample_candidate(foreground)
            });
            let next = foreground_due.map(|_| foreground).or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .filter(|window| {
                        Identity::of(*window).is_some_and(|id| {
                            sampled.get(&id).is_none_or(|time| {
                                now.duration_since(*time) >= BACKGROUND_CAPTURE_INTERVAL
                            })
                        })
                    })
                    .min_by_key(|window| {
                        Identity::of(*window).and_then(|id| sampled.get(&id).copied())
                    })
            });
            if let Some(window) = next
                && let Some(identity) = Identity::of(window)
            {
                // A hung capture is attempted once per interval, not every loop.
                sampled.insert(identity, now);
                if let Some(snapshot) = super::capture_live_snapshot(window)
                    && !STOPPED.load(Ordering::Acquire)
                {
                    super::CACHE.lock().insert(snapshot);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

pub(crate) fn start() {
    START.call_once(|| {
        let (sender, receiver) = sync_channel::<Request>(16);
        let worker = std::thread::Builder::new()
            .name("system-genie-actions".into())
            .spawn(move || {
                while !STOPPED.load(Ordering::Acquire) {
                    let request = match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(request) => request,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    if !playback_available() || request.received.elapsed() > EVENT_MAX_AGE {
                        continue;
                    }
                    let Some(window) = request.identity.window() else {
                        continue;
                    };
                    // Out-of-context WinEvents describe an operation about to occur.
                    // Wait off the hook thread for its genuine iconic state, never
                    // issue a second minimize/restore command to force the event.
                    while window.is_minimized() == request.restore
                        && request.received.elapsed() < Duration::from_millis(120)
                    {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    if window.is_minimized() == request.restore {
                        continue;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        super::animate_system_event(window, request.restore)
                    }));
                    if let Ok(Err(error)) = result {
                        log::debug!("System Genie native operation preserved: {error}");
                    }
                }
            });
        if let Err(error) = worker {
            log::warn!("System animation worker unavailable: {error}");
            return;
        }
        let _ = ACTIONS.set(sender);
        let hook_thread = std::thread::Builder::new()
            .name("system-genie-events".into())
            .spawn(|| unsafe {
                HOOK_THREAD.store(GetCurrentThreadId(), Ordering::Release);
                let hook = SetWinEventHook(
                    EVENT_SYSTEM_MINIMIZESTART,
                    EVENT_SYSTEM_MINIMIZEEND,
                    None,
                    Some(event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                if hook.is_invalid() {
                    log::warn!("System minimize/restore event hook unavailable");
                    return;
                }
                EVENTS_READY.store(true, Ordering::Release);
                log::info!("System minimize/restore Genie event input ready");
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let _ = UnhookWinEvent(hook);
                EVENTS_READY.store(false, Ordering::Release);
                HOOK_THREAD.store(0, Ordering::Release);
            });
        if let Err(error) = hook_thread {
            log::warn!("System animation event thread unavailable: {error}");
            return;
        }
        if let Err(error) = std::thread::Builder::new()
            .name("system-genie-samples".into())
            .spawn(capture_loop)
        {
            log::warn!("System animation snapshot sampler unavailable: {error}");
        }
    });
}

pub(super) fn shutdown() {
    STOPPED.store(true, Ordering::Release);
    EVENTS_READY.store(false, Ordering::Release);
    let thread = HOOK_THREAD.load(Ordering::Acquire);
    if thread != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_commands_are_consumed_once_without_swallowing_another_window_or_direction() {
        let now = Instant::now();
        let a = Identity { hwnd: 1, pid: 2 };
        let mut pending = ExpectedTransitions::default();
        pending.insert(a, false, now);
        assert!(!pending.consume(Identity { pid: 3, ..a }, false, now));
        assert!(!pending.consume(a, true, now));
        assert!(pending.consume(a, false, now));
        assert!(!pending.consume(a, false, now));
        pending.insert(a, true, now);
        assert!(!pending.consume(a, true, now + Duration::from_secs(3)));
    }

    #[test]
    fn pending_expectations_are_bounded_and_expiry_releases_capacity() {
        let now = Instant::now();
        let mut pending = ExpectedTransitions::default();
        for hwnd in 0..1000 {
            pending.insert(Identity { hwnd, pid: 1 }, false, now);
        }
        assert_eq!(pending.0.len(), MAX_CANDIDATES);
        let identity = Identity { hwnd: 0, pid: 1 };
        for _ in 0..1000 {
            pending.insert(identity, true, now);
        }
        assert_eq!(pending.0[&identity].len(), 4);
        pending.insert(
            Identity { hwnd: 999, pid: 1 },
            true,
            now + Duration::from_secs(3),
        );
        assert_eq!(pending.0.len(), 1);
    }
}
