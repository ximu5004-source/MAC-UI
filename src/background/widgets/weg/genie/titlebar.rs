//! Route an explicitly hit native minimize button through the same Dock effect.
//! No application is injected or subclassed. All other pointer input passes on.
use std::{
    cell::RefCell,
    sync::{
        Once, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
};

use macui_core::rect::Rect;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::IsWindowEnabled,
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GA_ROOT, GWL_STYLE, GetAncestor, GetForegroundWindow,
            GetMessageW, GetWindowLongPtrW, GetWindowThreadProcessId, HHOOK, HTMINBUTTON, IsIconic,
            IsWindow, IsWindowVisible, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, PostMessageW,
            SC_MINIMIZE, SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, SetWindowsHookExW,
            TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_NCHITTEST, WM_SYSCOMMAND, WS_MINIMIZEBOX, WindowFromPoint,
        },
    },
};

use crate::{
    state::application::FULL_STATE,
    windows_api::{
        event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
        window::Window,
    },
};

static START: Once = Once::new();
static ACTIONS: OnceLock<SyncSender<Request>> = OnceLock::new();
// Reserved at pointer-down, so a consumed click always has room in the queue.
static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    hwnd: isize,
    pid: u32,
}

struct Press {
    identity: Identity,
    target: Rect,
    foreground: isize,
}

struct Request {
    identity: Identity,
    target: Rect,
}

thread_local! {
    static PRESSED: RefCell<Option<Press>> = const { RefCell::new(None) };
}

struct BusyLease;
impl Drop for BusyLease {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

struct MouseHook(HHOOK);
impl Drop for MouseHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.0);
        }
    }
}

fn identity(hwnd: HWND) -> Option<Identity> {
    let mut pid = 0;
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    (pid != 0).then_some(Identity {
        hwnd: hwnd.0 as isize,
        pid,
    })
}

fn same_window(expected: Identity) -> bool {
    identity(HWND(expected.hwnd as _)) == Some(expected)
}

fn hit_lparam(point: POINT) -> Option<LPARAM> {
    // WM_NCHITTEST packs signed 16-bit screen coordinates. Never wrap a large
    // virtual desktop onto an unrelated caption button.
    let x = i16::try_from(point.x).ok()?;
    let y = i16::try_from(point.y).ok()?;
    Some(LPARAM(
        ((x as u16 as u32) | ((y as u16 as u32) << 16)) as i32 as isize,
    ))
}

fn native_minimize_button(point: POINT) -> Option<Identity> {
    let lparam = hit_lparam(point)?;
    unsafe {
        let hit = WindowFromPoint(point);
        let root = GetAncestor(hit, GA_ROOT);
        if root.is_invalid()
            || !IsWindowEnabled(root).as_bool()
            || !IsWindowVisible(root).as_bool()
            || IsIconic(root).as_bool()
            || GetWindowLongPtrW(root, GWL_STYLE) & WS_MINIMIZEBOX.0 as isize == 0
        {
            return None;
        }
        let mut result = 0;
        // A hook must return promptly. A hung/custom caption is left native;
        // capture, IPC and animation never run inside this callback.
        if SendMessageTimeoutW(
            root,
            WM_NCHITTEST,
            WPARAM(0),
            lparam,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            12,
            Some(&mut result),
        )
        .0 == 0
            || result != HTMINBUTTON as usize
        {
            return None;
        }
        identity(root)
    }
}

fn foreground_root() -> isize {
    unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT).0 as isize }
}

fn confirmed_release(pressed: Identity, released: Option<Identity>, focus_unchanged: bool) -> bool {
    released == Some(pressed) && focus_unchanged
}

fn native_fallback(expected: Identity) {
    if same_window(expected) {
        unsafe {
            // Preserve the normal caption command if our queued action fails.
            let _ = PostMessageW(
                Some(HWND(expected.hwnd as _)),
                WM_SYSCOMMAND,
                WPARAM(SC_MINIMIZE as usize),
                LPARAM(0),
            );
        }
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let next = || unsafe { CallNextHookEx(None, code, wparam, lparam) };
    if code < 0 || !matches!(wparam.0 as u32, WM_LBUTTONDOWN | WM_LBUTTONUP) {
        return next();
    }
    let Some(event) = (unsafe { (lparam.0 as *const MSLLHOOKSTRUCT).as_ref() }) else {
        return next();
    };
    if event.flags & LLMHF_INJECTED != 0 {
        return next();
    }

    if wparam.0 as u32 == WM_LBUTTONUP {
        let Some(press) = PRESSED.with(|state| state.borrow_mut().take()) else {
            return next();
        };
        // Consume only the release paired with a down we consumed. Moving off
        // the button or changing focus cancels it, just like a native caption.
        if confirmed_release(
            press.identity,
            native_minimize_button(event.pt),
            foreground_root() == press.foreground,
        ) {
            let request = Request {
                identity: press.identity,
                target: press.target,
            };
            if ACTIONS
                .get()
                .is_none_or(|sender| sender.try_send(request).is_err())
            {
                BUSY.store(false, Ordering::Release);
                native_fallback(press.identity);
            }
        } else {
            BUSY.store(false, Ordering::Release);
        }
        return LRESULT(1);
    }

    // A fresh physical down also recovers a press whose release was lost during
    // a session/device switch. Never leave pointer interception latched.
    if PRESSED.with(|state| state.borrow_mut().take()).is_some() {
        BUSY.store(false, Ordering::Release);
    }
    if ACTIONS.get().is_none()
        || BUSY.load(Ordering::Acquire)
        || !IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
        || !IS_DISPLAY_ON.load(Ordering::Acquire)
        || !FULL_STATE.load().is_weg_enabled()
        || !super::enabled()
    {
        return next();
    }
    let Some(identity) = native_minimize_button(event.pt) else {
        return next();
    };
    let Some(target) = super::targets::target_for(Window::from(identity.hwnd)) else {
        return next();
    };
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return next();
    }
    PRESSED.with(|state| {
        *state.borrow_mut() = Some(Press {
            identity,
            target,
            foreground: foreground_root(),
        })
    });
    LRESULT(1)
}

pub(crate) fn start() {
    START.call_once(|| {
        let (sender, receiver) = sync_channel::<Request>(1);
        let worker = std::thread::Builder::new()
            .name("titlebar-genie-actions".into())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    let _lease = BusyLease;
                    if !same_window(request.identity) {
                        continue;
                    }
                    let window = Window::from(request.identity.hwnd);
                    if window.is_minimized() || !window.is_visible() {
                        continue;
                    }
                    let action = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let target = super::targets::target_for(window).unwrap_or(request.target);
                        log::debug!(
                            "Title-bar Genie minimize requested: hwnd={}",
                            request.identity.hwnd
                        );
                        super::toggle(window, true, Some(target))
                    }));
                    if !matches!(action, Ok(Ok(()))) {
                        log::warn!(
                            "Title-bar animation failed; preserving native minimize command"
                        );
                        native_fallback(request.identity);
                    }
                }
            });
        if let Err(error) = worker {
            log::warn!("Title-bar animation worker unavailable: {error}");
            return;
        }
        let _ = ACTIONS.set(sender);
        let hook_thread = std::thread::Builder::new()
            .name("titlebar-genie-input".into())
            .spawn(|| unsafe {
                let instance = crate::windows_api::WindowsApi::module_handle_w()
                    .ok()
                    .map(Into::into);
                let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), instance, 0) {
                    Ok(hook) => MouseHook(hook),
                    Err(error) => {
                        log::warn!("Title-bar animation input unavailable: {error}");
                        return;
                    }
                };
                log::info!("Native title-bar minimize animation input ready");
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                drop(hook);
            });
        if let Err(error) = hook_thread {
            log::warn!("Title-bar animation input thread unavailable: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_button_release_must_match_window_and_process() {
        let a = Identity { hwnd: 10, pid: 20 };
        assert!(confirmed_release(a, Some(a), true));
        assert!(!confirmed_release(a, None, true));
        assert!(!confirmed_release(
            a,
            Some(Identity { hwnd: 11, ..a }),
            true
        ));
        assert!(!confirmed_release(a, Some(Identity { pid: 21, ..a }), true));
        assert!(!confirmed_release(a, Some(a), false));
    }

    #[test]
    fn negative_monitor_hit_coordinates_are_signed_and_never_wrap() {
        let packed = hit_lparam(POINT { x: -3839, y: -100 }).unwrap().0 as u32;
        assert_eq!(packed as u16 as i16, -3839);
        assert_eq!((packed >> 16) as u16 as i16, -100);
        assert!(hit_lparam(POINT { x: 32_768, y: 0 }).is_none());
        assert!(hit_lparam(POINT { x: 0, y: -32_769 }).is_none());
    }
}
