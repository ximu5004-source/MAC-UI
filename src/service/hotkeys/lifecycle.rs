use std::sync::Mutex;

/// Capture belongs to the service process, not to a particular shortcut set.
/// The upstream hook leaves its STARTED flag set after stop, so replacing a
/// registry with an empty one must keep the hook and callback executor alive.
pub(crate) struct CaptureLifecycle {
    started: Mutex<bool>,
}

impl CaptureLifecycle {
    pub(crate) const fn new() -> Self {
        Self {
            started: Mutex::new(false),
        }
    }

    pub(crate) fn with_registry<E>(
        &self,
        needs_capture: bool,
        start: impl FnOnce() -> Result<(), E>,
        replace_registry: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), E> {
        // Keep registration updates serialized as well as hook startup. This
        // prevents concurrent IPC requests from clearing a half-replaced set.
        let mut started = self.started.lock().unwrap_or_else(|err| err.into_inner());
        if needs_capture && !*started {
            start()?;
            *started = true;
        }
        replace_registry()
    }
}

/// Identifies the listener currently owned by the shortcut editor. Hook
/// callbacks are queued on another thread, so an old completion can arrive
/// after a new listener has been installed.
pub(crate) struct RegistrationSession<T> {
    active: Mutex<Option<T>>,
}

impl<T: Copy + PartialEq> RegistrationSession<T> {
    pub(crate) const fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    pub(crate) fn begin(&self, request_id: T, install_listener: impl FnOnce()) {
        let mut active = self.active.lock().unwrap_or_else(|err| err.into_inner());
        *active = Some(request_id);
        install_listener();
    }

    pub(crate) fn finish(&self, request_id: T, remove_listener: impl FnOnce()) -> bool {
        let mut active = self.active.lock().unwrap_or_else(|err| err.into_inner());
        if *active != Some(request_id) {
            return false;
        }
        *active = None;
        remove_listener();
        true
    }

    pub(crate) fn cancel(&self, free_keyboard: impl FnOnce()) {
        let mut active = self.active.lock().unwrap_or_else(|err| err.into_inner());
        *active = None;
        free_keyboard();
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureLifecycle, RegistrationSession};
    use std::cell::{Cell, RefCell};

    struct FakeRuntime {
        starts: Cell<usize>,
        clears: Cell<usize>,
        capture_alive: Cell<bool>,
        registry: RefCell<Vec<&'static str>>,
    }

    impl FakeRuntime {
        fn new() -> Self {
            Self {
                starts: Cell::new(0),
                clears: Cell::new(0),
                capture_alive: Cell::new(false),
                registry: RefCell::new(Vec::new()),
            }
        }

        fn apply(&self, capture: &CaptureLifecycle, keys: Vec<&'static str>) {
            capture
                .with_registry(
                    !keys.is_empty(),
                    || {
                        self.starts.set(self.starts.get() + 1);
                        assert!(!self.capture_alive.replace(true));
                        Ok::<_, ()>(())
                    },
                    || {
                        self.clears.set(self.clears.get() + 1);
                        *self.registry.borrow_mut() = keys;
                        Ok(())
                    },
                )
                .unwrap();
        }
    }

    #[test]
    fn nonempty_empty_nonempty_keeps_capture_and_replaces_the_registry() {
        let capture = CaptureLifecycle::new();
        let runtime = FakeRuntime::new();

        runtime.apply(&capture, vec!["Win", "RWin"]);
        assert_eq!(*runtime.registry.borrow(), ["Win", "RWin"]);

        runtime.apply(&capture, vec![]);
        assert!(runtime.registry.borrow().is_empty());
        assert!(runtime.capture_alive.get());

        runtime.apply(&capture, vec!["Win", "RWin", "Win+E"]);
        assert_eq!(*runtime.registry.borrow(), ["Win", "RWin", "Win+E"]);
        assert!(runtime.capture_alive.get());
        assert_eq!(runtime.starts.get(), 1);
        assert_eq!(runtime.clears.get(), 3);
    }

    #[test]
    fn initial_empty_registry_does_not_start_capture() {
        let capture = CaptureLifecycle::new();
        let runtime = FakeRuntime::new();
        runtime.apply(&capture, vec![]);
        assert_eq!(runtime.starts.get(), 0);
        assert!(!runtime.capture_alive.get());
        runtime.apply(&capture, vec!["Win"]);
        assert_eq!(runtime.starts.get(), 1);
        assert_eq!(*runtime.registry.borrow(), ["Win"]);
    }

    #[test]
    fn failed_start_is_reported_and_retry_can_start_capture() {
        let capture = CaptureLifecycle::new();
        let replaced = Cell::new(false);
        let result = capture.with_registry(
            true,
            || Err("capture failed"),
            || {
                replaced.set(true);
                Ok(())
            },
        );
        assert_eq!(result, Err("capture failed"));
        assert!(!replaced.get());

        let runtime = FakeRuntime::new();
        runtime.apply(&capture, vec!["Win"]);
        assert!(runtime.capture_alive.get());
        assert_eq!(runtime.starts.get(), 1);
    }

    #[test]
    fn registry_failure_does_not_start_another_executor_on_retry() {
        let capture = CaptureLifecycle::new();
        let starts = Cell::new(0);
        let result = capture.with_registry(
            true,
            || {
                starts.set(starts.get() + 1);
                Ok(())
            },
            || Err("registration failed"),
        );
        assert_eq!(result, Err("registration failed"));
        capture
            .with_registry(
                true,
                || {
                    starts.set(starts.get() + 1);
                    Ok::<_, &str>(())
                },
                || Ok(()),
            )
            .unwrap();
        assert_eq!(starts.get(), 1);
    }

    #[test]
    fn shortcut_editor_can_start_capture_before_any_enabled_registry() {
        let capture = CaptureLifecycle::new();
        let runtime = FakeRuntime::new();
        runtime.apply(&capture, vec![]);
        capture
            .with_registry(
                true,
                || {
                    runtime.starts.set(runtime.starts.get() + 1);
                    runtime.capture_alive.set(true);
                    Ok::<_, ()>(())
                },
                || Ok(()),
            )
            .unwrap();
        runtime.apply(&capture, vec!["Win", "RWin"]);
        assert_eq!(runtime.starts.get(), 1);
        assert_eq!(*runtime.registry.borrow(), ["Win", "RWin"]);
    }

    #[test]
    fn concurrent_registry_updates_share_one_capture_and_do_not_interleave() {
        use std::sync::{
            Barrier,
            atomic::{AtomicUsize, Ordering},
        };

        let capture = CaptureLifecycle::new();
        let starts = AtomicUsize::new(0);
        let in_registry = AtomicUsize::new(0);
        let completed = AtomicUsize::new(0);
        let ready = Barrier::new(8);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    ready.wait();
                    capture
                        .with_registry(
                            true,
                            || {
                                starts.fetch_add(1, Ordering::SeqCst);
                                Ok::<_, ()>(())
                            },
                            || {
                                assert_eq!(in_registry.fetch_add(1, Ordering::SeqCst), 0);
                                std::thread::yield_now();
                                completed.fetch_add(1, Ordering::SeqCst);
                                assert_eq!(in_registry.fetch_sub(1, Ordering::SeqCst), 1);
                                Ok(())
                            },
                        )
                        .unwrap();
                });
            }
        });
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(completed.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn delayed_old_completion_cannot_remove_the_new_recording_listener() {
        let session = RegistrationSession::new();
        let listener = Cell::new(None);
        session.begin(1, || listener.set(Some(1)));
        session.cancel(|| listener.set(None));
        session.begin(2, || listener.set(Some(2)));

        assert!(!session.finish(1, || listener.set(None)));
        assert_eq!(listener.get(), Some(2));
        assert!(session.finish(2, || listener.set(None)));
        assert_eq!(listener.get(), None);
    }

    #[test]
    fn cancellation_cleans_up_synchronously_and_completion_is_idempotent() {
        let session = RegistrationSession::new();
        let cleanups = Cell::new(0);
        session.begin(1, || {});
        session.cancel(|| cleanups.set(cleanups.get() + 1));
        assert_eq!(cleanups.get(), 1);
        assert!(!session.finish(1, || cleanups.set(cleanups.get() + 1)));

        session.begin(2, || {});
        assert!(session.finish(2, || cleanups.set(cleanups.get() + 1)));
        assert!(!session.finish(2, || cleanups.set(cleanups.get() + 1)));
        assert_eq!(cleanups.get(), 2);
    }
}
