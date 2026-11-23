//! Pure reconnect policy. No Windows handles, UI or native calls in this module.
use std::time::Duration;

pub(crate) const PROBE_INTERVAL: Duration = Duration::from_secs(2);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Decision {
    pub detach: bool,
    pub attach: bool,
}

pub(crate) struct ReconnectPolicy<K> {
    target: Option<K>,
    attached: bool,
    failures: u32,
    retry_at: Duration,
}

impl<K: PartialEq> ReconnectPolicy<K> {
    pub fn new() -> Self {
        Self {
            target: None,
            attached: false,
            failures: 0,
            retry_at: Duration::ZERO,
        }
    }

    pub fn observe(&mut self, target: Option<K>, now: Duration) -> Decision {
        let mut detach = false;
        if self.target != target {
            detach = self.attached;
            self.target = target;
            self.attached = false;
            self.failures = 0;
            self.retry_at = now;
        }
        Decision {
            detach,
            attach: self.target.is_some() && !self.attached && now >= self.retry_at,
        }
    }

    pub fn connected(&mut self) {
        self.attached = true;
        self.failures = 0;
    }

    pub fn failed(&mut self, now: Duration) -> Duration {
        self.attached = false;
        self.failures = self.failures.saturating_add(1);
        // Circuit cooldown is bounded, not permanent: a transient busy shell
        // must recover even when its PID/thread identity never changes.
        let delay = Duration::from_secs(1u64 << self.failures.min(6)).min(MAX_RETRY_DELAY);
        self.retry_at = now.saturating_add(delay);
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq)]
    struct Key {
        pid: u32,
        created: u64,
        thread: u32,
    }
    fn key() -> Key {
        Key {
            pid: 10,
            created: 100,
            thread: 20,
        }
    }

    #[test]
    fn unavailable_shell_does_not_attach_and_healthy_identity_never_duplicates() {
        let mut state = ReconnectPolicy::new();
        assert_eq!(
            state.observe(None, Duration::ZERO),
            Decision {
                detach: false,
                attach: false
            }
        );
        assert!(state.observe(Some(key()), Duration::ZERO).attach);
        state.connected();
        assert_eq!(
            state.observe(Some(key()), Duration::from_secs(30)),
            Decision {
                detach: false,
                attach: false
            }
        );
    }

    #[test]
    fn replacement_or_pid_reuse_detaches_before_a_new_attach() {
        for replacement in [
            Key { pid: 11, ..key() },
            Key {
                created: 101,
                ..key()
            },
            Key {
                thread: 21,
                ..key()
            },
        ] {
            let mut state = ReconnectPolicy::new();
            state.observe(Some(key()), Duration::ZERO);
            state.connected();
            assert_eq!(
                state.observe(Some(replacement), PROBE_INTERVAL),
                Decision {
                    detach: true,
                    attach: true
                }
            );
        }
    }

    #[test]
    fn missing_shell_detaches_once_and_reappearance_can_reconnect() {
        let mut state = ReconnectPolicy::new();
        state.observe(Some(key()), Duration::ZERO);
        state.connected();
        assert_eq!(
            state.observe(None, PROBE_INTERVAL),
            Decision {
                detach: true,
                attach: false
            }
        );
        assert!(!state.observe(None, PROBE_INTERVAL).detach);
        assert!(state.observe(Some(key()), PROBE_INTERVAL).attach);
    }

    #[test]
    fn failures_back_off_with_a_finite_cooldown_and_can_recover() {
        let mut state = ReconnectPolicy::new();
        let mut now = Duration::ZERO;
        for expected in [2, 4, 8, 16, 32, 60, 60, 60, 60] {
            assert!(state.observe(Some(key()), now).attach);
            let delay = state.failed(now);
            assert_eq!(delay, Duration::from_secs(expected));
            assert!(
                !state
                    .observe(Some(key()), now + delay - Duration::from_millis(1))
                    .attach
            );
            now += delay;
        }
        assert!(state.observe(Some(key()), now).attach);
        state.connected();
        assert!(!state.observe(Some(key()), now + MAX_RETRY_DELAY).attach);
    }

    #[test]
    fn new_explorer_resets_old_circuit_cooldown() {
        let mut state = ReconnectPolicy::new();
        state.observe(Some(key()), Duration::ZERO);
        state.failed(Duration::ZERO);
        assert!(
            state
                .observe(
                    Some(Key {
                        created: 101,
                        ..key()
                    }),
                    Duration::ZERO
                )
                .attach
        );
        assert_eq!(state.failed(Duration::ZERO), PROBE_INTERVAL);
    }
}
