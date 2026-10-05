//! Opt-in visual QA source for the native host backdrop. Not part of production.
//! It paints only its own temporary opaque window and never captures the screen,
//! sends input, or changes the target popup's position, activation or settings.

use std::{
    cell::Cell,
    path::PathBuf,
    sync::Once,
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, HBRUSH, InvalidateRect,
            PAINTSTRUCT, UpdateWindow,
        },
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GW_HWNDNEXT, GW_HWNDPREV, GWL_EXSTYLE, GWLP_USERDATA, GetClientRect,
            GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
            GetWindowThreadProcessId, HTTRANSPARENT, IsIconic, IsWindow, IsWindowVisible,
            MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SWP_NOACTIVATE,
            SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, TranslateMessage, WM_ERASEBKGND,
            WM_MOUSEACTIVATE, WM_NCCREATE, WM_NCDESTROY, WM_NCHITTEST, WM_PAINT, WNDCLASSW,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::{Owned, PWSTR, w},
};

use crate::windows_api::WindowsApi;

const TARGET_ENV: &str = "MAC_UI_FROST_QA_POPUP_HWND";
const EXPECTED_IMAGE: &str = r"E:\换皮\Seelen-UI\target\debug\mac-ui.exe";
const LIFETIME: Duration = Duration::from_secs(90);
const PHASE: Duration = Duration::from_secs(5);
const TILE: i32 = 48;

fn allowed_target_title(value: &str) -> bool {
    matches!(
        value,
        "网络"
            | "網絡"
            | "Network"
            | "蓝牙"
            | "藍牙"
            | "Bluetooth"
            | "Bluetooth Popup"
            | "启动台"
            | "啟動台"
            | "Launchpad"
    )
}

struct Brush(HBRUSH);

impl Brush {
    fn rgb(r: u8, g: u8, b: u8) -> Self {
        let brush = unsafe {
            CreateSolidBrush(COLORREF(
                u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16,
            ))
        };
        assert!(!brush.is_invalid(), "Could not create owned QA brush");
        Self(brush)
    }
}

impl Drop for Brush {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

struct PaintState {
    brushes: [Brush; 4],
    phase: usize,
    failed: Cell<bool>,
}

impl PaintState {
    fn new() -> Self {
        Self {
            // The input alternates between color and high-contrast neutral.
            // These colors belong to the QA source, not to the glass material.
            brushes: [
                Brush::rgb(24, 96, 208),
                Brush::rgb(32, 192, 224),
                Brush::rgb(232, 232, 232),
                Brush::rgb(24, 24, 24),
            ],
            phase: 0,
            failed: Cell::new(false),
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            }
            LRESULT(1)
        }
        WM_NCDESTROY => {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const PaintState;
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            // Do not unwind through a native window procedure on GDI failure.
            if !dc.is_invalid() && !state.is_null() {
                let state = unsafe { &*state };
                let mut rect = RECT::default();
                if unsafe { GetClientRect(hwnd, &mut rect) }.is_ok() {
                    for y in (0..rect.bottom).step_by(TILE as usize) {
                        for x in (0..rect.right).step_by(TILE as usize) {
                            let index = state.phase * 2 + (((x / TILE) + (y / TILE)) & 1) as usize;
                            let tile = RECT {
                                left: x,
                                top: y,
                                right: (x + TILE).min(rect.right),
                                bottom: (y + TILE).min(rect.bottom),
                            };
                            if unsafe { FillRect(dc, &tile, state.brushes[index].0) } == 0 {
                                state.failed.set(true);
                            }
                        }
                    }
                } else {
                    state.failed.set(true);
                }
            } else if !state.is_null() {
                unsafe { &*state }.failed.set(true);
            }
            unsafe {
                let _ = EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn owner(hwnd: HWND) -> u32 {
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process));
    }
    process
}

fn title(hwnd: HWND) -> String {
    let mut text = [0u16; 128];
    let length = unsafe { GetWindowTextW(hwnd, &mut text) };
    String::from_utf16_lossy(&text[..length.max(0) as usize])
}

fn target_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect).unwrap();
    }
    rect
}

fn pump() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Destroy HWND before dropping the state pointer/brushes used by its WndProc.
struct Fixture {
    hwnd: HWND,
    state: Box<PaintState>,
}

impl Fixture {
    fn new(rect: RECT, popup: HWND) -> Self {
        static REGISTER: Once = Once::new();
        let instance = WindowsApi::module_handle_w().unwrap().into();
        REGISTER.call_once(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    hInstance: instance,
                    lpfnWndProc: Some(window_proc),
                    lpszClassName: w!("MAC UI isolated frosted QA source"),
                    ..Default::default()
                }),
                0
            );
        });
        let mut state = Box::new(PaintState::new());
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_TOPMOST,
                w!("MAC UI isolated frosted QA source"),
                w!("MAC UI temporary checkerboard QA source"),
                WS_POPUP,
                rect.left - 10,
                rect.top - 10,
                rect.right - rect.left + 20,
                rect.bottom - rect.top + 20,
                None,
                None,
                Some(instance),
                Some((&mut *state as *mut PaintState).cast()),
            )
            .unwrap()
        };
        let fixture = Self { hwnd, state };
        assert_eq!(owner(hwnd), std::process::id());
        // The ONLY positioning call targets our own HWND. The existing popup
        // is only a read-only z-order reference, never changed or activated.
        unsafe {
            SetWindowPos(
                hwnd,
                Some(popup),
                rect.left - 10,
                rect.top - 10,
                rect.right - rect.left + 20,
                rect.bottom - rect.top + 20,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .unwrap();
            assert!(InvalidateRect(Some(hwnd), None, false).as_bool());
            assert!(UpdateWindow(hwnd).as_bool());
        }
        assert!(
            !fixture.state.failed.get(),
            "Owned checkerboard painting failed"
        );
        assert_ne!(unsafe { GetForegroundWindow() }, hwnd);
        fixture
    }

    fn set_phase(&mut self, phase: usize) {
        self.state.phase = phase;
        unsafe {
            assert!(InvalidateRect(Some(self.hwnd), None, false).as_bool());
            assert!(UpdateWindow(self.hwnd).as_bool());
        }
        assert!(
            !self.state.failed.get(),
            "Owned checkerboard painting failed"
        );
    }

    fn report_z(&self, popup: HWND) {
        let previous = unsafe { GetWindow(self.hwnd, GW_HWNDPREV) }.unwrap_or_default();
        let next = unsafe { GetWindow(popup, GW_HWNDNEXT) }.unwrap_or_default();
        let topmost =
            unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) } & WS_EX_TOPMOST.0 as isize != 0;
        let rect = target_rect(self.hwnd);
        println!(
            "FROST_QA_Z source_previous={} previous_pid={} target_next={} next_pid={} source_topmost={topmost} immediate_behind={} own_rect={},{},{},{}",
            previous.0 as isize,
            owner(previous),
            next.0 as isize,
            owner(next),
            previous == popup && next == self.hwnd,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            // Detach the Box pointer before any native destruction messages;
            // failed destruction cannot leave a WndProc pointing at freed data.
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

#[test]
#[ignore = "explicit 90-second native glass visual fixture; owned background only"]
fn owned_checkerboard_behind_network_popup() {
    let address: isize = std::env::var(TARGET_ENV)
        .expect("MAC_UI_FROST_QA_POPUP_HWND must identify a current debug glass widget")
        .parse()
        .expect("target must be a decimal HWND");
    assert!(address > 0);
    let popup = HWND(address as _);
    assert!(unsafe { IsWindow(Some(popup)).as_bool() });
    assert!(unsafe { IsWindowVisible(popup).as_bool() });
    assert!(!unsafe { IsIconic(popup).as_bool() });
    assert!(
        allowed_target_title(&title(popup)),
        "target must be an explicitly allowed native glass widget"
    );
    let process_id = owner(popup);
    assert_ne!(process_id, 0);
    assert_ne!(process_id, std::process::id());

    // PROCESS_QUERY_LIMITED_INFORMATION is read-only. Owned closes the handle
    // on every error, assertion and ordinary exit.
    let process: Owned<HANDLE> = unsafe {
        Owned::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).unwrap())
    };
    let mut image = [0u16; 32768];
    let mut length = image.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            *process,
            PROCESS_NAME_WIN32,
            PWSTR(image.as_mut_ptr()),
            &mut length,
        )
        .unwrap();
    }
    let actual = PathBuf::from(String::from_utf16(&image[..length as usize]).unwrap())
        .canonicalize()
        .unwrap();
    let expected = PathBuf::from(EXPECTED_IMAGE).canonicalize().unwrap();
    assert!(
        actual
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy()),
        "only the exact current debug MAC UI image is eligible"
    );
    assert_eq!(owner(popup), process_id);
    let rect = target_rect(popup);
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    assert!(width > 0 && height > 0 && width <= 4096 && height <= 4096);
    assert!(
        rect.left >= i32::MIN + 10
            && rect.top >= i32::MIN + 10
            && rect.right <= i32::MAX - 10
            && rect.bottom <= i32::MAX - 10
    );

    let mut fixture = Fixture::new(rect, popup);
    fixture.report_z(popup);
    println!(
        "FROST_QA_READY own_hwnd={} source=color then neutral every5s; fixed90s deadline; target_hwnd={address}",
        fixture.hwnd.0 as isize
    );
    let started = Instant::now();
    let mut phase = 0;
    while started.elapsed() < LIFETIME {
        pump();
        assert!(
            !fixture.state.failed.get(),
            "Owned checkerboard painting failed"
        );
        // Stop instead of moving/following an existing window or surviving
        // target closure/handle reuse. All observations are read-only.
        if !unsafe { IsWindow(Some(popup)).as_bool() }
            || owner(popup) != process_id
            || !unsafe { IsWindowVisible(popup).as_bool() }
            || target_rect(popup) != rect
            || !allowed_target_title(&title(popup))
        {
            break;
        }
        assert_ne!(unsafe { GetForegroundWindow() }, fixture.hwnd);
        let next = (started.elapsed().as_secs() / PHASE.as_secs()) as usize % 2;
        if next != phase {
            fixture.set_phase(next);
            phase = next;
            println!("FROST_QA_PHASE {phase}");
            fixture.report_z(popup);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let owned_hwnd = fixture.hwnd;
    drop(fixture);
    assert!(!unsafe { IsWindow(Some(owned_hwnd)).as_bool() });
    println!("FROST_QA_CLEANED owned window destroyed; all4brushes dropped");
}
