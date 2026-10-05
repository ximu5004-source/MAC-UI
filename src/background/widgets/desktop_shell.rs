//! Independent interactive desktop. It never renders, changes or attaches to wallpaper.
use crate::{
    app::get_app_handle,
    error::{Result, ResultLogExt},
    hook::HookManager,
    modules::monitors::{MonitorManager, MonitorManagerEvent},
    utils::{atomic_write_file, constants::SEELEN_COMMON},
    windows_api::{
        WindowsApi,
        event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
        window::event::WinEvent,
    },
};
use slu_ipc::{ServiceIpc, messages::SvcAction};
use slu_utils::desktop_order::{
    DesktopAnchor, DesktopWindow, desktop_anchor, desktop_applications_below, desktop_needs_restack,
};
use std::{
    collections::HashSet,
    path::Path,
    sync::{
        LazyLock, Once,
        atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
    },
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::{
        Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        Gdi::{CombineRgn, CreateRectRgn, DeleteObject, HRGN, RGN_OR, SetWindowRgn},
    },
    UI::Shell::{DefSubclassProc, GetWindowSubclass, SetWindowSubclass},
    UI::WindowsAndMessaging::{
        GW_HWNDNEXT, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetShellWindow, GetTopWindow, GetWindow, HWND_BOTTOM,
        HWND_NOTOPMOST, HWND_TOP, IsWindowVisible, KillTimer, PostMessageW, RegisterWindowMessageW,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
        SetParent, SetTimer, SetWindowLongPtrW, WINDOWPOS, WM_ACTIVATE, WM_NCDESTROY, WM_TIMER,
        WM_WINDOWPOSCHANGED, WM_WINDOWPOSCHANGING, WS_CHILDWINDOW, WS_EX_APPWINDOW,
        WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE, WS_POPUP,
    },
};

static ICON_OWNER: tokio::sync::Mutex<Option<isize>> = tokio::sync::Mutex::const_new(None);
static DESKTOP_WINDOW: AtomicIsize = AtomicIsize::new(0);
// Only native clipped regions receive input. CSS pointer-events cannot pass a
// cross-process click to Explorer; HTTRANSPARENT only forwards in one thread.
type RegionSnapshot = ([f64; 2], Vec<[f64; 4]>);
static REGIONS: parking_lot::Mutex<Option<RegionSnapshot>> = parking_lot::Mutex::new(None);

struct OwnedRegion(HRGN);
impl Drop for OwnedRegion {
    fn drop(&mut self) {
        unsafe { let _ = DeleteObject(self.0.into()); }
    }
}

fn apply_desktop_regions(hwnd: HWND) -> Result<()> {
    let mut client = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client)?; }
    let cached = REGIONS.lock();
    let rectangles = match cached.as_ref() {
        Some((viewport, rects)) => slu_utils::desktop_regions::physical_regions(
            *viewport, [client.right - client.left, client.bottom - client.top], rects,
        ).ok_or("Invalid desktop interactive region geometry")?,
        None => Vec::new(), // Fail closed while mounting, never a fullscreen hit area.
    };
    unsafe {
        let combined = OwnedRegion(CreateRectRgn(0, 0, 0, 0));
        if combined.0.is_invalid() { return Err("Could not create desktop region".into()); }
        for [left, top, right, bottom] in rectangles {
            let part = OwnedRegion(CreateRectRgn(left, top, right, bottom));
            if part.0.is_invalid()
                || CombineRgn(Some(combined.0), Some(combined.0), Some(part.0), RGN_OR).0 == 0
            { return Err("Could not combine desktop interactive regions".into()); }
        }
        if SetWindowRgn(hwnd, Some(combined.0), true) == 0 {
            return Err("Could not apply desktop interactive regions".into());
        }
        // SetWindowRgn transfers ownership to Windows on success.
        std::mem::forget(combined);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn set_desktop_regions(
    webview: tauri::WebviewWindow, viewport: [f64; 2], rects: Vec<[f64; 4]>,
) -> Result<()> {
    let label = super::webview::WidgetWebviewLabel::try_from_raw(webview.label())?;
    let handle = webview.hwnd()?.0 as isize;
    if label.widget_id.as_str() != "@seelen/desktop-shell"
        || DESKTOP_WINDOW.load(Ordering::Acquire) != handle
    { return Err("Only the current Desktop Manager can set its regions".into()); }
    if slu_utils::desktop_regions::physical_regions(viewport, [1, 1], &rects).is_none() {
        return Err("Invalid desktop interactive regions".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    webview.run_on_main_thread(move || {
        let result = if DESKTOP_WINDOW.load(Ordering::Acquire) != handle {
            Err("Desktop owner changed".into())
        } else {
            *REGIONS.lock() = Some((viewport, rects));
            apply_desktop_regions(HWND(handle as _))
        };
        let _ = tx.send(result);
    })?;
    rx.await.map_err(|err| err.to_string())?
}
static RESTACK_PENDING: AtomicIsize = AtomicIsize::new(0);
static RESTACKING: AtomicIsize = AtomicIsize::new(0);
static GUARD_TICKS: AtomicU64 = AtomicU64::new(0);
static REPAIR_FAILURE_LOGGED: AtomicBool = AtomicBool::new(false);
const DESKTOP_GUARD: usize = 0x534C5544;
static RESTACK_MESSAGE: LazyLock<u32> = LazyLock::new(|| unsafe {
    RegisterWindowMessageW(windows_core::w!("MAC UI::DesktopOrderGuard"))
});

fn desktop_rect() -> Result<RECT> {
    // Use the same validated snapshot as the frontend's monitor-relative layout.
    // A fresh enumeration during wake can still contain a temporary fallback mode.
    let monitors = MonitorManager::instance().get_cached_data();
    let mut monitors = monitors.iter();
    let first = monitors
        .next()
        .ok_or("No stable desktop display geometry is available")?;
    let mut rect = RECT {
        left: first.rect.left,
        top: first.rect.top,
        right: first.rect.right,
        bottom: first.rect.bottom,
    };
    for monitor in monitors {
        rect.left = rect.left.min(monitor.rect.left);
        rect.top = rect.top.min(monitor.rect.top);
        rect.right = rect.right.max(monitor.rect.right);
        rect.bottom = rect.bottom.max(monitor.rect.bottom);
    }
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return Err("Invalid desktop display geometry".into());
    }
    Ok(rect)
}

fn subscribe_to_display_recovery() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        MonitorManager::subscribe(|event| {
            if event != MonitorManagerEvent::ViewsChanged {
                return;
            }
            // Native geometry recovery must not depend on JavaScript timers in a
            // renderer that may still be resuming or drawing at the previous DPI.
            get_app_handle()
                .run_on_main_thread(|| {
                    let handle = DESKTOP_WINDOW.load(Ordering::Acquire);
                    if handle == 0 {
                        return;
                    }
                    (|| -> Result<()> {
                        let hwnd = HWND(handle as _);
                        WindowsApi::set_position(hwnd, None, &desktop_rect()?, SWP_FRAMECHANGED)?;
                        apply_desktop_regions(hwnd)?;
                        keep_at_desktop(hwnd)?;
                        // Reapply after native desktop sizing as well as the common
                        // recovery pass, which may run before this callback.
                        let label = super::webview::WidgetWebviewLabel::new(
                            &"@seelen/desktop-shell".into(),
                            None,
                            None,
                        );
                        if let Some(window) =
                            tauri::Manager::get_webview_window(get_app_handle(), &label.raw)
                        {
                            super::webview::restore_webview_bounds(&window)?;
                        }
                        Ok(())
                    })()
                    .log_error();
                })
                .log_error();
        });
    });
}

fn subscribe_to_window_order_changes() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let subscription = HookManager::subscribe(|(event, origin)| {
            if !matches!(
                event,
                WinEvent::ObjectShow
                    | WinEvent::ObjectUncloaked
                    | WinEvent::SystemMinimizeEnd
                    | WinEvent::SystemForeground
            ) || !IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
                || !IS_DISPLAY_ON.load(Ordering::Acquire)
            {
                return;
            }
            let desktop = DESKTOP_WINDOW.load(Ordering::Acquire);
            if desktop == 0 {
                return;
            }
            // The foreground event can carry a stale/wrong origin. Other events
            // must retain their own HWND (e.g. an app shown without activation).
            let hwnd = if event == WinEvent::SystemForeground {
                WindowsApi::get_foreground_window()
            } else {
                origin.hwnd()
            };
            if hwnd.is_invalid()
                || hwnd.0 as isize == desktop
                || WindowsApi::get_styles(hwnd).contains(WS_CHILDWINDOW)
                || WindowsApi::get_ex_styles(hwnd).contains(WS_EX_TOPMOST)
                || !WindowsApi::is_window_visible(hwnd)
                || WindowsApi::is_iconic(hwnd)
            {
                return;
            }
            // Only post a coalesced request: hook subscribers share a dispatcher
            // lock. Never enumerate windows, call COM or wait for the UI here.
            keep_at_desktop(HWND(desktop as _)).log_error();
        });
        // Run this cheap notification before slower focus/virtual-desktop work.
        // Subscribe once for process lifetime; a disabled desktop has HWND zero.
        HookManager::set_event_handler_priority(&subscription, 4);
    });
}

fn desktop_order_snapshot(own: isize) -> Result<Vec<DesktopWindow>> {
    let shell = unsafe { GetShellWindow() };
    let shell_pid = WindowsApi::window_thread_process_id(shell).0;
    let mut windows = Vec::new();
    let mut visited = HashSet::new();
    let mut current = unsafe { GetTopWindow(None) }.ok();
    let mut icon_host_rect = None;
    // EnumWindows also includes helper windows outside this sibling chain; its
    // enumeration order must not be mixed with GetWindow's real predecessor.
    // Bound the walk: another process can reorder/destroy a window while sampled.
    while let Some(hwnd) = current {
        if windows.len() >= 4096 || !visited.insert(hwnd.0 as isize) {
            return Err("Desktop z-order changed during enumeration".into());
        }
        let class = WindowsApi::get_class(hwnd).unwrap_or_default();
        let is_shell = shell_pid != 0 && WindowsApi::window_thread_process_id(hwnd).0 == shell_pid;
        let icon_host = is_shell
            && matches!(class.as_str(), "Progman" | "WorkerW")
            && WindowsApi::find_window(Some(hwnd), None, None, Some("SHELLDLL_DefView")).is_ok();
        let rect = WindowsApi::get_outer_window_rect(hwnd).ok();
        // Recognize only the real shell and a full-size Explorer backing WorkerW
        // below its icon host, not arbitrary Explorer windows named WorkerW.
        let desktop_surface = icon_host
            || hwnd == shell
            || (is_shell
                && class == "WorkerW"
                && icon_host_rect.is_some()
                && rect == icon_host_rect);
        if icon_host {
            icon_host_rect = rect;
        }
        let mut visible = unsafe { IsWindowVisible(hwnd).as_bool() }
            && !WindowsApi::is_iconic(hwnd)
            && rect.is_some_and(|rect| rect.right > rect.left && rect.bottom > rect.top)
            && !matches!(
                class.as_str(),
                "Tao Thread Event Target" | "NarratorHelperWindow"
            );
        if visible {
            let mut cloaked = 0u32;
            // CLOAKED is a bitmask; combinations must count as cloaked too.
            if unsafe {
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_CLOAKED,
                    &mut cloaked as *mut _ as _,
                    std::mem::size_of::<u32>() as u32,
                )
            }
            .is_ok()
            {
                visible = cloaked == 0;
            }
        }
        windows.push(DesktopWindow {
            id: hwnd.0 as isize,
            icon_host,
            desktop_surface,
            visible,
            topmost: WindowsApi::get_ex_styles(hwnd).contains(WS_EX_TOPMOST),
        });
        current = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    if !windows.iter().any(|window| window.id == own) {
        return Err("Desktop HWND was missing from the z-order snapshot".into());
    }
    Ok(windows)
}

pub fn keep_at_desktop(hwnd: HWND) -> Result<()> {
    let handle = hwnd.0 as isize;
    if DESKTOP_WINDOW.load(Ordering::Acquire) != handle || *RESTACK_MESSAGE == 0 {
        return Ok(());
    }
    if RESTACK_PENDING
        .compare_exchange(0, handle, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        && let Err(error) =
            unsafe { PostMessageW(Some(hwnd), *RESTACK_MESSAGE, WPARAM(0), LPARAM(0)) }
    {
        let _ = RESTACK_PENDING.compare_exchange(handle, 0, Ordering::AcqRel, Ordering::Acquire);
        return Err(error.into());
    }
    Ok(())
}

/// Runs only from our posted message, after activation/default processing unwinds.
fn restack_desktop(hwnd: HWND) -> Result<()> {
    let handle = hwnd.0 as isize;
    // Only our own native correction can bypass WM_WINDOWPOSCHANGING. This is
    // not a mutex and no widget/map lock is held across Win32 callbacks.
    struct Restacking;
    impl Drop for Restacking {
        fn drop(&mut self) {
            RESTACKING.store(0, Ordering::Release);
        }
    }
    RESTACKING.store(handle, Ordering::Release);
    let _restacking = Restacking;
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER;
    let result = (|| -> Result<()> {
        let before = desktop_order_snapshot(handle)?;
        if !desktop_needs_restack(&before, handle) {
            REPAIR_FAILURE_LOGGED.store(false, Ordering::Release);
            return Ok(());
        }
        let anchor = desktop_anchor(&before, handle);
        let after = match anchor {
            DesktopAnchor::After(id) => HWND(id as _),
            DesktopAnchor::Bottom => HWND_BOTTOM,
            DesktopAnchor::NonTopmost => {
                // HWND_NOTOPMOST alone does nothing for an already normal window.
                // The verified snapshot contains no visible normal applications in
                // this case, so normal-band TOP is safe after explicitly demoting.
                WindowsApi::set_position(hwnd, Some(HWND_NOTOPMOST), &RECT::default(), flags)?;
                HWND_TOP
            }
        };
        WindowsApi::set_position(hwnd, Some(after), &RECT::default(), flags)?;
        let repaired = desktop_order_snapshot(handle)?;
        if desktop_needs_restack(&repaired, handle) {
            return Err("Desktop correction did not reach the verified desktop layer".into());
        }
        REPAIR_FAILURE_LOGGED.store(false, Ordering::Release);
        log::debug!(
            "Restored desktop z-order: hwnd={handle}, anchor={anchor:?}, applications_below={} -> 0",
            desktop_applications_below(&before, handle)
        );
        Ok(())
    })();
    if let Err(error) = result {
        // This includes a partial snapshot, a missing own HWND, a disappeared
        // anchor and a failed verification. Do not leave input blocked until the
        // next timer. The bypass remains active for this fallback as well.
        let fallback = WindowsApi::set_position(hwnd, Some(HWND_BOTTOM), &RECT::default(), flags);
        if !REPAIR_FAILURE_LOGGED.swap(true, Ordering::AcqRel) {
            log::warn!(
                "Desktop z-order unverified; requested fail-safe lower: hwnd={handle}, reason={error}, native_request_succeeded={}",
                fallback.is_ok()
            );
        }
        fallback?;
    }
    Ok(())
}

// Serialize hide/restore with ownership checks: a delayed old-window teardown must
// not restore Explorer icons over a newly started desktop.
pub async fn set_icons_hidden(owner: isize, hidden: bool) -> Result<()> {
    let mut lease = ICON_OWNER.lock().await;
    if hidden {
        ServiceIpc::send(SvcAction::HideNativeDesktopIcons).await?;
        *lease = Some(owner);
    } else if *lease == Some(owner) {
        ServiceIpc::send(SvcAction::RestoreNativeDesktopIcons).await?;
        *lease = None;
    }
    Ok(())
}

pub fn window_destroyed(owner: isize) {
    let _ = DESKTOP_WINDOW.compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire);
    let _ = RESTACK_PENDING.compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire);
    crate::get_tokio_handle().spawn(async move {
        set_icons_hidden(owner, false).await.log_error();
    });
}

unsafe extern "system" fn desktop_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_WINDOWPOSCHANGING
            && lparam.0 != 0
            && RESTACKING.load(Ordering::Acquire) != hwnd.0 as isize
        {
            let pos = &mut *(lparam.0 as *mut WINDOWPOS);
            if !pos.flags.contains(SWP_NOZORDER) {
                // Keep the current order during activation. NOACTIVATE and
                // NOOWNERZORDER cannot be changed effectively in this message.
                // Never enumerate other processes inside this synchronous path.
                pos.flags |= SWP_NOZORDER;
                keep_at_desktop(hwnd).log_error();
            }
        }
        if msg == WM_ACTIVATE || msg == WM_WINDOWPOSCHANGED {
            // Windows activation can still reorder after CHANGING. Post one
            // correction after default processing, not a recursive SetWindowPos.
            keep_at_desktop(hwnd).log_error();
        }
        if msg == WM_TIMER && wparam.0 == DESKTOP_GUARD {
            if GUARD_TICKS.fetch_add(1, Ordering::Relaxed) == 0 {
                log::debug!(
                    "Desktop z-order guard timer active: hwnd={}",
                    hwnd.0 as isize
                );
            }
            keep_at_desktop(hwnd).log_error();
            return LRESULT(0);
        }
        if msg == *RESTACK_MESSAGE && *RESTACK_MESSAGE != 0 {
            let handle = hwnd.0 as isize;
            if DESKTOP_WINDOW.load(Ordering::Acquire) == handle {
                // Keep PENDING set through our own synchronous position messages
                // so a correction never schedules an endless correction loop.
                restack_desktop(hwnd).log_error();
            }
            let _ =
                RESTACK_PENDING.compare_exchange(handle, 0, Ordering::AcqRel, Ordering::Acquire);
            return LRESULT(0);
        }
        if msg == WM_NCDESTROY {
            let _ = KillTimer(Some(hwnd), DESKTOP_GUARD);
            let _ = RESTACK_PENDING.compare_exchange(
                hwnd.0 as isize,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

fn migrate_layout(data_dir: &Path) -> Result<()> {
    let source = data_dir.join("seelen-wallpaper-manager/desktop-shell.json");
    let destination = data_dir.join("seelen-desktop-shell/desktop-shell.json");
    if destination.exists() || !source.exists() {
        return Ok(());
    }
    let content = std::fs::read(&source)?;
    // Never silently discard a corrupt layout or overwrite a newer desktop layout.
    serde_json::from_slice::<serde_json::Value>(&content)?;
    std::fs::create_dir_all(
        destination
            .parent()
            .ok_or("Missing desktop data directory")?,
    )?;
    let backup = source.with_file_name("desktop-shell.before-independent-desktop.json");
    if !backup.exists() {
        atomic_write_file(&backup, &content)?;
    }
    atomic_write_file(&destination, &content)?;
    Ok(())
}

#[tauri::command(async)]
pub async fn set_as_desktop(webview: tauri::WebviewWindow) -> Result<()> {
    let label = super::webview::WidgetWebviewLabel::try_from_raw(webview.label())?;
    if label.widget_id.as_str() != "@seelen/desktop-shell" {
        return Err("Only Desktop Manager can own the interactive desktop".into());
    }
    migrate_layout(&SEELEN_COMMON.app_data_dir().join("data"))?;
    subscribe_to_display_recovery();
    subscribe_to_window_order_changes();
    let handle = webview.hwnd()?.0 as isize;
    let (tx, rx) = tokio::sync::oneshot::channel();
    webview.run_on_main_thread(move || {
        let result = (|| -> Result<()> {
            let hwnd = HWND(handle as _);
            unsafe {
                // Already-top-level desktop windows must not be reparented on
                // every monitor update (and NULL previous-parent is ambiguous).
                if WindowsApi::get_styles(hwnd).contains(WS_CHILDWINDOW) {
                    SetParent(hwnd, None)?;
                }
                let style = (WindowsApi::get_styles(hwnd) & !WS_CHILDWINDOW) | WS_POPUP;
                SetWindowLongPtrW(hwnd, GWL_STYLE, style.0 as isize);
                let ex_style = WindowsApi::get_ex_styles(hwnd)
                    & !(WS_EX_APPWINDOW | WS_EX_WINDOWEDGE | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style.0 as isize);
                if *RESTACK_MESSAGE == 0 {
                    return Err("Could not register desktop z-order guard message".into());
                }
                if !GetWindowSubclass(hwnd, Some(desktop_window_proc), DESKTOP_GUARD, None)
                    .as_bool()
                {
                    SetWindowSubclass(hwnd, Some(desktop_window_proc), DESKTOP_GUARD, 0).ok()?;
                    if SetTimer(Some(hwnd), DESKTOP_GUARD, 1500, None) == 0 {
                        return Err("Could not create desktop z-order guard timer".into());
                    }
                    log::debug!("Installed desktop z-order guard: hwnd={handle}");
                }
            }
            if DESKTOP_WINDOW.swap(handle, Ordering::AcqRel) != handle {
                *REGIONS.lock() = None;
                RESTACK_PENDING.store(0, Ordering::Release);
                GUARD_TICKS.store(0, Ordering::Release);
                REPAIR_FAILURE_LOGGED.store(false, Ordering::Release);
            }
            WindowsApi::set_position(
                hwnd,
                None,
                &desktop_rect()?,
                SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
            apply_desktop_regions(hwnd)?;
            keep_at_desktop(hwnd)?;
            Ok(())
        })();
        let _ = tx.send(result);
    })?;
    rx.await.map_err(|err| err.to_string())?
}
