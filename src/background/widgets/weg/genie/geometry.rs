//! Pure geometry for a strip-mesh Genie transition, independent of Win32.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
}

pub fn direction(source: Bounds, target: Bounds) -> Direction {
    let dx = (target.x + target.width / 2.0 - source.x - source.width / 2.0) / source.width;
    let dy = (target.y + target.height / 2.0 - source.y - source.height / 2.0) / source.height;
    if dx.abs() > dy.abs() {
        if dx < 0.0 {
            Direction::Left
        } else {
            Direction::Right
        }
    } else if dy < 0.0 {
        Direction::Top
    } else {
        Direction::Bottom
    }
}

fn smooth(value: f64) -> f64 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

/// The neck reaches the Dock before the rest of the window is swallowed.
/// This boundary is shared by both axes, so there is no discontinuous jump
/// between the stretching and swallowing phases.
const NECK_PHASE_END: f64 = 0.42;

/// Each strip first stretches into a funnel anchored at the destination, then
/// gets swallowed into that destination. The far edge stays at the original
/// window during the first phase, while the near edge reaches the Dock. During
/// the second phase the near edge stays at the Dock and the far edge catches up.
/// Both cross-axis width and center vary by strip: this is not an affine scale.
pub fn strip(
    source: Bounds,
    target: Bounds,
    side: Direction,
    t: f64,
    index: usize,
    count: usize,
) -> Bounds {
    assert!(count > 0 && index < count);
    let horizontal = matches!(side, Direction::Left | Direction::Right);
    let reverse = matches!(side, Direction::Top | Direction::Left);
    let begin = index as f64 / count as f64;
    let end = (index + 1) as f64 / count as f64;
    let u = (begin + end) / 2.0;
    let near = if reverse { 1.0 - u } else { u };
    let stretch = smooth(t / NECK_PHASE_END);
    let bend = stretch * smooth(near);
    let collapse = smooth((t - NECK_PHASE_END) / (1.0 - NECK_PHASE_END));
    let narrow = bend + (1.0 - bend) * collapse;
    let src_center = if horizontal {
        source.y + source.height / 2.0
    } else {
        source.x + source.width / 2.0
    };
    let dst_center = if horizontal {
        target.y + target.height / 2.0
    } else {
        target.x + target.width / 2.0
    };
    let width = mix(
        if horizontal {
            source.height
        } else {
            source.width
        },
        if horizontal {
            target.height
        } else {
            target.width
        },
        narrow,
    );
    let center = mix(src_center, dst_center, narrow);
    let src_axis = if horizontal { source.x } else { source.y };
    let dst_axis = if horizontal { target.x } else { target.y };
    let src_length = if horizontal {
        source.width
    } else {
        source.height
    };
    let dst_length = if horizontal {
        target.width
    } else {
        target.height
    };
    // Interpolate the two *boundary* positions, not every strip's uniform
    // translation. Stretching just the near boundary creates the classic long
    // neck; swallowing just the far boundary keeps that neck fixed to its icon.
    let low = mix(src_axis, dst_axis, if reverse { stretch } else { collapse });
    let high = mix(
        src_axis + src_length,
        dst_axis + dst_length,
        if reverse { collapse } else { stretch },
    );
    let a = mix(low, high, begin);
    let b = mix(low, high, end);
    if horizontal {
        Bounds {
            x: a,
            y: center - width / 2.0,
            width: b - a,
            height: width,
        }
    } else {
        Bounds {
            x: center - width / 2.0,
            y: a,
            width,
            height: b - a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: Bounds = Bounds {
        x: -1400.0,
        y: 50.0,
        width: 1000.0,
        height: 700.0,
    };
    const TARGET: Bounds = Bounds {
        x: -650.0,
        y: 1000.0,
        width: 50.0,
        height: 50.0,
    };

    fn target_for(side: Direction) -> Bounds {
        match side {
            Direction::Top => Bounds {
                y: -400.0,
                ..TARGET
            },
            Direction::Bottom => TARGET,
            Direction::Left => Bounds {
                x: -2000.0,
                y: 300.0,
                ..TARGET
            },
            Direction::Right => Bounds {
                x: 100.0,
                y: 300.0,
                ..TARGET
            },
        }
    }

    fn mesh_axis(side: Direction, t: f64) -> (f64, f64) {
        let target = target_for(side);
        let first = strip(SOURCE, target, side, t, 0, 96);
        let last = strip(SOURCE, target, side, t, 95, 96);
        if matches!(side, Direction::Left | Direction::Right) {
            (first.x, last.x + last.width)
        } else {
            (first.y, last.y + last.height)
        }
    }

    #[test]
    fn identity_at_start_and_destination_at_end() {
        for side in [
            Direction::Top,
            Direction::Bottom,
            Direction::Left,
            Direction::Right,
        ] {
            let horizontal = matches!(side, Direction::Left | Direction::Right);
            for index in 0..96 {
                for (t, bounds) in [(0.0, SOURCE), (1.0, TARGET)] {
                    let actual = strip(SOURCE, TARGET, side, t, index, 96);
                    let expected = if horizontal {
                        Bounds {
                            x: bounds.x + bounds.width * index as f64 / 96.0,
                            width: bounds.width / 96.0,
                            ..bounds
                        }
                    } else {
                        Bounds {
                            y: bounds.y + bounds.height * index as f64 / 96.0,
                            height: bounds.height / 96.0,
                            ..bounds
                        }
                    };
                    assert!((actual.x - expected.x).abs() < 1e-9);
                    assert!((actual.y - expected.y).abs() < 1e-9);
                    assert!((actual.width - expected.width).abs() < 1e-9);
                    assert!((actual.height - expected.height).abs() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn neck_forms_before_far_edge_collapses() {
        let far = strip(SOURCE, TARGET, Direction::Bottom, 0.3, 0, 96);
        let near = strip(SOURCE, TARGET, Direction::Bottom, 0.3, 95, 96);
        assert!(near.width < far.width * 0.6);
        assert!(far.width > SOURCE.width * 0.95);
    }

    #[test]
    fn stretches_to_dock_with_original_far_edge_before_swallowing() {
        for side in [
            Direction::Top,
            Direction::Bottom,
            Direction::Left,
            Direction::Right,
        ] {
            let target = target_for(side);
            let horizontal = matches!(side, Direction::Left | Direction::Right);
            let reverse = matches!(side, Direction::Top | Direction::Left);
            let source_low = if horizontal { SOURCE.x } else { SOURCE.y };
            let source_length = if horizontal {
                SOURCE.width
            } else {
                SOURCE.height
            };
            let target_low = if horizontal { target.x } else { target.y };
            let target_length = if horizontal {
                target.width
            } else {
                target.height
            };
            let (low, high) = mesh_axis(side, NECK_PHASE_END);
            let expected_low = if reverse { target_low } else { source_low };
            let expected_high = if reverse {
                source_low + source_length
            } else {
                target_low + target_length
            };
            assert!((low - expected_low).abs() < 1e-9);
            assert!((high - expected_high).abs() < 1e-9);
            assert!(
                high - low > source_length,
                "the funnel must visibly stretch before shrinking"
            );
        }
    }

    #[test]
    fn neck_remains_anchored_to_dock_through_swallowing() {
        for side in [
            Direction::Top,
            Direction::Bottom,
            Direction::Left,
            Direction::Right,
        ] {
            let target = target_for(side);
            let horizontal = matches!(side, Direction::Left | Direction::Right);
            let reverse = matches!(side, Direction::Top | Direction::Left);
            let target_low = if horizontal { target.x } else { target.y };
            let target_length = if horizontal {
                target.width
            } else {
                target.height
            };
            for step in 0..=58 {
                let t = NECK_PHASE_END + step as f64 / 100.0;
                let (low, high) = mesh_axis(side, t);
                let near = if reverse { low } else { high };
                let expected = if reverse {
                    target_low
                } else {
                    target_low + target_length
                };
                assert!((near - expected).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn stage_boundary_is_continuous_and_out_of_range_time_clamps() {
        for side in [
            Direction::Top,
            Direction::Bottom,
            Direction::Left,
            Direction::Right,
        ] {
            let target = target_for(side);
            for index in 0..96 {
                let before = strip(SOURCE, target, side, NECK_PHASE_END - 1e-7, index, 96);
                let after = strip(SOURCE, target, side, NECK_PHASE_END + 1e-7, index, 96);
                for delta in [
                    before.x - after.x,
                    before.y - after.y,
                    before.width - after.width,
                    before.height - after.height,
                ] {
                    assert!(delta.abs() < 1e-8, "phase boundary jumps by {delta}");
                }
                assert_eq!(
                    strip(SOURCE, target, side, -1.0, index, 96),
                    strip(SOURCE, target, side, 0.0, index, 96)
                );
                assert_eq!(
                    strip(SOURCE, target, side, 2.0, index, 96),
                    strip(SOURCE, target, side, 1.0, index, 96)
                );
            }
        }
    }

    #[test]
    fn positive_finite_mesh_with_contiguous_axis_for_all_directions() {
        for side in [
            Direction::Top,
            Direction::Bottom,
            Direction::Left,
            Direction::Right,
        ] {
            let target = target_for(side);
            for step in 0..=100 {
                let mut previous: Option<Bounds> = None;
                for index in 0..96 {
                    let bounds = strip(SOURCE, target, side, step as f64 / 100.0, index, 96);
                    assert!(bounds.width > 0.0 && bounds.height > 0.0);
                    assert!(
                        [bounds.x, bounds.y, bounds.width, bounds.height]
                            .iter()
                            .all(|v| v.is_finite())
                    );
                    if let Some(last) = previous {
                        if matches!(side, Direction::Left | Direction::Right) {
                            assert!((last.x + last.width - bounds.x).abs() < 1e-9);
                        } else {
                            assert!((last.y + last.height - bounds.y).abs() < 1e-9);
                        }
                    }
                    previous = Some(bounds);
                }
            }
        }
    }

    #[test]
    fn targets_on_each_screen_edge_choose_matching_axis() {
        assert_eq!(direction(SOURCE, TARGET), Direction::Bottom);
        assert_eq!(
            direction(
                SOURCE,
                Bounds {
                    y: -400.0,
                    ..TARGET
                }
            ),
            Direction::Top
        );
        assert_eq!(
            direction(
                SOURCE,
                Bounds {
                    x: -2000.0,
                    y: 300.0,
                    ..TARGET
                }
            ),
            Direction::Left
        );
        assert_eq!(
            direction(
                SOURCE,
                Bounds {
                    x: 100.0,
                    y: 300.0,
                    ..TARGET
                }
            ),
            Direction::Right
        );
    }
}
