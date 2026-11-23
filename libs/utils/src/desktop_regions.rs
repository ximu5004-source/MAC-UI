//! Convert DOM rectangles to native window regions without filling the gaps.
pub fn physical_regions(
    viewport: [f64; 2],
    client: [i32; 2],
    rects: &[[f64; 4]],
) -> Option<Vec<[i32; 4]>> {
    if viewport.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || client.iter().any(|v| *v <= 0)
        || rects.len() > 512
        || rects.iter().flatten().any(|v| !v.is_finite())
    {
        return None;
    }
    let sx = f64::from(client[0]) / viewport[0];
    let sy = f64::from(client[1]) / viewport[1];
    Some(
        rects
            .iter()
            .filter_map(|[left, top, right, bottom]| {
                let l = (left.clamp(0.0, viewport[0]) * sx).floor() as i32;
                let t = (top.clamp(0.0, viewport[1]) * sy).floor() as i32;
                let r = (right.clamp(0.0, viewport[0]) * sx).ceil() as i32;
                let b = (bottom.clamp(0.0, viewport[1]) * sy).ceil() as i32;
                (r > l && b > t).then_some([l, t, r.min(client[0]), b.min(client[1])])
            })
            .collect(),
    )
}

/// Native overlay HWND bounds are already physical screen coordinates. Clip
/// visible overlays to this window and translate the subtraction mask to the
/// window-relative coordinates required by SetWindowRgn (not monitor-local CSS).
pub fn overlay_exclusions(window: [i32; 4], overlays: &[([i32; 4], bool)]) -> Vec<[i32; 4]> {
    let [wx, wy, wr, wb] = window.map(i64::from);
    if wr <= wx || wb <= wy {
        return Vec::new();
    }
    let mut exclusions: Vec<_> = overlays
        .iter()
        .filter_map(|(rect, visible)| {
            if !visible {
                return None;
            }
            let [left, top, right, bottom] = rect.map(i64::from);
            let clipped = [left.max(wx), top.max(wy), right.min(wr), bottom.min(wb)];
            if clipped[2] <= clipped[0] || clipped[3] <= clipped[1] {
                return None;
            }
            Some([
                i32::try_from(clipped[0] - wx).ok()?,
                i32::try_from(clipped[1] - wy).ok()?,
                i32::try_from(clipped[2] - wx).ok()?,
                i32::try_from(clipped[3] - wy).ok()?,
            ])
        })
        .collect();
    // Tauri's HWND registry has no stable enumeration order. A stable mask lets
    // the native guard avoid repainting an unchanged desktop every timer tick.
    exclusions.sort_unstable();
    exclusions.dedup();
    exclusions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaps_are_not_converted_to_a_bounding_box() {
        let rects = [[10.0, 20.0, 30.0, 40.0], [100.0, 20.0, 120.0, 40.0]];
        assert_eq!(
            physical_regions([200.0, 100.0], [400, 200], &rects),
            Some(vec![[20, 40, 60, 80], [200, 40, 240, 80]])
        );
    }

    #[test]
    fn clips_to_client_at_non_uniform_dpi_and_rounds_outward() {
        assert_eq!(
            physical_regions(
                [100.0, 100.0],
                [125, 150],
                &[[-8.0, 10.2, 120.0, 50.1], [110.0, 0.0, 150.0, 20.0]]
            ),
            Some(vec![[0, 15, 125, 76]])
        );
    }

    #[test]
    fn empty_region_stays_empty_and_bad_data_is_rejected() {
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[]),
            Some(vec![])
        );
        assert_eq!(physical_regions([0.0, 100.0], [100, 100], &[]), None);
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[f64::NAN; 4]]),
            None
        );
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[0.0; 4]; 513]),
            None
        );
    }

    #[test]
    fn subtracts_visible_bars_on_two_screens_with_a_negative_desktop_origin() {
        let exclusions = overlay_exclusions(
            [-3840, 0, 3840, 2160],
            &[([0, 0, 3840, 40], true), ([-3840, 0, 0, 32], true)],
        );
        assert_eq!(exclusions, [[0, 0, 3840, 32], [3840, 0, 7680, 40]]);
        let remaining = |x, y| {
            x >= 0
                && x < 7680
                && y >= 0
                && y < 2160
                && !exclusions
                    .iter()
                    .any(|[l, t, r, b]| x >= *l && x < *r && y >= *t && y < *b)
        };
        assert!(!remaining(40, 16));
        assert!(!remaining(3880, 16));
        assert!(remaining(40, 32));
        assert!(remaining(3880, 40));
        assert!(remaining(1920, 1080));
    }

    #[test]
    fn hidden_bars_do_not_cut_the_desktop_and_resized_bars_change_the_mask() {
        let window = [-1920, -1080, 1920, 1080];
        assert!(overlay_exclusions(window, &[([-1920, -1080, 0, -1048], false)]).is_empty());
        assert_eq!(
            overlay_exclusions(window, &[([-1920, -1080, 0, -1048], true)]),
            [[0, 0, 1920, 32]]
        );
        assert_eq!(
            overlay_exclusions(window, &[([0, 1040, 1920, 1080], true)]),
            [[1920, 2120, 3840, 2160]]
        );
    }

    #[test]
    fn clips_overlay_edges_ignores_empty_or_remote_bars_and_deduplicates() {
        assert_eq!(
            overlay_exclusions(
                [100, 200, 1100, 800],
                &[
                    ([0, 190, 500, 230], true),
                    ([0, 190, 500, 230], true),
                    ([1100, 200, 1500, 250], true),
                    ([200, 250, 200, 270], true),
                ],
            ),
            [[0, 0, 400, 30]]
        );
    }
}
