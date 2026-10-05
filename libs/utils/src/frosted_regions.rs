//! Geometry for native, live-backdrop surfaces. These are paint regions, not
//! input regions: a glass surface must never expand the desktop's hit target.

#[derive(Debug, Clone, PartialEq)]
pub struct FrostedRegion {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub radius_x: f32,
    pub radius_y: f32,
}

/// Preserve the original rounded shape when it straddles the viewport. The
/// native composition target clips at the HWND boundary; clamping the origin
/// here would manufacture a new rounded edge on a partially offscreen panel.
pub fn physical_regions(
    viewport: [f64; 2],
    client: [i32; 2],
    rects: &[[f64; 5]],
) -> Option<Vec<FrostedRegion>> {
    if viewport.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || client.iter().any(|v| *v <= 0)
        || rects.len() > 512
        || rects.iter().flatten().any(|v| !v.is_finite())
        || rects.iter().any(|r| r[4] < 0.0)
    {
        return None;
    }
    let sx = f64::from(client[0]) / viewport[0];
    let sy = f64::from(client[1]) / viewport[1];
    let mut regions = Vec::new();
    for &[left, top, right, bottom, radius] in rects {
        let width = right - left;
        let height = bottom - top;
        if width <= 0.0
            || height <= 0.0
            || right <= 0.0
            || bottom <= 0.0
            || left >= viewport[0]
            || top >= viewport[1]
        {
            continue;
        }
        let radius = radius.min(width / 2.0).min(height / 2.0);
        let values = [
            left * sx,
            top * sy,
            width * sx,
            height * sy,
            radius * sx,
            radius * sy,
        ];
        // A malformed caller must not submit infinities/huge allocations to DWM.
        if values
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return None;
        }
        regions.push(FrostedRegion {
            left: values[0] as f32,
            top: values[1] as f32,
            width: values[2] as f32,
            height: values[3] as f32,
            radius_x: values[4] as f32,
            radius_y: values[5] as f32,
        });
    }
    Some(regions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_separate_surfaces_and_exact_radii_at_dpi() {
        let output = physical_regions(
            [200.0, 100.0],
            [400, 150],
            &[
                [10.0, 10.0, 80.0, 50.0, 22.0],
                [100.0, 10.0, 190.0, 90.0, 28.0],
            ],
        )
        .unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(
            output[0],
            FrostedRegion {
                left: 20.0,
                top: 15.0,
                width: 140.0,
                height: 60.0,
                radius_x: 40.0,
                radius_y: 30.0,
            }
        );
        assert_eq!(output[1].left, 200.0);
        assert_eq!(output[1].radius_x, 56.0);
    }

    #[test]
    fn offscreen_shape_is_not_replaced_by_clipped_new_corners() {
        let output = physical_regions(
            [100.0, 100.0],
            [100, 100],
            &[
                [-10.0, -20.0, 80.0, 90.0, 22.0],
                [100.0, 0.0, 140.0, 40.0, 8.0],
            ],
        )
        .unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].left, -10.0);
        assert_eq!(output[0].top, -20.0);
        assert_eq!(output[0].width, 90.0);
        assert_eq!(output[0].radius_x, 22.0);
    }

    #[test]
    fn hiding_a_surface_never_fills_the_window() {
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[]),
            Some(vec![])
        );
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[1.0, 1.0, 1.0, 1.0, 0.0]]),
            Some(vec![])
        );
    }

    #[test]
    fn rejects_invalid_geometry_and_excessive_regions() {
        assert_eq!(physical_regions([0.0, 100.0], [100, 100], &[]), None);
        assert_eq!(physical_regions([100.0, 100.0], [0, 100], &[]), None);
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[f64::NAN; 5]]),
            None
        );
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[0.0, 0.0, 20.0, 20.0, -1.0]]),
            None
        );
        assert_eq!(
            physical_regions([100.0, 100.0], [100, 100], &[[0.0; 5]; 513]),
            None
        );
        assert_eq!(
            physical_regions(
                [100.0, 100.0],
                [100, 100],
                &[[-f64::MAX, 0.0, f64::MAX, 50.0, 0.0]]
            ),
            None
        );
    }
}
