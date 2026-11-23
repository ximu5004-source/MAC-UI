use std::{
    collections::HashSet,
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use macui_core::{state::AppExtraFlag, system_state::UserAppWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    WS_CHILD, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MINIMIZEBOX,
};

use crate::{
    hook::HookManager,
    modules::apps::application::{USER_APPS_MANAGER, UserAppWinEvent, UserAppsManager},
    utils::{lock_free::SyncVec, spawn_named_thread},
    windows_api::{
        WindowEnumerator, WindowsApi,
        event_window::IS_INTERACTIVE_SESSION,
        window::{Window, event::WinEvent},
    },
};

pub(super) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A native predicate may synchronously message another GUI thread or call COM.
/// Snapshot first; only the final identity-based removal holds the collection.
fn prune_uninteractable(
    windows: &SyncVec<UserAppWindow>,
    mut still_interactable: impl FnMut(&UserAppWindow) -> bool,
) -> Vec<isize> {
    let rejected: HashSet<_> = windows
        .to_vec()
        .into_iter()
        .filter(|entry| !still_interactable(entry))
        .map(|entry| (entry.hwnd, entry.process.id))
        .collect();
    let mut removed = Vec::new();
    windows.retain(|entry| {
        let keep = !rejected.contains(&(entry.hwnd, entry.process.id));
        if !keep {
            removed.push(entry.hwnd);
        }
        keep
    });
    removed
}

/// Commit only fields owned by this event. A slow native query must not replace
/// unrelated fields if another event refreshed the entry in the meantime.
fn apply_window_update(current: &mut UserAppWindow, fresh: UserAppWindow, event: WinEvent) {
    match event {
        WinEvent::ObjectNameChange => {
            current.title = fresh.title;
            current.app_name = fresh.app_name;
            current.relaunch = fresh.relaunch;
            current.prevent_pinning = fresh.prevent_pinning;
            current.umid = fresh.umid;
        }
        WinEvent::SynDebouncedRectChange => {
            current.is_zoomed = fresh.is_zoomed;
            current.is_fullscreen = fresh.is_fullscreen;
            current.rect = fresh.rect;
            current.monitor = fresh.monitor;
        }
        WinEvent::SystemForeground => current.last_foreground_at = fresh.last_foreground_at,
        WinEvent::SystemMinimizeStart | WinEvent::SystemMinimizeEnd => {
            current.is_iconic = fresh.is_iconic;
        }
        _ => {}
    }
}

fn refresh_tracked_window(
    windows: &SyncVec<UserAppWindow>,
    hwnd: isize,
    event: WinEvent,
    query: impl FnOnce(&mut UserAppWindow, WinEvent) -> bool,
) -> bool {
    let Some(mut fresh) = windows.find_and_clone(|entry| entry.hwnd == hwnd) else {
        return false;
    };
    // query is deliberately outside both find_and_clone and for_each locks.
    if !query(&mut fresh, event) {
        return false;
    }
    let mut fresh = Some(fresh);
    let mut changed = false;
    windows.for_each(|entry| {
        if fresh
            .as_ref()
            .is_some_and(|fresh| entry.hwnd == fresh.hwnd && entry.process.id == fresh.process.id)
        {
            apply_window_update(
                entry,
                fresh.take().expect("one tracked window per HWND"),
                event,
            );
            changed = true;
        }
    });
    changed
}

impl UserAppsManager {
    pub(super) fn init_listing_app_windows() -> Vec<UserAppWindow> {
        let mut initial = Vec::new();
        let _ = WindowEnumerator::new().for_each(|window| {
            if is_interactable_window(&window) {
                initial.push(window.to_serializable());
            }
        });

        // The enumerator yields windows in z-order (top-most first). Stamp
        // last_foreground_at so that clients sorting by that field preserve
        // z-order on first load. Each item gets now - index, keeping relative
        // ordering without blocking any future foreground update.
        let now = now_millis();
        for (i, w) in initial.iter_mut().enumerate() {
            w.last_foreground_at = now - i as i64;
        }

        HookManager::subscribe(|(event, window)| Self::on_win_event(event, window));

        spawn_named_thread("InteractableWindowsRevalidator", || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(5000));

                // Pause when session is not interactive to reduce CPU usage
                if !IS_INTERACTIVE_SESSION.load(Ordering::Acquire) {
                    continue;
                }

                let removed = prune_uninteractable(&Self::instance().interactable_windows, |w| {
                    let window = Window::from(w.hwnd);
                    window.is_interactable_and_not_hidden()
                });
                for hwnd in removed {
                    Self::send(UserAppWinEvent::Removed(hwnd));
                }

                // Scan for windows that now qualify but were never tracked (e.g. style/frame
                // state was not settled at creation time).
                let _ = WindowEnumerator::new().for_each(|window| {
                    if is_interactable_window(&window) && USER_APPS_MANAGER.add_win(&window) {
                        Self::send(UserAppWinEvent::Added(window.address()));
                    }
                });
            }
        });

        initial
    }

    fn on_win_event(event: WinEvent, window: Window) {
        // A Dock transition hides only for its short native state commit. Keep
        // its app item stable; no timer masks a later real hide/destroy event.
        if matches!(event, WinEvent::ObjectHide | WinEvent::ObjectStateChange)
            && (crate::widgets::weg::genie::is_transient_visibility(window)
                || (event == WinEvent::ObjectHide && window.is_visible()))
        {
            return;
        }
        let mut is_interactable = USER_APPS_MANAGER.contains_win(&window);

        match event {
            WinEvent::ObjectCreate | WinEvent::ObjectShow => {
                if !is_interactable
                    && is_interactable_window(&window)
                    && USER_APPS_MANAGER.add_win(&window)
                {
                    Self::send(UserAppWinEvent::Added(window.address()));
                }
            }
            WinEvent::ObjectNameChange
            | WinEvent::ObjectParentChange
            | WinEvent::ObjectUncloaked
            | WinEvent::ObjectStateChange => {
                let was_interactable = is_interactable;
                is_interactable = is_interactable_window(&window);
                match (was_interactable, is_interactable) {
                    (false, true) => {
                        if USER_APPS_MANAGER.add_win(&window) {
                            Self::send(UserAppWinEvent::Added(window.address()));
                        }
                    }
                    (true, false) => {
                        USER_APPS_MANAGER.remove_win(&window);
                        Self::send(UserAppWinEvent::Removed(window.address()));
                    }
                    _ => {}
                }

                // re-check for UWP apps that on creation starts without a parent
                if event == WinEvent::ObjectParentChange
                    && let Some(parent) = window.parent()
                    && parent.is_interactable_and_not_hidden()
                    && USER_APPS_MANAGER.add_win(&parent)
                {
                    Self::send(UserAppWinEvent::Added(parent.address()));
                }
            }
            WinEvent::ObjectHide => {
                // UWP ApplicationFrameHosts are always hidden on minimize
                if is_interactable && !window.is_frame().unwrap_or(false) {
                    USER_APPS_MANAGER.remove_win(&window);
                    Self::send(UserAppWinEvent::Removed(window.address()));
                }
            }
            #[allow(clippy::collapsible_match)]
            WinEvent::ObjectDestroy => {
                if is_interactable {
                    USER_APPS_MANAGER.remove_win(&window);
                    Self::send(UserAppWinEvent::Removed(window.address()));
                }
            }
            _ => {}
        }

        // update cases on UserAppWindow
        if is_interactable {
            let changed = refresh_tracked_window(
                &USER_APPS_MANAGER.interactable_windows,
                window.address(),
                event,
                Self::update_window_data,
            );

            if changed {
                Self::send(UserAppWinEvent::Updated(window.address()));
            }
        }
    }

    fn update_window_data(data: &mut UserAppWindow, event: WinEvent) -> bool {
        match event {
            WinEvent::ObjectNameChange => {
                let window = Window::from(data.hwnd);
                data.title = window.title();

                // Windows has no event for property-store changes, so we piggyback on the
                // name change event to re-check the umid. Some apps (e.g. Firefox PWAs) only
                // get their AppUserModelId assigned after the window is already created.
                let umid = window.app_user_model_id();
                data.app_name = window.app_display_name().unwrap_or_default();
                let (relaunch, prevent_pinning) = window.relaunch_info(&umid);
                data.relaunch = relaunch;
                data.prevent_pinning = prevent_pinning;
                data.umid = umid.map(|umid| umid.to_string());

                true
            }
            WinEvent::SynDebouncedRectChange => {
                let window = Window::from(data.hwnd);
                data.is_zoomed = window.is_maximized();
                data.is_fullscreen = window.is_fullscreen();
                data.rect = window.inner_rect().ok();
                data.monitor = window.monitor().stable_id().unwrap_or_default();
                true
            }
            WinEvent::SystemForeground => {
                let now = now_millis();
                data.last_foreground_at = now;
                true
            }
            WinEvent::SystemMinimizeStart => {
                data.is_iconic = true;
                true
            }
            WinEvent::SystemMinimizeEnd => {
                data.is_iconic = false;
                true
            }
            _ => false,
        }
    }
}

/// The idea with this module is contain all the logic under the filteriong of windows
/// that can be considered as applications windows, it means windows that are interactable
/// for the users.
///
/// As windows properties can change, this should be reevaluated on every change.
pub fn is_interactable_window(window: &Window) -> bool {
    // It must be a visible Window and not cloaked
    if !window.is_window() || !window.is_visible() || window.is_cloaked() {
        return false;
    }

    // this class is used for edge tabs to be shown as independent windows on alt + tab
    // this only applies when the new tab is created it is binded to explorer.exe for some reason
    // maybe we can search/learn more about edge tabs later.
    // fix: https://github.com/eythaann/Seelen-UI/issues/83
    if window.class() == "Windows.Internal.Shell.TabProxyWindow" {
        return false;
    }

    let style = WindowsApi::get_styles(window.hwnd());
    let ex_style = WindowsApi::get_ex_styles(window.hwnd());

    if !ex_style.contains(WS_EX_APPWINDOW) {
        // It must not be owned by another window
        if style.contains(WS_CHILD) || window.owner().is_some() {
            return false;
        }

        // Discard tool windows without WS_EX_APPWINDOW
        if ex_style.contains(WS_EX_TOOLWINDOW) || ex_style.contains(WS_EX_NOACTIVATE) {
            return false;
        }
    }

    // Reject helpers/children before GetWindowText: for a window in this same
    // process it synchronously sends WM_GETTEXT to that window's GUI thread.
    // A Shell extension STA may be busy preparing/destroying such a helper.
    if window.title().is_empty() {
        return false;
    }

    let process = window.process();
    // unmanageable window, these probably are system processes
    if process.open_limited_handle().is_err() {
        return false;
    }

    // Internal behaviour for macui ui widgets:
    // Discard unminimizable windows (they have no caption/title bar)
    if !style.contains(WS_MINIMIZEBOX) && process.is_macui() {
        return false;
    }

    if process.is_frozen().unwrap_or(false) {
        return false;
    }

    let to_validate = match window.get_frame_creator() {
        Ok(None) => return false, // not found
        Ok(Some(creator)) => creator,
        Err(_) => *window, // window is not a frame
    };

    match to_validate.get_app_config() {
        Ok(Some(config)) => {
            if config.options.contains(&AppExtraFlag::NoInteractive) {
                return false;
            }
        }
        Ok(_) => {}
        Err(err) => {
            log::error!("Error getting app config: {err}");
            return false;
        }
    }

    true
}

#[cfg(test)]
mod locking_tests {
    use super::*;
    use macui_core::system_state::{MonitorId, ProcessInformation};

    fn entry(hwnd: isize, pid: u32) -> UserAppWindow {
        UserAppWindow {
            hwnd,
            monitor: MonitorId("fixture-monitor".into()),
            title: "fixture-title".into(),
            app_name: "fixture-app".into(),
            is_zoomed: false,
            is_iconic: false,
            is_fullscreen: false,
            umid: None,
            process: ProcessInformation {
                id: pid,
                path: None,
            },
            prevent_pinning: false,
            relaunch: None,
            rect: None,
            last_foreground_at: 10,
        }
    }

    #[test]
    fn native_revalidation_can_read_and_add_without_holding_the_window_list() {
        let windows = SyncVec::from(vec![entry(1, 10), entry(2, 20)]);
        let removed = prune_uninteractable(&windows, |current| {
            // A native/COM callback can reenter readers and another producer.
            // The old retain(predicate) path held this exact lock here.
            assert!(windows.any(|item| item.hwnd == current.hwnd));
            if current.hwnd == 1 {
                windows.push(entry(3, 30));
                return false;
            }
            true
        });
        assert_eq!(removed, [1]);
        assert_eq!(
            windows
                .to_vec()
                .iter()
                .map(|item| item.hwnd)
                .collect::<Vec<_>>(),
            [2, 3]
        );
    }

    #[test]
    fn revalidation_does_not_remove_a_replacement_process_with_the_same_handle() {
        let windows = SyncVec::from(vec![entry(1, 10)]);
        let removed = prune_uninteractable(&windows, |_| {
            windows.replace(vec![entry(1, 20)]);
            false
        });
        assert!(removed.is_empty());
        assert_eq!(windows.to_vec()[0].process.id, 20);
    }

    #[test]
    fn native_update_is_outside_the_lock_and_preserves_other_event_fields() {
        let windows = SyncVec::from(vec![entry(1, 10)]);
        let changed =
            refresh_tracked_window(&windows, 1, WinEvent::ObjectNameChange, |fresh, _| {
                assert!(windows.any(|item| item.hwnd == 1));
                windows.for_each(|item| item.last_foreground_at = 99);
                fresh.title = "new-title".into();
                true
            });
        assert!(changed);
        let current = windows.to_vec().remove(0);
        assert_eq!(current.title, "new-title");
        assert_eq!(current.last_foreground_at, 99);
    }

    #[test]
    fn native_update_cannot_overwrite_a_replacement_process() {
        let windows = SyncVec::from(vec![entry(1, 10)]);
        assert!(!refresh_tracked_window(
            &windows,
            1,
            WinEvent::ObjectNameChange,
            |fresh, _| {
                windows.replace(vec![entry(1, 20)]);
                fresh.title = "stale-title".into();
                true
            }
        ));
        let current = windows.to_vec().remove(0);
        assert_eq!(current.process.id, 20);
        assert_eq!(current.title, "fixture-title");
    }
}
