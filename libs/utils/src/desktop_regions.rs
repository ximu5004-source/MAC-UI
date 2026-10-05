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
}
