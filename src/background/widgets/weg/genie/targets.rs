//! Last measured physical icon bounds, owned by the Dock webview that sent them.
//! The mouse hook uses only a try-lock and lightweight native identity checks.
use std::{collections::HashMap, sync::LazyLock};

use macui_core::rect::Rect;
use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    System::Threading::GetCurrentProcessId,
    UI::WindowsAndMessaging::{GetClientRect, GetWindowThreadProcessId, IsWindow, IsWindowVisible},
};

use crate::{error::Result, widgets::webview::WidgetWebviewLabel, windows_api::window::Window};

const MAX_TARGETS: usize = 512;
const MAX_DOCKS: usize = 16;

#[derive(Clone)]
struct IconTarget {
    pid: u32,
    rect: Rect,
}

#[derive(Clone)]
struct DockTargets {
    owner: isize,
    origin: [i32; 2],
    monitor: isize,
    submitted: usize,
    icons: HashMap<isize, IconTarget>,
}

static DOCKS: LazyLock<Mutex<HashMap<isize, DockTargets>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn client_bounds(hwnd: HWND) -> Option<Rect> {
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetClientRect(hwnd, &mut client).ok()?;
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return None;
        }
    }
    Some(Rect {
        left: origin.x,
        top: origin.y,
        right: origin.x.checked_add(client.right)?,
        bottom: origin.y.checked_add(client.bottom)?,
    })
}

fn valid_target(rect: &Rect, bounds: &Rect, screen: &Rect) -> bool {
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    width > 0
        && height > 0
        && width <= 512
        && height <= 512
        && rect.left >= bounds.left
        && rect.top >= bounds.top
        && rect.right <= bounds.right
        && rect.bottom <= bounds.bottom
        && rect.left >= screen.left
        && rect.top >= screen.top
        && rect.right <= screen.right
        && rect.bottom <= screen.bottom
}

pub(crate) fn set_targets(
    webview: tauri::WebviewWindow,
    targets: HashMap<isize, Rect>,
) -> Result<()> {
    let label = WidgetWebviewLabel::try_from_raw(webview.label())?;
    if label.widget_id.as_str() != "@macui/weg" || targets.len() > MAX_TARGETS {
        return Err("Animation icon positions are limited to the Dock".into());
    }
    let submitted = targets.len();
    let owner = HWND(webview.hwnd()?.0);
    let owner_id = owner.0 as isize;
    let bounds = client_bounds(owner).ok_or("Dock client bounds unavailable")?;
    let dock_window = Window::from(owner);
    let monitor = dock_window.monitor();
    let screen = monitor.info()?.monitorInfo.rcMonitor;
    let screen = Rect {
        left: screen.left,
        top: screen.top,
        right: screen.right,
        bottom: screen.bottom,
    };
    let mut icons = HashMap::new();
    for (hwnd, rect) in targets {
        if hwnd == 0 || !valid_target(&rect, &bounds, &screen) {
            continue;
        }
        let window = Window::from(hwnd);
        if !window.is_window() || window.monitor() != monitor {
            continue;
        }
        icons.insert(
            hwnd,
            IconTarget {
                pid: window.process().id(),
                rect,
            },
        );
    }
    let mut docks = DOCKS.lock();
    docks.retain(|owner, _| unsafe { IsWindow(Some(HWND(*owner as _))).as_bool() });
    let previous_count = docks
        .get(&owner_id)
        .map(|dock| (dock.icons.len(), dock.submitted));
    if previous_count != Some((icons.len(), submitted)) {
        log::debug!(
            "Dock animation icon positions: owner={owner_id}, count={}/{submitted}",
            icons.len(),
        );
    }
    {
        if !docks.contains_key(&owner_id) && docks.len() >= MAX_DOCKS {
            return Err("Too many Dock animation position sources".into());
        }
        docks.insert(
            owner_id,
            DockTargets {
                owner: owner_id,
                origin: [bounds.left, bounds.top],
                monitor: monitor.handle().0 as isize,
                submitted,
                icons,
            },
        );
    }
    Ok(())
}

pub(crate) fn target_for(window: Window) -> Option<Rect> {
    let hwnd = window.address();
    let monitor = window.monitor().handle().0 as isize;
    let candidates: Vec<_> = {
        let docks = DOCKS.try_lock()?;
        docks
            .values()
            .filter(|dock| dock.monitor == monitor)
            .filter_map(|dock| {
                dock.icons
                    .get(&hwnd)
                    .map(|icon| (dock.owner, dock.origin, icon.clone()))
            })
            .collect()
    };
    for (owner, origin, icon) in candidates {
        unsafe {
            let owner = HWND(owner as _);
            let mut pid = 0;
            GetWindowThreadProcessId(owner, Some(&mut pid));
            if pid != GetCurrentProcessId() || !IsWindowVisible(owner).as_bool() {
                continue;
            }
            let Some(bounds) = client_bounds(owner) else {
                continue;
            };
            if [bounds.left, bounds.top] != origin
                || window.process().id() != icon.pid
                || icon.rect.left < bounds.left
                || icon.rect.top < bounds.top
                || icon.rect.right > bounds.right
                || icon.rect.bottom > bounds.bottom
                || Window::from(owner).monitor().handle().0 as isize != monitor
            {
                continue;
            }
        }
        return Some(icon.rect);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_real_icon_on_a_negative_origin_monitor() {
        let bounds = Rect {
            left: -3840,
            top: 1080,
            right: 0,
            bottom: 2160,
        };
        let screen = Rect {
            left: -3840,
            top: 0,
            right: 0,
            bottom: 2160,
        };
        let icon = Rect {
            left: -1900,
            top: 2080,
            right: -1850,
            bottom: 2130,
        };
        assert!(valid_target(&icon, &bounds, &screen));
    }

    #[test]
    fn rejects_zero_oversized_outside_and_overflowing_icon_bounds() {
        let bounds = Rect {
            left: 0,
            top: 1080,
            right: 3840,
            bottom: 2160,
        };
        let screen = Rect {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2160,
        };
        for icon in [
            Rect {
                left: 100,
                top: 2000,
                right: 100,
                bottom: 2050,
            },
            Rect {
                left: 100,
                top: 2000,
                right: 613,
                bottom: 2050,
            },
            Rect {
                left: 100,
                top: 2000,
                right: 150,
                bottom: 2200,
            },
            Rect {
                left: i32::MIN,
                top: 2000,
                right: i32::MAX,
                bottom: 2050,
            },
            Rect {
                left: 100,
                top: 500,
                right: 150,
                bottom: 550,
            },
        ] {
            assert!(!valid_target(&icon, &bounds, &screen));
        }
    }
}
