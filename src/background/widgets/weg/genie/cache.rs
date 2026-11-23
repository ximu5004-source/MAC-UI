use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use macui_core::rect::Rect;
use windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT;

const MAX_BYTES: usize = 64 * 1024 * 1024;
// 1MP textures still share a strict 64MiB budget, including live samples for
// windows minimized through shortcuts or another application's system command.
const MAX_ENTRIES: usize = 16;
const MAX_AGE: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub(super) struct Snapshot {
    pub hwnd: isize,
    pub pid: u32,
    pub class: String,
    pub source: Rect,
    pub placement: WINDOWPLACEMENT,
    /// Premultiplied BGRA capture texture, independent of physical window size.
    pub texture_width: i32,
    pub texture_height: i32,
    pub pixels: Arc<[u8]>,
    pub captured_at: Instant,
}

#[derive(Default)]
pub(super) struct SnapshotCache(VecDeque<Snapshot>);

impl SnapshotCache {
    pub fn insert(&mut self, snapshot: Snapshot) {
        self.0
            .retain(|entry| entry.hwnd != snapshot.hwnd && entry.captured_at.elapsed() < MAX_AGE);
        if snapshot.pixels.len() > MAX_BYTES {
            return;
        }
        self.0.push_back(snapshot);
        while self.0.len() > MAX_ENTRIES
            || self.0.iter().map(|entry| entry.pixels.len()).sum::<usize>() > MAX_BYTES
        {
            self.0.pop_front();
        }
    }

    pub fn get(&mut self, hwnd: isize, pid: u32, class: &str) -> Option<Snapshot> {
        self.0.retain(|entry| entry.captured_at.elapsed() < MAX_AGE);
        self.0
            .iter()
            .find(|entry| entry.hwnd == hwnd && entry.pid == pid && entry.class == class)
            .cloned()
    }

    pub fn remove(&mut self, hwnd: isize) {
        self.0.retain(|entry| entry.hwnd != hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(hwnd: isize, size: usize) -> Snapshot {
        Snapshot {
            hwnd,
            pid: 7,
            class: "owned-test".into(),
            source: Rect::default(),
            placement: WINDOWPLACEMENT::default(),
            texture_width: 1,
            texture_height: (size / 4) as i32,
            pixels: vec![0; size].into(),
            captured_at: Instant::now(),
        }
    }

    #[test]
    fn cache_is_bounded_replaces_hwnd_and_rejects_reused_identity() {
        let mut cache = SnapshotCache::default();
        for hwnd in 1..=18 {
            cache.insert(snapshot(hwnd, 1024));
        }
        assert!(cache.get(1, 7, "owned-test").is_none());
        assert!(cache.get(18, 8, "owned-test").is_none());
        assert!(cache.get(18, 7, "different-class").is_none());
        cache.insert(snapshot(18, 2048));
        assert_eq!(cache.get(18, 7, "owned-test").unwrap().pixels.len(), 2048);
        assert_eq!(cache.0.len(), MAX_ENTRIES);
        cache.remove(18);
        assert!(cache.get(18, 7, "owned-test").is_none());
    }

    #[test]
    fn stale_snapshots_are_discarded() {
        let mut cache = SnapshotCache::default();
        let mut stale = snapshot(1, 4);
        stale.captured_at = Instant::now() - MAX_AGE - Duration::from_secs(1);
        cache.insert(stale);
        assert!(cache.get(1, 7, "owned-test").is_none());
    }

    #[test]
    fn byte_budget_limits_live_samples_even_below_the_entry_limit() {
        let mut cache = SnapshotCache::default();
        for hwnd in 1..=4 {
            cache.insert(snapshot(hwnd, 20 * 1024 * 1024));
        }
        assert_eq!(cache.0.len(), 3);
        assert!(cache.get(1, 7, "owned-test").is_none());
        assert!(
            cache
                .0
                .iter()
                .map(|snapshot| snapshot.pixels.len())
                .sum::<usize>()
                <= MAX_BYTES
        );
    }
}
