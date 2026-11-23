//! A bounded, single-consumption cache; values never leave the owner's STA.
use std::time::{Duration, Instant};

pub(super) struct Prepared<K, V> {
    entries: Vec<(K, V, Instant, bool)>,
    ttl: Duration,
}

// Moving the pointer across another icon must not discard an already built
// menu. Background preparation has its own bounded share and cannot evict a
// menu already prepared for a real interaction. COM allocations are opaque:
// these limits bound the number of owned menus, not a guessed byte budget.
const CAPACITY: usize = 128;
const BACKGROUND_CAPACITY: usize = 96;

impl<K: PartialEq, V> Prepared<K, V> {
    pub(super) fn new(ttl: Duration) -> Self {
        Self {
            entries: Vec::new(),
            ttl,
        }
    }
    pub(super) fn matches(&self, key: &K, now: Instant) -> bool {
        self.entries.iter().any(|(saved, _, created, _)| {
            saved == key && now.saturating_duration_since(*created) < self.ttl
        })
    }
    pub(super) fn insert(&mut self, key: K, value: V, now: Instant) {
        self.entries.retain(|(saved, _, created, _)| {
            saved != &key && now.saturating_duration_since(*created) < self.ttl
        });
        if self.entries.len() == CAPACITY {
            let oldest_background = self
                .entries
                .iter()
                .position(|(_, _, _, background)| *background);
            self.entries.remove(oldest_background.unwrap_or(0));
        }
        self.entries.push((key, value, now, false));
    }
    pub(super) fn insert_background(&mut self, key: K, value: V, now: Instant) -> bool {
        self.entries
            .retain(|(_, _, created, _)| now.saturating_duration_since(*created) < self.ttl);
        // Never downgrade an interaction-prepared entry to background priority.
        if self.entries.iter().any(|(saved, _, _, _)| saved == &key) {
            return true;
        }
        let background_count = self
            .entries
            .iter()
            .filter(|(_, _, _, background)| *background)
            .count();
        if self.entries.len() == CAPACITY || background_count >= BACKGROUND_CAPACITY {
            let Some(index) = self
                .entries
                .iter()
                .position(|(_, _, _, background)| *background)
            else {
                return false;
            };
            self.entries.remove(index);
        }
        self.entries.push((key, value, now, true));
        true
    }
    pub(super) fn promote(&mut self, key: &K) {
        if let Some((_, _, _, background)) = self
            .entries
            .iter_mut()
            .find(|(saved, _, _, _)| saved == key)
        {
            *background = false;
        }
    }
    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
    pub(super) fn take(&mut self, key: &K, now: Instant) -> Option<V> {
        let index = self
            .entries
            .iter()
            .position(|(saved, _, _, _)| saved == key)?;
        let (_, value, created, _) = self.entries.remove(index);
        (now.saturating_duration_since(created) < self.ttl).then_some(value)
    }
}

/// New requests replace the one pending request instead of extending a queue.
pub(super) struct Latest<T> {
    pending: Option<T>,
    scheduled: bool,
}
impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self {
            pending: None,
            scheduled: false,
        }
    }
}
impl<T> Latest<T> {
    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
    /// True means the caller must post one wake-up to the owner thread.
    pub(super) fn replace(&mut self, value: T) -> bool {
        self.pending = Some(value);
        if self.scheduled {
            false
        } else {
            self.scheduled = true;
            true
        }
    }
    pub(super) fn take(&mut self) -> Option<T> {
        self.scheduled = false;
        self.pending.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_menu_is_consumed_once_and_rejects_expired_or_changed_keys() {
        let now = Instant::now();
        let mut cache = Prepared::new(Duration::from_millis(1800));
        cache.insert("selection-a", 10, now);
        assert_eq!(
            cache.take(&"selection-a", now + Duration::from_millis(30)),
            Some(10)
        );
        assert_eq!(
            cache.take(&"selection-a", now + Duration::from_millis(31)),
            None
        );
        cache.insert("selection-a", 11, now);
        assert_eq!(cache.take(&"selection-b", now), None);
        assert_eq!(cache.take(&"selection-a", now), Some(11));
        cache.insert("selection-a", 12, now);
        assert_eq!(
            cache.take(&"selection-a", now + Duration::from_millis(1800)),
            None
        );
    }

    #[test]
    fn moving_across_other_icons_retains_recent_menus_with_bounded_ownership() {
        let now = Instant::now();
        let mut cache = Prepared::new(Duration::from_secs(120));
        for key in 0..CAPACITY {
            cache.insert(key, key + 10, now);
        }
        assert!(cache.matches(&0, now));
        cache.insert(CAPACITY, 99, now);
        assert!(!cache.matches(&0, now));
        assert_eq!(cache.take(&2, now), Some(12));
        assert_eq!(cache.take(&2, now), None);
        assert_eq!(cache.take(&1, now), Some(11));
        assert!(cache.entries.len() < CAPACITY);
        cache.insert(3, 100, now);
        assert_eq!(cache.take(&3, now), Some(100));
        assert!(!cache.matches(&3, now));
    }

    #[test]
    fn hover_changes_keep_only_the_latest_request_and_one_pending_wakeup() {
        let mut queue = Latest::default();
        assert!(!queue.is_pending());
        assert!(queue.replace(1));
        assert!(queue.is_pending());
        assert!(!queue.replace(2));
        assert!(!queue.replace(3));
        assert_eq!(queue.take(), Some(3));
        assert!(!queue.is_pending());
        assert_eq!(queue.take(), None);
        assert!(queue.replace(4));
        assert_eq!(queue.take(), Some(4));
    }

    #[test]
    fn background_share_is_bounded_and_never_evicts_interaction_menus() {
        let now = Instant::now();
        let mut cache = Prepared::new(Duration::from_secs(120));
        for key in 0..32 {
            cache.insert(key, key, now);
        }
        for key in 100..100 + BACKGROUND_CAPACITY {
            assert!(cache.insert_background(key, key, now));
        }
        assert_eq!(cache.entries.len(), CAPACITY);
        assert!(cache.insert_background(999, 999, now));
        assert_eq!(cache.entries.len(), CAPACITY);
        assert!(!cache.matches(&100, now));
        for key in 0..32 {
            assert!(
                cache.matches(&key, now),
                "background evicted an interaction entry"
            );
        }
        cache.insert(500, 500, now);
        assert_eq!(cache.entries.len(), CAPACITY);
        assert!(cache.matches(&0, now));
        assert!(cache.matches(&500, now));
    }

    #[test]
    fn idle_fill_cannot_evict_a_promoted_or_full_interaction_cache() {
        let now = Instant::now();
        let mut cache = Prepared::new(Duration::from_secs(120));
        assert!(cache.insert_background(0, 0, now));
        cache.promote(&0);
        for key in 1..=BACKGROUND_CAPACITY + 1 {
            assert!(cache.insert_background(key, key, now));
        }
        assert!(cache.matches(&0, now));
        assert_eq!(cache.entries.len(), BACKGROUND_CAPACITY + 1);

        let mut full = Prepared::new(Duration::from_secs(120));
        for key in 0..CAPACITY {
            full.insert(key, key, now);
        }
        assert!(!full.insert_background(999, 999, now));
        assert_eq!(full.entries.len(), CAPACITY);
        assert!(full.matches(&0, now));
        assert!(full.insert_background(999, 999, now + Duration::from_secs(120)));
        assert_eq!(full.entries.len(), 1);
    }

    #[test]
    fn unchanged_menu_survives_stationary_pointer_but_has_a_hard_lifetime_bound() {
        let now = Instant::now();
        let mut cache = Prepared::new(Duration::from_secs(120));
        cache.insert("same-valid-key", 10, now);
        assert!(cache.matches(&"same-valid-key", now + Duration::from_secs(30)));
        assert_eq!(
            cache.take(&"same-valid-key", now + Duration::from_secs(90)),
            Some(10)
        );
        cache.insert("same-valid-key", 11, now);
        assert_eq!(
            cache.take(&"same-valid-key", now + Duration::from_secs(120)),
            None
        );
        cache.insert("same-valid-key", 12, now);
        cache.clear();
        assert!(!cache.matches(&"same-valid-key", now));
    }
}
