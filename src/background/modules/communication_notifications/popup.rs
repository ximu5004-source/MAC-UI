//! Read only visible, client-owned notification popups through public UIA/MSAA.
//! The generic path requires an Alert subtree; it never walks a chat main window.
#[path = "popup_uia.rs"]
mod uia;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

use macui_core::system_state::CommunicationApp;
use parking_lot::Mutex;
use windows::{
    Win32::{
        System::{Com::IDispatch, Variant::VARIANT},
        UI::{
            Accessibility::{AccessibleChildren, AccessibleObjectFromWindow, IAccessible},
            WindowsAndMessaging::{
                GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
                WS_POPUP,
            },
        },
    },
    core::Interface,
};

use crate::{
    error::Result,
    hook::HookManager,
    modules::system_tray::SystemTrayManager,
    utils::spawn_named_thread,
    windows_api::{
        ComThread, WindowEnumerator,
        event_window::IS_INTERACTIVE_SESSION,
        window::{Window, event::WinEvent},
    },
};

use super::{
    application::CommunicationNotificationManager,
    domain::{ClientIdentity, PopupNotification, communication_executable},
};

const ROLE_ALERT: i32 = 8;
const ROLE_LIST_ITEM: i32 = 0x22;
const ROLE_STATIC_TEXT: i32 = 0x29;
const STATE_INVISIBLE_OR_OFFSCREEN: i32 = 0x8000 | 0x10000;
const MAX_POPUPS: usize = 64;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 12;
const MAX_TEXT_CHARS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupRule {
    WeChat,
    DingTalk,
    Alert,
}

// The first two rules are evidenced in MyDockFinder 5.9.9.92. There are no
// invented WeCom/Feishu class names: those clients use the generic Alert rule.
fn popup_rule(app: CommunicationApp, class: &str, style: u32, ex_style: u32) -> Option<PopupRule> {
    match (app, class) {
        (CommunicationApp::WeChat, "TrayNotifyWnd") => Some(PopupRule::WeChat),
        (CommunicationApp::DingTalk, "DingtalkMsgComming") => Some(PopupRule::DingTalk),
        _ if style & WS_POPUP.0 != 0
            && ex_style & (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) != 0 =>
        {
            Some(PopupRule::Alert)
        }
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PopupKey {
    window: isize,
    process_id: u32,
    executable: std::path::PathBuf,
}

struct Candidate {
    key: PopupKey,
    window: Window,
    app: CommunicationApp,
    rule: PopupRule,
}

fn candidate(window: Window) -> Option<Candidate> {
    if !window.is_window() || !window.is_visible() || window.is_cloaked() {
        return None;
    }
    // Check native popup characteristics before opening process metadata.
    let class = window.class();
    let style = unsafe { GetWindowLongPtrW(window.hwnd(), GWL_STYLE) } as u32;
    let ex_style = unsafe { GetWindowLongPtrW(window.hwnd(), GWL_EXSTYLE) } as u32;
    if class != "TrayNotifyWnd"
        && class != "DingtalkMsgComming"
        && (style & WS_POPUP.0 == 0 || ex_style & (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) == 0)
    {
        return None;
    }
    let process = window.process();
    let executable = process.program_path().ok()?;
    let app = communication_executable(&executable)?;
    let rule = popup_rule(app, &class, style, ex_style)?;
    Some(Candidate {
        key: PopupKey {
            window: window.address(),
            process_id: process.id(),
            executable,
        },
        window,
        app,
        rule,
    })
}

fn candidates() -> Vec<Candidate> {
    let mut found = Vec::new();
    let _ = WindowEnumerator::new().for_each(|window| {
        if found.len() < MAX_POPUPS {
            if let Some(candidate) = candidate(window) {
                found.push(candidate);
            }
        }
    });
    found
}

#[derive(Default)]
struct SeenPopup {
    text: Option<String>,
    baseline: bool,
}

impl SeenPopup {
    fn observe(&mut self, body: &str) -> bool {
        if self.text.as_deref() == Some(body) {
            return false;
        }
        self.text = Some(body.to_owned());
        !std::mem::take(&mut self.baseline)
    }
}

#[derive(Default)]
struct PopupLifecycle {
    watched: HashSet<isize>,
    invalidated: HashSet<isize>,
}

struct PopupState {
    seen: HashMap<PopupKey, SeenPopup>,
    stopping: Arc<AtomicBool>,
    lifecycle: Arc<Mutex<PopupLifecycle>>,
    uia: Option<uia::UiaReader>,
}

impl PopupState {
    fn new(stopping: Arc<AtomicBool>, lifecycle: Arc<Mutex<PopupLifecycle>>) -> Self {
        // Baseline window identities without calling an external MSAA provider
        // during lazy manager construction. Existing text is not a new event.
        let seen: HashMap<_, _> = candidates()
            .into_iter()
            .map(|item| {
                (
                    item.key,
                    SeenPopup {
                        baseline: true,
                        text: None,
                    },
                )
            })
            .collect();
        lifecycle.lock().watched = seen.keys().map(|key| key.window).collect();
        Self {
            seen,
            stopping,
            lifecycle,
            uia: uia::UiaReader::new()
                .map_err(|error| {
                    log::warn!("Notification popup UIA unavailable; MSAA remains active: {error}");
                })
                .ok(),
        }
    }

    fn scan(&mut self) {
        if self.stopping.load(Ordering::Acquire) || !IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
        {
            return;
        }
        let candidates = candidates();
        let present: HashSet<_> = candidates.iter().map(|item| item.key.clone()).collect();
        let invalidated: HashSet<_> = {
            let mut lifecycle = self.lifecycle.lock();
            lifecycle.watched = present.iter().map(|key| key.window).collect();
            lifecycle.invalidated.drain().collect()
        };
        self.seen
            .retain(|key, _| present.contains(key) && !invalidated.contains(&key.window));
        for item in candidates {
            if self.stopping.load(Ordering::Acquire) {
                return;
            }
            let Some((body, provider)) = self
                .uia
                .as_ref()
                .and_then(|reader| reader.text(item.window, item.rule))
                .map(|body| (body, "UIA"))
                .or_else(|| accessible_text(item.window, item.rule).map(|body| (body, "MSAA")))
            else {
                continue;
            };
            if self.stopping.load(Ordering::Acquire) {
                return;
            }
            // An HWND can be destroyed/reused while an external provider is
            // answering. Check ownership again before accepting its text.
            if !candidate(item.window).is_some_and(|current| current.key == item.key) {
                continue;
            }
            let seen = self.seen.entry(item.key.clone()).or_default();
            if !seen.observe(&body) {
                continue;
            }
            // Diagnostics identify the adapter, never record message contents.
            log::debug!(
                "Captured client notification popup: app={:?}, provider={provider}, class={}",
                item.app,
                item.window.class()
            );
            // Keep the owning process identity for safe activation even if the
            // client has no tray icon. Never launch a guessed executable.
            let icon = SystemTrayManager::instance()
                .icons()
                .into_iter()
                .find(|icon| {
                    icon.window_handle.is_some_and(|handle| {
                        Window::from(handle).process().id() == item.key.process_id
                    }) && icon.executable.as_ref().is_some_and(|path| {
                        path.as_os_str()
                            .eq_ignore_ascii_case(item.key.executable.as_os_str())
                    })
                });
            CommunicationNotificationManager::publish_popup(PopupNotification {
                app: item.app,
                client: ClientIdentity {
                    process_id: item.key.process_id,
                    executable: item.key.executable,
                },
                tray_id: icon.as_ref().map(|icon| icon.stable_id.clone()),
                icon_path: icon.and_then(|icon| icon.icon_path),
                body,
            });
        }
    }
}

/// Owns the accessibility apartment, bounded wakeups, and hook subscription.
/// All COM interfaces are created and dropped exclusively on its windowless MTA.
pub(super) struct PopupCollector {
    stop: mpsc::Sender<()>,
    scheduler: Option<JoinHandle<()>>,
    subscription: String,
    com: Option<Arc<ComThread<PopupState>>>,
    stopping: Arc<AtomicBool>,
}

struct PendingScan(Arc<AtomicBool>);

impl Drop for PendingScan {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl PopupCollector {
    pub(super) fn start() -> Result<Self> {
        let stopping = Arc::new(AtomicBool::new(false));
        let state_stopping = stopping.clone();
        let lifecycle = Arc::new(Mutex::new(PopupLifecycle::default()));
        let state_lifecycle = lifecycle.clone();
        let com = Arc::new(ComThread::spawn_mta(
            "Notification popup accessibility",
            move || Ok(PopupState::new(state_stopping, state_lifecycle)),
        )?);
        let requested = Arc::new(AtomicBool::new(false));
        let event_requested = requested.clone();
        let subscription = HookManager::subscribe(move |(event, window)| match event {
            WinEvent::ObjectShow | WinEvent::ObjectNameChange | WinEvent::SystemAlert => {
                event_requested.store(true, Ordering::Release);
            }
            WinEvent::ObjectHide | WinEvent::ObjectDestroy => {
                // Never append one COM command per WinEvent: a hung external
                // accessibility provider must not make that queue grow forever.
                let mut lifecycle = lifecycle.lock();
                if lifecycle.watched.contains(&window.address()) {
                    lifecycle.invalidated.insert(window.address());
                }
                event_requested.store(true, Ordering::Release);
            }
            _ => {}
        });
        let (stop, receiver) = mpsc::channel();
        let scheduled_com = com.clone();
        let queued = Arc::new(AtomicBool::new(false));
        let scheduler = spawn_named_thread("Notification popup scheduler", move || {
            let mut ticks = 0u8;
            while matches!(
                receiver.recv_timeout(Duration::from_millis(250)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                ticks = ticks.wrapping_add(1);
                if ticks % 4 != 0 && !requested.swap(false, Ordering::AcqRel) {
                    continue;
                }
                if queued.swap(true, Ordering::AcqRel) {
                    continue;
                }
                let pending = queued.clone();
                if scheduled_com
                    .execute(move |state| {
                        let _pending = PendingScan(pending);
                        state.scan();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Ok(Self {
            stop,
            scheduler: Some(scheduler),
            subscription,
            com: Some(com),
            stopping,
        })
    }
}

impl Drop for PopupCollector {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        HookManager::unsubscribe(&self.subscription);
        let _ = self.stop.send(());
        if let Some(scheduler) = self.scheduler.take() {
            // The scheduler only enqueues work; it never waits on MSAA calls.
            let _ = scheduler.join();
        }
        if let Some(com) = self.com.take() {
            // ComThread::drop joins its apartment. An external provider can hang an
            // MSAA call, so keep that join off the manager/UI teardown thread.
            // Normal cleanup still posts WM_QUIT and drops COM on its owner.
            spawn_named_thread("Notification popup cleanup", move || drop(com));
        }
    }
}

fn accessible_text(window: Window, rule: PopupRule) -> Option<String> {
    // OBJID_WINDOW = 0 matches the observed adapter; client-root fallback is
    // useful for clients publishing their accessibility tree at OBJID_CLIENT.
    for object_id in [0, 0xffff_fffc] {
        let mut raw = std::ptr::null_mut();
        if unsafe {
            AccessibleObjectFromWindow(window.hwnd(), object_id, &IAccessible::IID, &mut raw)
        }
        .is_err()
            || raw.is_null()
        {
            continue;
        }
        let root = unsafe { IAccessible::from_raw(raw) };
        let mut reader = TextReader {
            rule,
            visited: 0,
            texts: Vec::new(),
        };
        reader.visit(&root, &VARIANT::from(0i32), 0, false);
        if !reader.texts.is_empty() {
            return Some(reader.texts.join("\n"));
        }
    }
    None
}

struct TextReader {
    rule: PopupRule,
    visited: usize,
    texts: Vec<String>,
}

impl TextReader {
    fn visit(&mut self, object: &IAccessible, child: &VARIANT, depth: usize, in_alert: bool) {
        if self.visited >= MAX_NODES
            || depth > MAX_DEPTH
            || self.texts.len() >= 16
        {
            return;
        }
        self.visited += 1;
        let role = unsafe { object.get_accRole(child) }
            .ok()
            .and_then(|v| i32::try_from(&v).ok());
        let state = unsafe { object.get_accState(child) }
            .ok()
            .and_then(|v| i32::try_from(&v).ok())
            .unwrap_or(0);
        if state & STATE_INVISIBLE_OR_OFFSCREEN != 0 {
            return;
        }
        let in_alert = in_alert || role == Some(ROLE_ALERT);
        let wanted = match self.rule {
            PopupRule::WeChat => role == Some(ROLE_STATIC_TEXT),
            PopupRule::DingTalk => role == Some(ROLE_LIST_ITEM),
            PopupRule::Alert => in_alert && matches!(role, Some(ROLE_STATIC_TEXT | ROLE_LIST_ITEM)),
        };
        if wanted {
            let (mut left, mut top, mut width, mut height) = (0, 0, 0, 0);
            if unsafe { object.accLocation(&mut left, &mut top, &mut width, &mut height, child) }
                .is_ok()
                && width > 0
                && height > 0
            {
                if let Ok(name) = unsafe { object.get_accName(child) } {
                    let text: String = name
                        .to_string()
                        .trim()
                        .chars()
                        .take(MAX_TEXT_CHARS)
                        .collect();
                    if !text.is_empty() && !self.texts.contains(&text) && self.texts.len() < 16 {
                        self.texts.push(text);
                    }
                }
            }
        }
        // Simple children have no subtree. Dispatch children recurse with SELF.
        if i32::try_from(child).ok() != Some(0) {
            return;
        }
        let count = unsafe { object.accChildCount() }
            .unwrap_or(0)
            .clamp(0, MAX_NODES as i32) as usize;
        if count == 0 {
            return;
        }
        let mut children = vec![VARIANT::default(); count];
        let mut obtained = 0;
        if unsafe { AccessibleChildren(object, 0, &mut children, &mut obtained) }.is_err() {
            return;
        }
        for child in children.iter().take(obtained.max(0) as usize) {
            if let Ok(dispatch) = IDispatch::try_from(child) {
                if let Ok(accessible) = dispatch.cast::<IAccessible>() {
                    self.visit(&accessible, &VARIANT::from(0i32), depth + 1, in_alert);
                }
            } else if let Ok(id) = i32::try_from(child) {
                if id > 0 {
                    self.visit(object, child, depth + 1, in_alert);
                }
            }
        }
    }
}

pub(super) fn activate_client(client: &ClientIdentity) -> Result<()> {
    let mut target = None;
    WindowEnumerator::new().for_each(|window| {
        if target.is_some() || !window.is_interactable_and_not_hidden() {
            return;
        }
        if owns_window(client, window) && candidate(window).is_none() {
            target = Some(window);
        }
    })?;
    let window = target.ok_or("The messaging client window is no longer available")?;
    window.unminimize()?;
    window.focus()
}

pub(super) fn owns_window(client: &ClientIdentity, window: Window) -> bool {
    let process = window.process();
    window.is_window()
        && process.id() == client.process_id
        && process.program_path().is_ok_and(|path| {
            path.as_os_str()
                .eq_ignore_ascii_case(client.executable.as_os_str())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_rules_require_the_correct_client_and_class() {
        assert_eq!(
            popup_rule(CommunicationApp::WeChat, "TrayNotifyWnd", 0, 0),
            Some(PopupRule::WeChat)
        );
        assert_eq!(
            popup_rule(CommunicationApp::DingTalk, "DingtalkMsgComming", 0, 0),
            Some(PopupRule::DingTalk)
        );
        assert_eq!(
            popup_rule(CommunicationApp::Feishu, "DingtalkMsgComming", 0, 0),
            None
        );
        assert_eq!(
            popup_rule(CommunicationApp::WeCom, "TrayNotifyWnd", 0, 0),
            None
        );
    }

    #[test]
    fn generic_clients_require_a_transient_popup_before_reading_alerts() {
        for app in [CommunicationApp::WeCom, CommunicationApp::Feishu] {
            assert_eq!(popup_rule(app, "MainWindow", 0, WS_EX_TOOLWINDOW.0), None);
            assert_eq!(popup_rule(app, "MainWindow", WS_POPUP.0, 0), None);
            assert_eq!(
                popup_rule(app, "NativePopup", WS_POPUP.0, WS_EX_NOACTIVATE.0),
                Some(PopupRule::Alert)
            );
        }
    }

    #[test]
    fn startup_text_is_baselined_but_later_popup_content_is_new() {
        let mut initial = SeenPopup {
            baseline: true,
            text: None,
        };
        assert!(!initial.observe("Already visible"));
        assert!(!initial.observe("Already visible"));
        assert!(initial.observe("A later notification"));
        assert!(!initial.observe("A later notification"));
        // A fresh popup lifetime can carry an identical real message.
        assert!(SeenPopup::default().observe("A later notification"));
    }

    /// Exercises the real Windows MSAA provider using only an owned, zero-alpha
    /// popup and a native STATIC child. It never starts PopupCollector or scans
    /// other windows. Run explicitly when an interactive desktop is available.
    #[test]
    #[ignore = "creates an owned zero-alpha native popup; run explicitly on Windows"]
    fn owned_static_popup_msaa_probe() {
        use windows::{
            Win32::{
                Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
                System::LibraryLoader::GetModuleHandleW,
                UI::WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DestroyWindow, LWA_ALPHA, RegisterClassW,
                    SW_HIDE, SW_SHOWNOACTIVATE, SetLayeredWindowAttributes, SetWindowTextW,
                    ShowWindow, UnregisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_CHILD,
                    WS_EX_LAYERED, WS_VISIBLE,
                },
            },
            core::w,
        };

        struct OwnedClass(HINSTANCE);
        unsafe extern "system" fn probe_window_proc(
            hwnd: HWND,
            message: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        impl Drop for OwnedClass {
            fn drop(&mut self) {
                let _ = unsafe { UnregisterClassW(w!("MacUiOwnedMsaaPopupProbe"), Some(self.0)) };
            }
        }
        struct OwnedWindow(HWND);
        impl Drop for OwnedWindow {
            fn drop(&mut self) {
                let _ = unsafe { DestroyWindow(self.0) };
            }
        }

        let sta = ComThread::spawn("Owned notification MSAA probe", || Ok(())).unwrap();
        sta.call(|_| -> Result<()> {
            let instance = HINSTANCE(unsafe { GetModuleHandleW(None)? }.0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(probe_window_proc),
                hInstance: instance,
                lpszClassName: w!("MacUiOwnedMsaaPopupProbe"),
                ..Default::default()
            };
            if unsafe { RegisterClassW(&class) } == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let _class = OwnedClass(instance);
            let popup = OwnedWindow(unsafe {
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                    w!("MacUiOwnedMsaaPopupProbe"), w!(""), WS_POPUP,
                    40, 40, 420, 90, None, None, Some(instance), None,
                )?
            });
            unsafe { SetLayeredWindowAttributes(popup.0, COLORREF(0), 0, LWA_ALPHA)?; }
            let text = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(), w!("STATIC"),
                    w!("Owned MSAA probe: fictional message 42"), WS_CHILD | WS_VISIBLE,
                    8, 8, 400, 64, Some(popup.0), None, Some(instance), None,
                )?
            };
            let sender = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(), w!("STATIC"), w!("Test sender"), WS_CHILD | WS_VISIBLE,
                    8, 1, 200, 18, Some(popup.0), None, Some(instance), None,
                )?
            };
            let window = Window::from(popup.0);
            let foreground_before = Window::get_foregrounded().address();
            let _ = unsafe { ShowWindow(popup.0, SW_SHOWNOACTIVATE) };
            assert!(window.is_visible(), "the owned zero-alpha popup must be logically visible");
            assert_eq!(Window::get_foregrounded().address(), foreground_before, "the probe must not take focus");

            let actual = accessible_text(window, PopupRule::WeChat);
            println!("owned-msaa-probe visible STATIC accName: {actual:?}");
            let lines: Vec<_> = actual.as_deref().unwrap().lines().collect();
            assert_eq!(lines.len(), 2);
            assert!(lines.contains(&"Test sender"));
            assert!(lines.contains(&"Owned MSAA probe: fictional message 42"));
            // No Alert role was exposed by the standard controls: generic
            // WeCom/Feishu admission must remain stricter than the text rule.
            assert!(accessible_text(window, PopupRule::Alert).is_none());

            let mut store = super::super::domain::CommunicationNotificationStore::default();
            let publish = |store: &mut super::super::domain::CommunicationNotificationStore, body: Option<String>| {
                body.is_some_and(|body| store.update_popup(PopupNotification {
                    app: CommunicationApp::WeChat,
                    client: ClientIdentity { process_id: std::process::id(), executable: std::env::current_exe().unwrap() },
                    tray_id: None, icon_path: None, body,
                }, 1))
            };
            assert!(publish(&mut store, actual));
            assert_eq!(store.notifications().len(), 1);
            store.clear();

            let _ = unsafe { ShowWindow(popup.0, SW_HIDE) };
            let hidden = accessible_text(window, PopupRule::WeChat);
            println!("owned-msaa-probe hidden popup: {hidden:?}");
            assert!(!publish(&mut store, hidden));
            assert!(store.notifications().is_empty());

            unsafe { SetWindowTextW(text, w!(""))?; SetWindowTextW(sender, w!(""))?; }
            let _ = unsafe { ShowWindow(popup.0, SW_SHOWNOACTIVATE) };
            let empty = accessible_text(window, PopupRule::WeChat);
            println!("owned-msaa-probe empty STATIC: {empty:?}");
            assert!(!publish(&mut store, empty));
            assert!(store.notifications().is_empty());
            println!("owned-msaa-probe passed: visible text captured; hidden/empty text not published; no client enumeration");
            Ok(())
        }).unwrap();
    }
}
