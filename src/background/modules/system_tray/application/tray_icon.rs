use std::hash::{DefaultHasher, Hash, Hasher};

use macui_core::system_state::{SysTrayIcon, SysTrayIconId, SystrayIconAction};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Controls::{WM_MOUSEHOVER, WM_MOUSELEAVE},
        Shell::{NIN_POPUPCLOSE, NIN_POPUPOPEN, NIN_SELECT},
        WindowsAndMessaging::{
            AllowSetForegroundWindow, GetWindowThreadProcessId, HICON, SendNotifyMessageW,
            WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
        },
    },
};

use crate::{
    modules::system_tray::application::{SystemTrayManager, util::Util},
    utils::{constants::MACUI_COMMON, icon_extractor::convert_hicon_to_rgba_image},
    windows_api::{WindowsApi, window::Window},
};
use macui_ipc::messages::{IconEventData, Win32TrayEvent};

// Native NIF_ICON + NULL is an explicit blank frame, not a failed decode or a
// fabricated unread signal. Retain the last display image while observing it.
const BLANK_NATIVE_ICON_HASH: &str = "native:null-icon";

/// Events that can be emitted by `Systray`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum SystrayEvent {
    IconAdd(SysTrayIcon),
    IconUpdate(SysTrayIcon),
    IconRemove(SysTrayIconId),
}

impl SystemTrayManager {
    /// Returns all icons managed by the `Systray`.
    pub fn icons(&self) -> Vec<SysTrayIcon> {
        self.icons.values()
    }

    fn find_icon(&self, icon_data: &IconEventData) -> Result<Option<SysTrayIcon>, ()> {
        super::identity::find_icon(&self.icons.values(), icon_data).map(|icon| icon.cloned())
    }

    /// Native metadata/image work belongs to the single tray worker, while a
    /// click may acknowledge notifications concurrently. Merge that one shared
    /// field at commit time, never resurrecting a copied notification or icon
    /// which Explorer reset removed. No events are emitted while the map locks.
    fn commit_icon_update(
        &self,
        mut updated: SysTrayIcon,
        notification: Option<&str>,
    ) -> Option<SysTrayIcon> {
        let id = updated.stable_id.clone();
        self.icons
            .get(&id, |current| {
                updated.notification = match notification {
                    Some(text) => (!text.is_empty()).then(|| text.to_owned()),
                    None => current.notification.clone(),
                };
                updated.notification_revision = current
                    .notification_revision
                    .saturating_add(u32::from(notification.is_some_and(|text| !text.is_empty())));
                if updated == *current {
                    return None;
                }
                *current = updated.clone();
                Some(updated)
            })
            .flatten()
    }

    /// A client may exit or replace its callback window without NIM_DELETE.
    /// Keep hidden live icons, but retire dead owners so reopening Task Manager
    /// cannot leave historical entries in either tray presentation.
    pub(super) fn remove_dead_icons(&self) -> bool {
        let mut removed = false;
        self.icons.retain(|(_, icon)| {
            let live = icon
                .window_handle
                .is_none_or(|handle| WindowsApi::is_window(HWND(handle as _)));
            removed |= !live;
            live
        });
        removed
    }

    /// Handles an event from the `Systray`.
    ///
    /// Returns `None` if the event should be ignored (e.g. if an icon that
    /// doesn't exist was removed).
    pub(super) fn process_event(
        &self,
        mut event: Win32TrayEvent,
        copied_icon: Option<isize>,
    ) -> Option<SystrayEvent> {
        // set application name if not tooltip is set
        match &mut event {
            Win32TrayEvent::IconAdd { data: icon_data }
            | Win32TrayEvent::IconUpdate { data: icon_data }
                if icon_data.tooltip.as_ref().is_none()
                    && self.find_icon(icon_data) == Ok(None) =>
            {
                if let Some(window_handle) = icon_data.window_handle {
                    let window = Window::from(window_handle);
                    if let Ok(name) = window.app_display_name() {
                        icon_data.tooltip = Some(name);
                    }
                }
            }
            _ => {}
        }

        match &event {
            Win32TrayEvent::IconAdd { data: icon_data }
            | Win32TrayEvent::IconUpdate { data: icon_data } => {
                let found_icon = self.find_icon(icon_data).ok()?;

                // Update the icon in-place if found.
                if let Some(found_icon) = found_icon {
                    // Avoid emitting update events for no-op changes.
                    if !has_change(&found_icon, icon_data) {
                        return None;
                    }

                    let mut to_update = found_icon.clone();
                    if let Some(uid) = icon_data.uid {
                        to_update.uid = Some(uid);
                    }

                    if let Some(window_handle) = icon_data.window_handle {
                        to_update.window_handle = Some(window_handle);
                    }

                    if let Some(guid) = icon_data.guid {
                        to_update.guid = Some(guid);
                    }

                    if let Some(tooltip) = &icon_data.tooltip {
                        to_update.tooltip = tooltip.clone();
                    }

                    if icon_data.icon_handle == Some(0) {
                        to_update.icon_handle = Some(0);
                        to_update.icon_image_hash = Some(BLANK_NATIVE_ICON_HASH.into());
                    } else if let Some(icon_handle) = copied_icon {
                        // A flashing tray icon can change pixels without changing HICON.
                        if let Ok(img) = convert_hicon_to_rgba_image(&HICON(icon_handle as _)) {
                            to_update.icon_handle = icon_data.icon_handle;
                            let hash = Some(image_to_hash(&img));
                            if to_update.icon_image_hash != hash {
                                let path = MACUI_COMMON
                                    .app_temp_dir()
                                    .join(format!("{}.png", to_update.stable_id));
                                if img.save(&path).is_ok() {
                                    to_update.icon_image_hash = hash;
                                    to_update.icon_path = Some(path);
                                }
                            }
                        }
                    }

                    if let Some(callback_message) = icon_data.callback_message {
                        to_update.callback_message = Some(callback_message);
                    }

                    if let Some(version) = icon_data.version {
                        to_update.version = Some(version);
                    }

                    if let Some(is_visible) = icon_data.is_visible {
                        to_update.is_visible = is_visible;
                    }

                    // Flashing applications may send the same HICON repeatedly.
                    // Decode it to detect pixel changes, but do not redraw every
                    // webview when neither pixels nor metadata actually changed.
                    self.commit_icon_update(to_update, icon_data.notification.as_deref())
                        .map(SystrayEvent::IconUpdate)
                } else {
                    // Icon doesn't exist yet, so add new icon. Skip icons that
                    // cannot be identified.
                    let stable_id = icon_data.guid.map(SysTrayIconId::Guid).or({
                        match (icon_data.window_handle, icon_data.uid) {
                            (Some(handle), Some(uid)) => {
                                Some(SysTrayIconId::HandleUid(handle, uid))
                            }
                            _ => None,
                        }
                    })?;

                    log::trace!("Tray icon added: {}", stable_id);

                    let mut icon_image_hash = (icon_data.icon_handle == Some(0))
                        .then(|| BLANK_NATIVE_ICON_HASH.to_owned());
                    let mut icon_path = None;

                    if let Some(icon_handle) = copied_icon
                        && let Ok(img) = convert_hicon_to_rgba_image(&HICON(icon_handle as _))
                    {
                        icon_image_hash = Some(image_to_hash(&img));
                        let path = MACUI_COMMON
                            .app_temp_dir()
                            .join(format!("{}.png", stable_id));
                        if img.save(&path).is_ok() {
                            icon_path = Some(path);
                        }
                    }

                    let icon = SysTrayIcon {
                        executable: icon_data
                            .window_handle
                            .and_then(|hwnd| Window::from(hwnd).process().program_path().ok()),
                        notification_revision: u32::from(
                            icon_data
                                .notification
                                .as_ref()
                                .is_some_and(|text| !text.is_empty()),
                        ),
                        notification: icon_data
                            .notification
                            .clone()
                            .filter(|text| !text.is_empty()),
                        stable_id,
                        uid: icon_data.uid,
                        window_handle: icon_data.window_handle,
                        guid: icon_data.guid,
                        tooltip: icon_data.tooltip.clone().unwrap_or_default(),
                        icon_handle: icon_data.icon_handle,
                        icon_path,
                        icon_image_hash,
                        callback_message: icon_data.callback_message,
                        version: icon_data.version,
                        is_visible: icon_data.is_visible.unwrap_or(true),
                    };

                    self.icons.upsert(icon.stable_id.clone(), icon.clone());
                    Some(SystrayEvent::IconAdd(icon))
                }
            }
            Win32TrayEvent::IconRemove { data: icon_data } => {
                let icon_id = self
                    .find_icon(icon_data)
                    .ok()?
                    .map(|icon| icon.stable_id.clone());
                if let Some(icon_id) = icon_id {
                    log::trace!("Tray icon removed: {}", icon_id);
                    self.icons.remove(&icon_id);
                    Some(SystrayEvent::IconRemove(icon_id))
                } else {
                    None
                }
            }
        }
    }

    /// Sends an action to the systray icon.
    pub fn send_action(
        &self,
        icon_id: &SysTrayIconId,
        action: &SystrayIconAction,
    ) -> crate::Result<()> {
        log::trace!("Sending icon action: {:?} to: {:?}", action, icon_id);
        let icon = self
            .icons
            .get(icon_id, |v| v.clone())
            .ok_or("Icon not found")?;

        // Early return if we don't have the required fields.
        let window_handle = icon
            .window_handle
            .ok_or("Inoperable icon, missing window handle")?;
        let uid = icon.uid.ok_or("Inoperable icon, missing uid")?;
        let callback = icon
            .callback_message
            .ok_or("Inoperable icon, missing callback")?;

        if !WindowsApi::is_window(HWND(window_handle as _)) {
            return Err("Window handle is invalid".into());
        }

        let is_mouse_click = matches!(
            action,
            SystrayIconAction::LeftClick
                | SystrayIconAction::LeftDoubleClick
                | SystrayIconAction::RightClick
                | SystrayIconAction::MiddleClick
        );

        // For mouse clicks, there is often a menu that appears after the
        // click. Allow the notify icon to gain focus so that the menu can be
        // dismissed after clicking outside.
        if is_mouse_click {
            let mut proc_id = u32::default();
            unsafe { GetWindowThreadProcessId(HWND(window_handle as _), Some(&mut proc_id)) };
            let _ = unsafe { AllowSetForegroundWindow(proc_id) };
        }

        let wm_messages = match action {
            SystrayIconAction::LeftClick => vec![WM_LBUTTONDOWN, WM_LBUTTONUP],
            SystrayIconAction::LeftDoubleClick => vec![WM_LBUTTONDBLCLK, WM_LBUTTONUP],
            SystrayIconAction::RightClick => {
                vec![WM_RBUTTONDOWN, WM_RBUTTONUP]
            }
            SystrayIconAction::MiddleClick => {
                vec![WM_MBUTTONDOWN, WM_MBUTTONUP]
            }
            SystrayIconAction::HoverEnter => vec![WM_MOUSEHOVER],
            SystrayIconAction::HoverLeave => vec![WM_MOUSELEAVE],
            SystrayIconAction::HoverMove => vec![WM_MOUSEMOVE],
        };

        for wm_message in wm_messages {
            Self::notify_icon(window_handle, callback, uid, icon.version, wm_message)?;
        }

        // Additional messages are sent for version 4 and above. Explorer sends
        // these for version 3 as well though, so we do the same.
        // Ref: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyicona#remarks
        if icon.version.is_some_and(|version| version >= 3) {
            let v3_message = match action {
                SystrayIconAction::HoverEnter => NIN_POPUPOPEN,
                SystrayIconAction::HoverLeave => NIN_POPUPCLOSE,
                SystrayIconAction::LeftClick => NIN_SELECT,
                SystrayIconAction::RightClick => WM_CONTEXTMENU,
                _ => return Ok(()),
            };

            Self::notify_icon(window_handle, callback, uid, icon.version, v3_message)?;
        }

        Ok(())
    }

    /// Sends a message to the systray icon window.
    fn notify_icon(
        window_handle: isize,
        callback: u32,
        uid: u32,
        version: Option<u32>,
        message: u32,
    ) -> crate::Result<()> {
        // The wparam is the mouse position for version > 3 (with the low and
        // high word being the x and y-coordinates respectively), and the UID
        // for version <= 3.
        let wparam = if version.is_some_and(|version| version > 3) {
            let cursor_pos = Util::cursor_position()?;
            Util::pack_i32(cursor_pos.0 as i16, cursor_pos.1 as i16) as u32
        } else {
            uid
        };

        // The high word for the lparam is the UID for version > 3, and 0 for
        // version <= 3. The low word is always the message.
        let lparam = if version.is_some_and(|version| version > 3) {
            Util::pack_i32(message as i16, uid as i16)
        } else {
            Util::pack_i32(message as i16, 0)
        };

        unsafe {
            SendNotifyMessageW(
                HWND(window_handle as _),
                callback,
                WPARAM(wparam as _),
                LPARAM(lparam as _),
            )
        }?;

        Ok(())
    }
}

/// Computes a hash of the icon image.
fn image_to_hash(icon_image: &image::RgbaImage) -> String {
    let mut hasher = DefaultHasher::new();
    icon_image.as_raw().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Checks if the icon would change from the given icon data.
fn has_change(icon: &SysTrayIcon, data: &IconEventData) -> bool {
    data.notification.is_some()
        || data.icon_handle.is_some()
        || data.uid.is_some_and(|uid| icon.uid != Some(uid))
        || data
            .window_handle
            .is_some_and(|handle| icon.window_handle != Some(handle))
        || data.guid.is_some_and(|guid| icon.guid != Some(guid))
        || data.tooltip.as_ref().is_some_and(|t| &icon.tooltip != t)
        || data
            .icon_handle
            .is_some_and(|handle| icon.icon_handle != Some(handle))
        || data
            .callback_message
            .is_some_and(|msg| icon.callback_message != Some(msg))
        || data.version.is_some_and(|ver| icon.version != Some(ver))
        || data
            .is_visible
            .is_some_and(|visible| icon.is_visible != visible)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::lock_free::SyncHashMap;

    fn seeded_manager() -> SystemTrayManager {
        let manager = SystemTrayManager {
            icons: SyncHashMap::new(),
            _loader: None,
        };
        let guid = uuid::Uuid::from_u128(1234);
        let icon = SysTrayIcon {
            stable_id: SysTrayIconId::Guid(guid),
            guid: Some(guid),
            window_handle: Some(100),
            uid: Some(1),
            executable: None,
            notification: Some("Actual message".into()),
            notification_revision: 7,
            tooltip: "WeChat unread".into(),
            icon_handle: Some(10),
            icon_path: None,
            icon_image_hash: Some("baseline".into()),
            callback_message: Some(1024),
            version: Some(4),
            is_visible: true,
        };
        manager.icons.upsert(icon.stable_id.clone(), icon);
        manager
    }

    fn partial_data() -> IconEventData {
        IconEventData {
            window_handle: Some(100),
            uid: Some(1),
            guid: None,
            tooltip: None,
            notification: None,
            icon_handle: None,
            callback_message: None,
            version: None,
            is_visible: None,
        }
    }

    #[test]
    fn guidless_modify_and_delete_operate_on_the_original_guid_entry() {
        let manager = seeded_manager();
        let mut data = partial_data();
        data.tooltip = Some("Task Manager CPU 12%".into());
        assert!(matches!(
            manager.process_event(Win32TrayEvent::IconUpdate { data }, None),
            Some(SystrayEvent::IconUpdate(_))
        ));
        assert_eq!(manager.icons.len(), 1);
        assert_eq!(manager.icons.values()[0].tooltip, "Task Manager CPU 12%");
        assert!(matches!(
            manager.process_event(
                Win32TrayEvent::IconRemove {
                    data: partial_data()
                },
                None
            ),
            Some(SystrayEvent::IconRemove(_))
        ));
        assert!(manager.icons.is_empty());
    }

    #[test]
    fn explicit_clear_does_not_create_a_new_notification_revision() {
        let manager = seeded_manager();
        let mut data = partial_data();
        data.notification = Some(String::new());
        data.tooltip = Some(String::new());
        manager
            .process_event(Win32TrayEvent::IconUpdate { data }, None)
            .unwrap();
        let icon = &manager.icons.values()[0];
        assert_eq!(icon.notification, None);
        assert_eq!(icon.notification_revision, 7);
        assert_eq!(icon.tooltip, "");
    }

    #[test]
    fn same_hicon_is_redecoded_for_clients_that_change_its_pixels_in_place() {
        let manager = seeded_manager();
        let mut data = partial_data();
        data.icon_handle = Some(10);
        assert!(has_change(&manager.icons.values()[0], &data));
    }

    #[test]
    fn explicit_blank_frame_retains_the_app_image_and_is_not_a_notification() {
        let manager = seeded_manager();
        let id = manager.icons.values()[0].stable_id.clone();
        manager.icons.get(&id, |icon| {
            icon.icon_path = Some("last-app-image.png".into())
        });
        let mut data = partial_data();
        data.icon_handle = Some(0);
        assert!(matches!(
            manager.process_event(Win32TrayEvent::IconUpdate { data }, None),
            Some(SystrayEvent::IconUpdate(_))
        ));
        let icon = &manager.icons.values()[0];
        assert_eq!(icon.icon_handle, Some(0));
        assert_eq!(
            icon.icon_image_hash.as_deref(),
            Some(BLANK_NATIVE_ICON_HASH)
        );
        assert_eq!(
            icon.icon_path.as_deref(),
            Some(std::path::Path::new("last-app-image.png"))
        );
        assert_eq!(icon.notification_revision, 7);
        assert_eq!(icon.notification.as_deref(), Some("Actual message"));
    }

    #[test]
    fn acknowledgement_between_snapshot_and_commit_does_not_resurrect_a_notification() {
        let manager = seeded_manager();
        let mut pending = manager.icons.values()[0].clone();
        pending.tooltip = "WeChat latest tooltip".into();
        pending.icon_image_hash = Some("latest-frame".into());
        manager.acknowledge_notification(&pending.stable_id);
        let committed = manager.commit_icon_update(pending, None).unwrap();
        assert_eq!(committed.notification, None);
        assert_eq!(committed.notification_revision, 7);
        assert_eq!(committed.tooltip, "WeChat latest tooltip");
        assert_eq!(committed.icon_image_hash.as_deref(), Some("latest-frame"));
    }

    #[test]
    fn acknowledgement_does_not_roll_back_native_metadata_or_a_new_notification_after_it() {
        let manager = seeded_manager();
        let mut pending = manager.icons.values()[0].clone();
        pending.tooltip = "New native state".into();
        pending.icon_image_hash = Some("new-pixels".into());
        pending.is_visible = false;
        manager.commit_icon_update(pending.clone(), None).unwrap();
        manager.acknowledge_notification(&pending.stable_id);
        let acknowledged = manager.icons.values()[0].clone();
        assert_eq!(acknowledged.tooltip, "New native state");
        assert_eq!(acknowledged.icon_image_hash.as_deref(), Some("new-pixels"));
        assert!(!acknowledged.is_visible);
        assert_eq!(acknowledged.notification, None);

        let committed = manager
            .commit_icon_update(pending, Some("Actual later notification"))
            .unwrap();
        assert_eq!(
            committed.notification.as_deref(),
            Some("Actual later notification")
        );
        assert_eq!(committed.notification_revision, 8);
    }

    #[test]
    fn a_snapshot_cannot_reinsert_an_icon_removed_during_native_decoding() {
        let manager = seeded_manager();
        let pending = manager.icons.values()[0].clone();
        manager.icons.remove(&pending.stable_id);
        assert!(manager.commit_icon_update(pending, None).is_none());
        assert!(manager.icons.is_empty());
    }
}
