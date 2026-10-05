//! Keep one delayed renderer reply from turning into a desktop-wide reload storm.
#[derive(Default)]
pub struct WidgetHealth {
    missed: u8,
    reloads: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HealthAction {
    Retry,
    Reload(u8),
    GiveUp,
}

impl WidgetHealth {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn missed_reply(&mut self) -> HealthAction {
        self.missed += 1;
        if self.missed < 3 {
            return HealthAction::Retry;
        }
        self.missed = 0;
        if self.reloads >= 5 {
            return HealthAction::GiveUp;
        }
        self.reloads += 1;
        HealthAction::Reload(self.reloads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_late_reply_never_reloads_a_page() {
        let mut health = WidgetHealth::default();
        for _ in 0..30 {
            assert_eq!(health.missed_reply(), HealthAction::Retry);
            health.reset();
        }
    }

    #[test]
    fn genuine_failure_has_a_bounded_reload_budget() {
        let mut health = WidgetHealth::default();
        for reload in 1..=5 {
            assert_eq!(health.missed_reply(), HealthAction::Retry);
            assert_eq!(health.missed_reply(), HealthAction::Retry);
            assert_eq!(health.missed_reply(), HealthAction::Reload(reload));
        }
        assert_eq!(health.missed_reply(), HealthAction::Retry);
        assert_eq!(health.missed_reply(), HealthAction::Retry);
        assert_eq!(health.missed_reply(), HealthAction::GiveUp);
    }

    #[test]
    fn session_resume_and_healthy_reply_clear_old_failures() {
        let mut health = WidgetHealth::default();
        for _ in 0..8 {
            health.missed_reply();
        }
        health.reset();
        assert_eq!(health.missed_reply(), HealthAction::Retry);
        assert_eq!(health.missed_reply(), HealthAction::Retry);
        assert_eq!(health.missed_reply(), HealthAction::Reload(1));
    }
}
