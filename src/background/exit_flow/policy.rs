use std::sync::atomic::{AtomicU8, Ordering};

const RUNNING: u8 = 0;
const STOPPING: u8 = 1;
const READY: u8 = 2;

/// A second exit request must not race the service stop already in progress.
pub(super) struct ExitGate(AtomicU8);

impl ExitGate {
    pub const fn new() -> Self {
        Self(AtomicU8::new(RUNNING))
    }

    pub fn is_ready(&self) -> bool {
        self.0.load(Ordering::Acquire) == READY
    }

    pub fn begin(&self) -> bool {
        self.0
            .compare_exchange(RUNNING, STOPPING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn complete(&self) {
        self.0.store(READY, Ordering::Release);
    }

    pub fn retry(&self) {
        self.0.store(RUNNING, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::ExitGate;

    #[test]
    fn repeated_exit_requests_share_one_stop_and_only_complete_after_success() {
        let gate = ExitGate::new();
        assert!(!gate.is_ready());
        assert!(gate.begin());
        assert!(!gate.begin());
        assert!(!gate.is_ready());
        gate.complete();
        assert!(gate.is_ready());
        assert!(!gate.begin());
    }

    #[test]
    fn failed_stop_keeps_the_app_running_and_allows_retry() {
        let gate = ExitGate::new();
        assert!(gate.begin());
        gate.retry();
        assert!(!gate.is_ready());
        assert!(gate.begin());
        assert!(!gate.begin());
    }

    #[test]
    fn concurrent_requests_start_only_one_service_stop() {
        let gate = std::sync::Arc::new(ExitGate::new());
        let requests: Vec<_> = (0..16)
            .map(|_| {
                let gate = gate.clone();
                std::thread::spawn(move || gate.begin())
            })
            .collect();
        assert_eq!(
            requests
                .into_iter()
                .map(|request| usize::from(request.join().unwrap()))
                .sum::<usize>(),
            1
        );
    }
}
