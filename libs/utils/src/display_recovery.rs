//! Pure display-recovery decisions shared with the native monitor manager.

/// A single valid enumeration is not enough after DPMS, resume or a hotplug: Windows
/// can briefly report a fallback resolution even when the monitor count is unchanged.
pub struct StableDisplaySnapshot<T> {
    previous: Option<Vec<T>>,
    consecutive: usize,
    required: usize,
}

impl<T: PartialEq + Clone> StableDisplaySnapshot<T> {
    pub fn new(required: usize) -> Self {
        Self {
            previous: None,
            consecutive: 0,
            required: required.max(2),
        }
    }

    /// Invalid/empty samples break the streak, and must never replace the cached
    /// desktop with an empty topology (all monitors can disappear while asleep).
    pub fn observe(&mut self, sample: Option<Vec<T>>) -> Option<Vec<T>> {
        let Some(sample) = sample.filter(|items| !items.is_empty()) else {
            self.previous = None;
            self.consecutive = 0;
            return None;
        };
        if self.previous.as_ref() == Some(&sample) {
            self.consecutive += 1;
        } else {
            self.previous = Some(sample.clone());
            self.consecutive = 1;
        }
        (self.consecutive >= self.required).then_some(sample)
    }
}

pub fn should_refresh_display_layout(topology_changed: bool, recovering: bool) -> bool {
    topology_changed || recovering
}

/// WebView2 bounds use parent-client physical pixels. Never apply monitor DPI a
/// second time, and do not overwrite a surviving viewport with a minimized size.
pub fn physical_client_viewport(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Option<(i32, i32)> {
    let width = right.checked_sub(left)?;
    let height = bottom.checked_sub(top)?;
    (width > 0 && height > 0).then_some((width, height))
}

/// DWM blur-behind transparency is only valid for the app's redirected top-level
/// windows. In particular, never apply it to a wallpaper child or its Explorer host.
pub fn should_restore_native_transparency(is_child: bool, no_redirection_bitmap: bool) -> bool {
    !is_child && !no_redirection_bitmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_half_resolution_is_not_published() {
        let mut stable = StableDisplaySnapshot::new(3);
        assert_eq!(stable.observe(Some(vec![("main", 960, 540)])), None);
        assert_eq!(stable.observe(Some(vec![("main", 1920, 1080)])), None);
        assert_eq!(stable.observe(Some(vec![("main", 1920, 1080)])), None);
        assert_eq!(
            stable.observe(Some(vec![("main", 1920, 1080)])),
            Some(vec![("main", 1920, 1080)])
        );
    }

    #[test]
    fn failed_or_empty_enumeration_never_removes_all_monitors() {
        let mut stable = StableDisplaySnapshot::new(2);
        assert_eq!(stable.observe(Some(vec![1])), None);
        assert_eq!(stable.observe(None), None);
        assert_eq!(stable.observe(Some(vec![1])), None);
        assert_eq!(stable.observe(Some(vec![])), None);
        assert_eq!(stable.observe(Some(vec![])), None);
        assert_eq!(stable.observe(Some(vec![1])), None);
        assert_eq!(stable.observe(Some(vec![1])), Some(vec![1]));
    }

    #[test]
    fn equal_counts_do_not_mask_identity_or_scale_changes() {
        let mut stable = StableDisplaySnapshot::new(2);
        assert_eq!(stable.observe(Some(vec![("first", 100)])), None);
        assert_eq!(stable.observe(Some(vec![("second", 100)])), None);
        assert_eq!(stable.observe(Some(vec![("second", 150)])), None);
        assert_eq!(
            stable.observe(Some(vec![("second", 150)])),
            Some(vec![("second", 150)])
        );
    }

    #[test]
    fn wake_reapplies_geometry_even_when_topology_did_not_change() {
        assert!(should_refresh_display_layout(false, true));
        assert!(should_refresh_display_layout(true, false));
        assert!(!should_refresh_display_layout(false, false));
    }

    #[test]
    fn controller_bounds_keep_client_physical_pixels_without_dpi_scaling() {
        assert_eq!(
            physical_client_viewport(0, 0, 3840, 2160),
            Some((3840, 2160))
        );
        assert_eq!(physical_client_viewport(0, 0, 2560, 56), Some((2560, 56)));
        assert_eq!(physical_client_viewport(0, 0, 640, 92), Some((640, 92)));
    }

    #[test]
    fn invalid_or_minimized_clients_do_not_erase_the_controller_viewport() {
        assert_eq!(physical_client_viewport(0, 0, 0, 0), None);
        assert_eq!(physical_client_viewport(0, 0, -1, 1080), None);
        assert_eq!(physical_client_viewport(i32::MIN, 0, i32::MAX, 1080), None);
    }

    #[test]
    fn redirected_top_level_overlay_can_restore_transparency() {
        assert!(should_restore_native_transparency(false, false));
    }

    #[test]
    fn wallpaper_children_and_nonredirected_windows_are_not_reconfigured() {
        assert!(!should_restore_native_transparency(true, false));
        assert!(!should_restore_native_transparency(false, true));
        assert!(!should_restore_native_transparency(true, true));
    }
}
