//! Pure bounds/return-value policy for native icon bitmap decoding.
const MAX_BITMAP_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BitmapLayout {
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

pub(crate) fn checked_layout(width: i32, height: i32) -> Option<BitmapLayout> {
    let width = u32::try_from(width).ok().filter(|value| *value > 0)?;
    let height = u32::try_from(height.checked_abs()?)
        .ok()
        .filter(|value| *value > 0)?;
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= MAX_BITMAP_BYTES).then_some(BitmapLayout {
        width,
        height,
        bytes,
    })
}

pub(crate) fn complete_scanlines(actual: i32, expected: u32) -> bool {
    expected > 0 && u32::try_from(actual) == Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_dimensions_and_allocation_are_positive_and_bounded() {
        assert_eq!(
            checked_layout(2, -2),
            Some(BitmapLayout {
                width: 2,
                height: 2,
                bytes: 16
            })
        );
        for (width, height) in [(0, 2), (2, 0), (-1, 2), (2, i32::MIN), (i32::MAX, i32::MAX)] {
            assert!(checked_layout(width, height).is_none());
        }
        assert!(checked_layout(4096, 4096).is_some());
        assert!(checked_layout(4097, 4096).is_none());
    }

    #[test]
    fn partial_or_failed_scanline_reads_are_not_images() {
        assert!(complete_scanlines(2, 2));
        for actual in [-1, 0, 1, 3] {
            assert!(!complete_scanlines(actual, 2));
        }
        assert!(!complete_scanlines(0, 0));
    }
}
