//! The service owns the reversible session preference and an exact-process
//! watchdog. The UI never changes a user preference without a live custodian.
use std::{
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use macui_ipc::{ServiceIpc, messages::SvcAction};
use parking_lot::Mutex;

struct State {
    // None means a request may have reached the service but its acknowledgement
    // was lost. A later disable must still release that possible lease.
    applied: Option<bool>,
    last_failure: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            applied: Some(false),
            last_failure: None,
        }
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
// The input hook must never wait behind an IPC request holding STATE for up to
// two seconds. Only a real service acknowledgement grants playback permission.
static CONFIRMED: AtomicBool = AtomicBool::new(false);
static NEEDS_RENEWAL: AtomicBool = AtomicBool::new(false);

pub(super) fn confirmed() -> bool {
    CONFIRMED.load(Ordering::Acquire)
        && !NEEDS_RENEWAL.load(Ordering::Acquire)
        && !SHUTTING_DOWN.load(Ordering::Acquire)
}

pub(super) fn service_recovered() {
    CONFIRMED.store(false, Ordering::Release);
    NEEDS_RENEWAL.store(true, Ordering::Release);
}

pub(super) fn sync(suppressed: bool) {
    let mut state = STATE.lock();
    if NEEDS_RENEWAL.swap(false, Ordering::AcqRel) {
        state.applied = None;
        state.last_failure = None;
    }
    let suppressed = suppressed && !SHUTTING_DOWN.load(Ordering::Acquire);
    if !suppressed {
        // Stop custom frames immediately, even when release must be retried.
        CONFIRMED.store(false, Ordering::Release);
    }
    if state.applied == Some(suppressed)
        || state
            .last_failure
            .is_some_and(|time| time.elapsed() < Duration::from_secs(5))
    {
        return;
    }
    let message = SvcAction::SetNativeWindowAnimationSuppressed {
        owner_pid: std::process::id(),
        suppressed,
    };
    let result = crate::get_tokio_handle().block_on(async {
        tokio::time::timeout(Duration::from_secs(2), ServiceIpc::send(message)).await
    });
    match result {
        Ok(Ok(())) => {
            state.applied = Some(suppressed);
            state.last_failure = None;
            CONFIRMED.store(
                suppressed && !SHUTTING_DOWN.load(Ordering::Acquire),
                Ordering::Release,
            );
            log::info!("Native minimize/restore animation service lease: suppressed={suppressed}");
        }
        error => {
            CONFIRMED.store(false, Ordering::Release);
            state.applied = None;
            state.last_failure = Some(Instant::now());
            log::warn!(
                "Native window animation preference unchanged: service lease failed: {error:?}"
            );
        }
    }
}

pub(super) fn restore() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    CONFIRMED.store(false, Ordering::Release);
    // The Tauri event loop itself runs inside #[tokio::main]. A Handle::block_on
    // directly in RunEvent::Exit would panic, so release on a plain OS thread.
    let release = std::thread::Builder::new()
        .name("genie-preference-release".into())
        .spawn(|| {
            STATE.lock().last_failure = None;
            sync(false);
        });
    match release {
        Ok(thread) => {
            let _ = thread.join();
        }
        Err(error) => log::warn!(
            "Animation lease release worker failed; service owner-exit watchdog remains active: {error}"
        ),
    }
}
