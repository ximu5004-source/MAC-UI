use slu_ipc::{
    AppIpc,
    messages::{AppMessage, IconEventData, Win32TrayEvent},
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::DataExchange::COPYDATASTRUCT,
    UI::{
        Shell::{
            NIF_GUID, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_STATE, NIF_TIP, NIM_ADD, NIM_DELETE,
            NIM_MODIFY, NIM_SETVERSION, NIS_HIDDEN, NOTIFY_ICON_DATA_FLAGS,
            NOTIFY_ICON_INFOTIP_FLAGS, NOTIFY_ICON_MESSAGE, NOTIFY_ICON_STATE, NOTIFYICONDATAW_0,
        },
        WindowsAndMessaging::{CWPSTRUCT, CallNextHookEx, GetClassNameW, WM_COPYDATA},
    },
};
use windows_core::GUID;

// ============================================================================
// Native Windows Structures
// ============================================================================

/// Tray message sent to `Shell_TrayWnd`
#[repr(C)]
struct ShellTrayMessage {
    magic_number: i32,
    message_type: u32,
    icon_data: NotifyIconData,
    version: u32,
}

/// Contains the data for a system tray icon.
#[repr(C)]
#[derive(Clone, Copy)]
struct NotifyIconData {
    callback_size: u32,
    window_handle: u32,
    uid: u32,
    flags: NOTIFY_ICON_DATA_FLAGS,
    callback_message: u32,
    icon_handle: u32,
    tooltip: [u16; 128],
    state: NOTIFY_ICON_STATE,
    state_mask: NOTIFY_ICON_STATE,
    size_info: [u16; 256],
    anonymous: NOTIFYICONDATAW_0,
    info_title: [u16; 64],
    info_flags: NOTIFY_ICON_INFOTIP_FLAGS,
    guid_item: GUID,
    balloon_icon_handle: u32,
}

impl From<NotifyIconData> for IconEventData {
    fn from(icon_data: NotifyIconData) -> Self {
        let icon_handle = if icon_data.icon_handle != 0 && icon_data.flags.0 & NIF_ICON.0 != 0 {
            Some(icon_data.icon_handle as isize)
        } else {
            None
        };

        let guid = if icon_data.guid_item != GUID::default() && icon_data.flags.0 & NIF_GUID.0 != 0
        {
            Some(uuid::Uuid::from_u128(icon_data.guid_item.to_u128()))
        } else {
            None
        };

        let tooltip = if icon_data.flags.0 & NIF_TIP.0 != 0 {
            let tooltip_len = icon_data.tooltip.iter().position(|&c| c == 0).unwrap_or(0);
            let tooltip_str = String::from_utf16_lossy(&icon_data.tooltip[..tooltip_len])
                .replace('\r', "")
                .to_string();
            (!tooltip_str.is_empty()).then_some(tooltip_str)
        } else {
            None
        };

        let (window_handle, uid) = if icon_data.window_handle != 0 {
            (Some(icon_data.window_handle as isize), Some(icon_data.uid))
        } else {
            (None, None)
        };

        let callback_message = if icon_data.flags.contains(NIF_MESSAGE) {
            Some(icon_data.callback_message)
        } else {
            None
        };

        // uVersion shares storage with balloon uTimeout. Only NIM_SETVERSION
        // selects a callback protocol; reading it on NIM_MODIFY corrupts clicks.
        let version = None;
        let is_visible = (icon_data.flags.contains(NIF_STATE)
            && icon_data.state_mask.contains(NIS_HIDDEN))
        .then_some(!icon_data.state.contains(NIS_HIDDEN));

        IconEventData {
            notification: if icon_data.flags.contains(NIF_INFO) {
                let len = icon_data
                    .size_info
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(256);
                let title_len = icon_data
                    .info_title
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(64);
                (len > 0).then(|| {
                    format!(
                        "{}\n{}",
                        String::from_utf16_lossy(&icon_data.info_title[..title_len]),
                        String::from_utf16_lossy(&icon_data.size_info[..len])
                    )
                    .trim()
                    .to_string()
                })
            } else {
                None
            },
            uid,
            window_handle,
            guid,
            tooltip,
            icon_handle,
            callback_message,
            version,
            is_visible,
        }
    }
}

fn get_window_class(hwnd: HWND) -> String {
    let mut text: [u16; 512] = [0; 512];
    let len = unsafe { GetClassNameW(hwnd, &mut text) };
    let length = usize::try_from(len).unwrap_or(0);
    String::from_utf16_lossy(&text[..length])
}

// ============================================================================
// Hook Implementation
// ============================================================================

/// https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks
/// https://learn.microsoft.com/en-us/windows/win32/winmsg/callwndproc
///
/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CallWndProc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let next = || CallNextHookEx(None, code, wparam, lparam);
        if code < 0 {
            return next();
        }

        let Some(msg) = (lparam.0 as *const CWPSTRUCT).as_ref() else {
            return next();
        };

        let class = get_window_class(msg.hwnd);
        if class != "Shell_TrayWnd" {
            return next();
        }

        // Send debug message for all messages received
        /* let debug_msg = format!(
            "CallWndProc - code: {}, hwnd: {:?}, message: 0x{:X}, wparam: {:?}, lparam: {:?}",
            code, msg.hwnd, msg.message, msg.wParam, msg.lParam
        );
        let _ = AppIpc::send_sync(&AppMessage::Debug(debug_msg)); */

        if let Some(event) = process_tray_message(msg) {
            send_event_via_ipc(event);
        }

        next()
    }
}

/// Processes a tray message and returns an event if relevant
unsafe fn process_tray_message(msg: &CWPSTRUCT) -> Option<Win32TrayEvent> {
    unsafe {
        if msg.message != WM_COPYDATA {
            return None;
        }

        let copy_data = (msg.lParam.0 as *const COPYDATASTRUCT).as_ref()?;

        // Type 1 is the tray icon message
        if copy_data.dwData != 1 || copy_data.lpData.is_null() {
            return None;
        }

        if (copy_data.cbData as usize) < std::mem::size_of::<ShellTrayMessage>() {
            return None;
        }

        let tray_message = &*copy_data.lpData.cast::<ShellTrayMessage>();
        let mut icon_data: IconEventData = tray_message.icon_data.into();

        if NOTIFY_ICON_MESSAGE(tray_message.message_type) == NIM_ADD {
            icon_data.version = Some(0);
        } else if NOTIFY_ICON_MESSAGE(tray_message.message_type) == NIM_SETVERSION {
            let version = tray_message.icon_data.anonymous.uVersion;
            if !matches!(version, 0 | 3 | 4) {
                return None;
            }
            // NIM_SETVERSION only changes the version, irrespective of stale
            // flags/union fields left in the caller's NOTIFYICONDATA buffer.
            icon_data.version = Some(version);
            icon_data.tooltip = None;
            icon_data.icon_handle = None;
            icon_data.callback_message = None;
            icon_data.notification = None;
            icon_data.is_visible = None;
        }

        match NOTIFY_ICON_MESSAGE(tray_message.message_type) {
            NIM_ADD => Some(Win32TrayEvent::IconAdd { data: icon_data }),
            NIM_MODIFY | NIM_SETVERSION => Some(Win32TrayEvent::IconUpdate { data: icon_data }),
            NIM_DELETE => Some(Win32TrayEvent::IconRemove { data: icon_data }),
            _ => None,
        }
    }
}

// ============================================================================
// IPC Implementation
// ============================================================================

/// Sends an event through the IPC channel to the main process
fn send_event_via_ipc(event: Win32TrayEvent) {
    // Send the event as AppMessage::TrayChanged via AppIpc
    // If it fails, we simply ignore it (we don't want to crash the hook)
    let _ = AppIpc::send_sync(&AppMessage::TrayChanged(event));
}

// ============================================================================
// DLL Entry Point
// ============================================================================

/// DLL entry point
/// This is called when the DLL is loaded/unloaded in any process
///
/// # Safety
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _hinst_dll: windows::Win32::Foundation::HINSTANCE,
    _fdw_reason: u32,
    _lpv_reserved: *const std::ffi::c_void,
) -> bool {
    // No special initialization needed
    // The hook will be installed by the main process
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_balloon_is_forwarded_without_losing_title() {
        let mut data: NotifyIconData = unsafe { std::mem::zeroed() };
        data.flags = NIF_INFO;
        data.info_title[..4].copy_from_slice(&[84, 101, 115, 116]);
        data.size_info[..3].copy_from_slice(&[78, 101, 119]);
        assert_eq!(
            IconEventData::from(data).notification.as_deref(),
            Some("Test\nNew")
        );
    }

    #[test]
    fn empty_or_unflagged_balloon_is_not_a_new_notification() {
        let mut data: NotifyIconData = unsafe { std::mem::zeroed() };
        data.flags = NIF_INFO;
        assert!(IconEventData::from(data).notification.is_none());
        data.flags = NIF_TIP;
        data.size_info[0] = 65;
        assert!(IconEventData::from(data).notification.is_none());
    }

    fn event_data(message: NOTIFY_ICON_MESSAGE, data: NotifyIconData) -> IconEventData {
        let packet = ShellTrayMessage {
            magic_number: 0,
            message_type: message.0,
            icon_data: data,
            version: 0,
        };
        let copy_data = COPYDATASTRUCT {
            dwData: 1,
            cbData: std::mem::size_of::<ShellTrayMessage>() as u32,
            lpData: (&packet as *const ShellTrayMessage).cast_mut().cast(),
        };
        let message = CWPSTRUCT {
            message: WM_COPYDATA,
            lParam: LPARAM(&copy_data as *const COPYDATASTRUCT as isize),
            ..Default::default()
        };
        match unsafe { process_tray_message(&message) }.unwrap() {
            Win32TrayEvent::IconAdd { data }
            | Win32TrayEvent::IconUpdate { data }
            | Win32TrayEvent::IconRemove { data } => data,
        }
    }

    #[test]
    fn only_set_version_changes_the_callback_protocol() {
        let mut data: NotifyIconData = unsafe { std::mem::zeroed() };
        data.flags = NIF_INFO | NIF_MESSAGE;
        data.anonymous.uTimeout = 4;
        data.callback_message = 1234;
        assert_eq!(event_data(NIM_MODIFY, data).version, None);
        assert_eq!(event_data(NIM_ADD, data).version, Some(0));
        let version_change = event_data(NIM_SETVERSION, data);
        assert_eq!(version_change.version, Some(4));
        assert_eq!(version_change.callback_message, None);
        data.anonymous.uVersion = 0;
        assert_eq!(event_data(NIM_SETVERSION, data).version, Some(0));
    }

    #[test]
    fn partial_updates_preserve_visibility_unless_explicitly_masked() {
        let mut data: NotifyIconData = unsafe { std::mem::zeroed() };
        data.flags = NIF_ICON;
        assert_eq!(IconEventData::from(data).is_visible, None);
        data.flags = NIF_STATE;
        assert_eq!(IconEventData::from(data).is_visible, None);
        data.state_mask = NIS_HIDDEN;
        assert_eq!(IconEventData::from(data).is_visible, Some(true));
        data.state = NIS_HIDDEN;
        assert_eq!(IconEventData::from(data).is_visible, Some(false));
        data.anonymous.uVersion = 4;
        assert_eq!(event_data(NIM_SETVERSION, data).is_visible, None);
    }
}
