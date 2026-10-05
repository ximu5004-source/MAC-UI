//! Desktop placement from an actual top-to-bottom sibling z-order snapshot.
#[derive(Clone, Copy, Debug)]
pub struct DesktopWindow {
    pub id: isize,
    pub icon_host: bool,
    /// A verified Explorer desktop/wallpaper surface, never an ordinary app.
    pub desktop_surface: bool,
    pub topmost: bool,
    /// Currently drawable: excludes hidden, minimized, cloaked and empty helpers.
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAnchor {
    After(isize),
    NonTopmost,
    Bottom,
}

pub fn desktop_anchor(windows: &[DesktopWindow], own: isize) -> DesktopAnchor {
    let Some(host) = windows.iter().position(|window| {
        window.icon_host && window.visible && !window.topmost && window.id != own
    }) else {
        // Explorer may be restarting. Never fall back to HWND_TOP.
        return DesktopAnchor::Bottom;
    };
    // Explorer can temporarily raise its desktop. Following it would put our
    // interactive fullscreen surface over applications that are still below it.
    if windows[host + 1..]
        .iter()
        .any(|window| is_application(window, own))
    {
        return DesktopAnchor::Bottom;
    }
    match windows[..host]
        .iter()
        .rev()
        .find(|window| is_application(window, own))
    {
        Some(window) if !window.topmost => DesktopAnchor::After(window.id),
        // Inserting after a topmost window would promote our fullscreen surface.
        _ => DesktopAnchor::NonTopmost,
    }
}

fn is_application(window: &DesktopWindow, own: isize) -> bool {
    window.id != own && window.visible && !window.desktop_surface && !window.topmost
}

pub fn desktop_applications_below(windows: &[DesktopWindow], own: isize) -> usize {
    windows
        .iter()
        .position(|window| window.id == own)
        .map_or(0, |index| {
            windows[index + 1..]
                .iter()
                .filter(|window| is_application(window, own))
                .count()
        })
}

/// Ignore invisible helpers between real surfaces instead of continuously
/// restacking a desktop whose effective order is already correct.
pub fn desktop_needs_restack(windows: &[DesktopWindow], own: isize) -> bool {
    let Some(index) = windows.iter().position(|window| window.id == own) else {
        return false;
    };
    if windows[index].topmost || desktop_applications_below(windows, own) != 0 {
        return true;
    }
    match desktop_anchor(windows, own) {
        DesktopAnchor::Bottom => false,
        DesktopAnchor::After(_) | DesktopAnchor::NonTopmost => {
            // The applications are already above us; only visibility above the
            // real desktop host remains to be checked.
            windows[..index]
                .iter()
                .any(|window| window.icon_host && window.visible)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(id: isize, icon_host: bool, topmost: bool) -> DesktopWindow {
        DesktopWindow {
            id,
            icon_host,
            desktop_surface: icon_host,
            topmost,
            visible: true,
        }
    }
    #[test]
    fn ignores_unrelated_worker_and_current_desktop() {
        assert_eq!(
            desktop_anchor(
                &[
                    window(1, false, true),
                    window(2, false, false),
                    window(9, false, false),
                    window(3, true, false)
                ],
                9
            ),
            DesktopAnchor::After(2)
        );
    }
    #[test]
    fn never_joins_topmost_band() {
        assert_eq!(
            desktop_anchor(
                &[
                    window(1, false, true),
                    window(9, false, true),
                    window(3, true, false)
                ],
                9
            ),
            DesktopAnchor::NonTopmost
        );
        assert_eq!(
            desktop_anchor(&[window(3, true, false)], 9),
            DesktopAnchor::NonTopmost
        );
    }
    #[test]
    fn missing_explorer_fails_to_bottom_not_top() {
        assert_eq!(
            desktop_anchor(&[window(2, false, false)], 9),
            DesktopAnchor::Bottom
        );
    }

    #[test]
    fn ignores_hidden_helper_windows_as_anchors() {
        let hidden = DesktopWindow {
            visible: false,
            ..window(4, false, false)
        };
        assert_eq!(
            desktop_anchor(
                &[
                    window(2, false, false),
                    window(9, false, false),
                    hidden,
                    window(3, true, false)
                ],
                9
            ),
            DesktopAnchor::After(2)
        );
    }

    #[test]
    fn raised_explorer_cannot_lift_desktop_over_a_lower_application() {
        let stack = [
            window(1, false, true),
            window(2, false, false),
            window(3, true, false),
            window(4, false, false),
            window(9, false, false),
        ];
        assert_eq!(desktop_anchor(&stack, 9), DesktopAnchor::Bottom);
        assert!(!desktop_needs_restack(&stack, 9));
        let raised = [stack[0], stack[1], stack[4], stack[2], stack[3]];
        assert!(desktop_needs_restack(&raised, 9));
        assert_eq!(desktop_applications_below(&raised, 9), 1);
    }

    #[test]
    fn non_topmost_is_not_sufficient_when_applications_are_below() {
        let stack = [
            window(9, false, false),
            window(2, false, false),
            window(3, true, false),
        ];
        assert!(desktop_needs_restack(&stack, 9));
        assert_eq!(desktop_anchor(&stack, 9), DesktopAnchor::After(2));
    }

    #[test]
    fn invisible_or_minimized_helpers_do_not_cause_repair_loops() {
        let helper = DesktopWindow {
            visible: false,
            ..window(4, false, false)
        };
        let stack = [
            window(2, false, false),
            helper,
            window(9, false, false),
            helper,
            window(3, true, false),
            helper,
        ];
        assert!(!desktop_needs_restack(&stack, 9));
        assert_eq!(desktop_applications_below(&stack, 9), 0);
    }

    #[test]
    fn verified_wallpaper_surface_is_not_an_application() {
        let wallpaper = DesktopWindow {
            desktop_surface: true,
            ..window(4, false, false)
        };
        let stack = [
            window(2, false, false),
            window(9, false, false),
            window(3, true, false),
            wallpaper,
        ];
        assert_eq!(desktop_anchor(&stack, 9), DesktopAnchor::After(2));
        assert!(!desktop_needs_restack(&stack, 9));
    }

    #[test]
    fn desktop_behind_host_is_restored_without_moving_over_apps() {
        let stack = [
            window(2, false, false),
            window(3, true, false),
            window(9, false, false),
        ];
        assert!(desktop_needs_restack(&stack, 9));
        assert_eq!(desktop_anchor(&stack, 9), DesktopAnchor::After(2));
    }

    #[test]
    fn topmost_desktop_is_always_repaired() {
        assert!(desktop_needs_restack(
            &[window(9, false, true), window(3, true, false)],
            9
        ));
    }

    #[test]
    fn every_five_window_permutation_keeps_all_applications_above_desktop() {
        fn check(stack: &mut [DesktopWindow], start: usize) {
            if start != stack.len() {
                for index in start..stack.len() {
                    stack.swap(start, index);
                    check(stack, start + 1);
                    stack.swap(start, index);
                }
                return;
            }
            let anchor = desktop_anchor(stack, 9);
            let mut result = stack.to_vec();
            let desktop = result.remove(result.iter().position(|window| window.id == 9).unwrap());
            let insert = match anchor {
                DesktopAnchor::Bottom => result.len(),
                DesktopAnchor::After(id) => {
                    result.iter().position(|window| window.id == id).unwrap() + 1
                }
                DesktopAnchor::NonTopmost => result
                    .iter()
                    .position(|window| !window.topmost)
                    .unwrap_or(result.len()),
            };
            result.insert(insert, desktop);
            assert_eq!(
                desktop_applications_below(&result, 9),
                0,
                "{stack:?} -> {anchor:?}"
            );
            assert!(
                !desktop_needs_restack(&result, 9),
                "repair must settle: {result:?}"
            );
        }
        check(
            &mut [
                window(9, false, false),
                window(2, false, false),
                window(4, false, false),
                window(3, true, false),
                window(1, false, true),
            ],
            0,
        );
    }
}
