/// A label may be reused while its retired native window is still shutting down.
/// Only the bound HWND can change live state; early reports are buffered solely
/// during native creation and are checked again against the resulting HWND.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum StatusAdmission {
    Apply,
    Buffer,
    Reject,
}

pub(super) fn status_admission(
    bound_hwnd: Option<isize>,
    creating: bool,
    sender_hwnd: isize,
) -> StatusAdmission {
    match bound_hwnd {
        Some(hwnd) if hwnd == sender_hwnd => StatusAdmission::Apply,
        Some(_) => StatusAdmission::Reject,
        None if creating => StatusAdmission::Buffer,
        None => StatusAdmission::Reject,
    }
}

pub(super) struct EarlyWindowStatus<T>(Vec<(isize, T)>);

impl<T> Default for EarlyWindowStatus<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> EarlyWindowStatus<T> {
    pub(super) fn remember(&mut self, hwnd: isize, status: T) {
        if let Some((_, previous)) = self.0.iter_mut().find(|(sender, _)| *sender == hwnd) {
            *previous = status;
            return;
        }
        if self.0.len() == 8 {
            self.0.remove(0);
        }
        self.0.push((hwnd, status));
    }

    pub(super) fn take_for(&mut self, hwnd: isize) -> Option<T> {
        std::mem::take(&mut self.0)
            .into_iter()
            .find_map(|(sender, status)| (sender == hwnd).then_some(status))
    }
}

/// The caller supplies an atomic removal. Ready and trigger paths can race to
/// flush, but neither may consume the payload before native readiness is known.
pub(super) fn take_ready_trigger<T>(ready: bool, take: impl FnOnce() -> Option<T>) -> Option<T> {
    if ready { take() } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_ready_cannot_mark_an_unstarted_replacement_ready() {
        assert_eq!(status_admission(None, false, 10), StatusAdmission::Reject);
        assert_eq!(
            status_admission(Some(20), false, 10),
            StatusAdmission::Reject
        );
        assert_eq!(
            status_admission(Some(20), false, 20),
            StatusAdmission::Apply
        );
    }

    #[test]
    fn ready_before_creation_commit_is_consumed_only_by_its_native_window() {
        assert_eq!(status_admission(None, true, 20), StatusAdmission::Buffer);
        let mut pending = EarlyWindowStatus::default();
        pending.remember(20, "Ready");
        // A late report from the retired window cannot replace the new HWND's report.
        pending.remember(10, "Ready");
        assert_eq!(pending.take_for(20), Some("Ready"));
        assert_eq!(pending.take_for(10), None);
    }

    #[test]
    fn unrelated_early_ready_does_not_start_a_new_window_in_ready_state() {
        let mut pending = EarlyWindowStatus::default();
        pending.remember(10, "Ready");
        assert_eq!(pending.take_for(20), None);
    }

    #[test]
    fn ready_between_readiness_check_and_pending_insert_is_not_lost() {
        let pending = std::cell::Cell::new(None);
        // Trigger observes not-ready; Ready flush runs before the payload is inserted.
        assert_eq!(take_ready_trigger(false, || pending.take()), None);
        assert_eq!(take_ready_trigger(true, || pending.take()), None);
        pending.set(Some(7));
        // Trigger rechecks readiness after insert/start.
        assert_eq!(take_ready_trigger(true, || pending.take()), Some(7));
        assert_eq!(take_ready_trigger(true, || pending.take()), None);
    }

    #[test]
    fn simultaneous_ready_and_trigger_flush_deliver_only_once() {
        let pending = std::sync::Mutex::new(Some(7));
        std::thread::scope(|scope| {
            let first = scope.spawn(|| take_ready_trigger(true, || pending.lock().unwrap().take()));
            let second =
                scope.spawn(|| take_ready_trigger(true, || pending.lock().unwrap().take()));
            let results = [first.join().unwrap(), second.join().unwrap()];
            assert_eq!(results.iter().filter(|result| result.is_some()).count(), 1);
        });
    }
}
