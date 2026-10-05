//! Wallpaper placement and linear-light contrast, independent of Win32/COM.

pub fn luminance(rgb: [u8; 3]) -> f64 {
    let linear = rgb.map(|value| {
        let v = f64::from(value) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

/// Windows wallpaper placement: center=0, tile=1, stretch=2, fit=3, fill=4,
/// span=5 (the caller supplies the virtual-desktop rectangle for span).
/// None means the desktop background color shows through a letterbox.
pub fn source_point(
    image: [u32; 2],
    target: [f64; 2],
    point: [f64; 2],
    mode: i32,
) -> Option<[u32; 2]> {
    let [iw, ih] = image.map(f64::from);
    let [tw, th] = target;
    let [px, py] = point;
    if image.contains(&0)
        || !target.into_iter().all(|v| v.is_finite() && v > 0.0)
        || !point.into_iter().all(f64::is_finite)
        || px < 0.0
        || py < 0.0
        || px >= tw
        || py >= th
    {
        return None;
    }
    let [x, y] = match mode {
        1 => [px.rem_euclid(iw), py.rem_euclid(ih)],
        2 => [px * iw / tw, py * ih / th],
        _ => {
            let scale = match mode {
                0 => 1.0,
                3 => (tw / iw).min(th / ih),
                _ => (tw / iw).max(th / ih),
            };
            [
                (px - (tw - iw * scale) / 2.0) / scale,
                (py - (th - ih * scale) / 2.0) / scale,
            ]
        }
    };
    if x < 0.0 || y < 0.0 || x >= iw || y >= ih {
        return None;
    }
    Some([x.floor() as u32, y.floor() as u32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_samples_visible_crop_not_entire_wallpaper() {
        assert_eq!(
            source_point([4000, 1000], [2000.0, 1000.0], [0.0, 0.0], 4),
            Some([1000, 0])
        );
        assert_eq!(
            source_point([4000, 1000], [2000.0, 1000.0], [1999.0, 20.0], 4),
            Some([2999, 20])
        );
    }

    #[test]
    fn letterbox_and_center_return_background_color_area() {
        assert_eq!(
            source_point([1000, 1000], [2000.0, 1000.0], [0.0, 10.0], 3),
            None
        );
        assert_eq!(
            source_point([1000, 1000], [2000.0, 1000.0], [500.0, 10.0], 3),
            Some([0, 10])
        );
        assert_eq!(
            source_point([100, 100], [200.0, 200.0], [49.0, 50.0], 0),
            None
        );
    }

    #[test]
    fn tile_stretch_span_and_invalid_geometry() {
        assert_eq!(
            source_point([100, 50], [500.0, 300.0], [212.0, 123.0], 1),
            Some([12, 23])
        );
        assert_eq!(
            source_point([100, 50], [500.0, 300.0], [250.0, 150.0], 2),
            Some([50, 25])
        );
        assert_eq!(
            source_point([400, 100], [400.0, 100.0], [300.0, 10.0], 5),
            Some([300, 10])
        );
        assert_eq!(source_point([0, 100], [400.0, 100.0], [1.0, 10.0], 4), None);
        assert_eq!(
            source_point([100, 100], [f64::NAN, 100.0], [1.0, 10.0], 4),
            None
        );
    }

    #[test]
    fn luminance_uses_linear_light_not_byte_average() {
        assert_eq!(luminance([0, 0, 0]), 0.0);
        assert!((luminance([255, 255, 255]) - 1.0).abs() < 0.00001);
        assert!((luminance([128, 128, 128]) - 0.21586).abs() < 0.00001);
    }
}
