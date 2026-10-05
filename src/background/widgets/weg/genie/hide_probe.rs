//! Explicit synthetic-window-only hide/minimize compatibility probe.
//! It cannot establish native compositor visual behaviour for third-party apps.
//! No DWM attribute, window region, foreign user HWND, capture, or preference.

use std::{
    io::{BufRead, BufReader, Write},
    mem::size_of,
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{
        Once,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
            GetSystemMetrics, GetWindowPlacement, GetWindowThreadProcessId, IsIconic, IsWindow,
            IsWindowVisible, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SHOW_WINDOW_CMD,
            SM_CXVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOWMINNOACTIVE,
            SW_SHOWNA, SW_SHOWNOACTIVATE, ShowWindow, ShowWindowAsync, TranslateMessage,
            WINDOWPLACEMENT, WM_ACTIVATE, WM_SHOWWINDOW, WM_SIZE, WM_WINDOWPOSCHANGED, WNDCLASSW,
            WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_OVERLAPPEDWINDOW,
        },
    },
    core::w,
};

use crate::windows_api::WindowsApi;

const WAIT: Duration = Duration::from_millis(700);
const HELPER_LIFETIME: Duration = Duration::from_secs(12);
const READY: &str = "MAC_UI_OWNED_HIDE_READY ";
const TOKEN: &str = "MAC_UI_OWNED_HIDE_HELPER_TOKEN";
static SHOW_MESSAGES: AtomicU32 = AtomicU32::new(0);
static HIDE_MESSAGES: AtomicU32 = AtomicU32::new(0);
static SIZE_MESSAGES: AtomicU32 = AtomicU32::new(0);
static POSITION_MESSAGES: AtomicU32 = AtomicU32::new(0);
static ACTIVATIONS: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SHOWWINDOW => {
            if wparam.0 == 0 {
                HIDE_MESSAGES.fetch_add(1, Ordering::Relaxed);
            } else {
                SHOW_MESSAGES.fetch_add(1, Ordering::Relaxed);
            }
        }
        WM_SIZE => {
            SIZE_MESSAGES.fetch_add(1, Ordering::Relaxed);
        }
        WM_WINDOWPOSCHANGED => {
            POSITION_MESSAGES.fetch_add(1, Ordering::Relaxed);
        }
        WM_ACTIVATE if wparam.0 & 0xffff != 0 => {
            ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

struct OwnedWindow(HWND);

impl OwnedWindow {
    fn new() -> Self {
        static REGISTER: Once = Once::new();
        let instance = WindowsApi::module_handle_w().unwrap().into();
        REGISTER.call_once(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    hInstance: instance,
                    lpfnWndProc: Some(window_proc),
                    lpszClassName: w!("MAC UI owned hide integration probe"),
                    ..Default::default()
                }),
                0
            );
        });
        let hwnd = unsafe {
            // Off-screen keeps this state-only probe away from user windows.
            // APPWINDOW makes it an ordinary taskbar-capable top-level rather
            // than a tool window; NOACTIVATE prevents stealing foreground.
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN)
                .saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN))
                .saturating_add(1000);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN).saturating_add(1000);
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_APPWINDOW,
                w!("MAC UI owned hide integration probe"),
                w!("MAC UI synthetic hide probe only"),
                WS_OVERLAPPEDWINDOW,
                x,
                y,
                320,
                240,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap()
        };
        let window = Self(hwnd);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        pump();
        assert_eq!(owner(hwnd), std::process::id());
        assert_ne!(unsafe { GetForegroundWindow() }, hwnd);
        window
    }
}

impl Drop for OwnedWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

fn owner(hwnd: HWND) -> u32 {
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process));
    }
    process
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

fn wait_until(mut ready: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    loop {
        pump();
        if ready() {
            return true;
        }
        if start.elapsed() >= WAIT {
            return false;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

fn placement(hwnd: HWND) -> WINDOWPLACEMENT {
    let mut state = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe {
        GetWindowPlacement(hwnd, &mut state).unwrap();
    }
    state
}

fn rect_key(rect: RECT) -> [i32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom]
}

fn state(hwnd: HWND) -> (bool, bool) {
    unsafe { (IsWindowVisible(hwnd).as_bool(), IsIconic(hwnd).as_bool()) }
}

fn mutate(hwnd: HWND, expected_owner: u32, command: SHOW_WINDOW_CMD, asynchronous: bool) {
    assert_ne!(expected_owner, 0);
    assert_eq!(
        owner(hwnd),
        expected_owner,
        "only exact owned/helper HWND may be changed"
    );
    assert!(unsafe { IsWindow(Some(hwnd)).as_bool() });
    let started = Instant::now();
    unsafe {
        if asynchronous {
            ShowWindowAsync(hwnd, command).unwrap();
        } else {
            // ShowWindow returns previous visibility, not success/failure.
            // Only the subsequent state observations determine success.
            let _previous_visibility = ShowWindow(hwnd, command);
        }
    }
    println!(
        "OWNED_HIDE_CALL async={asynchronous} command={} elapsed_us={}",
        command.0,
        started.elapsed().as_micros()
    );
}

fn exercise(hwnd: HWND, expected_owner: u32, label: &str, asynchronous: bool) {
    assert_eq!(owner(hwnd), expected_owner);
    assert_eq!(state(hwnd), (true, false));
    let before = placement(hwnd);
    let foreground = unsafe { GetForegroundWindow() };
    let normal = rect_key(before.rcNormalPosition);

    // Cancellation before the minimize is submitted: restore current normal
    // visibility using a nonactivating command, preserving initial placement.
    mutate(hwnd, expected_owner, SW_HIDE, asynchronous);
    assert!(wait_until(|| state(hwnd) == (false, false)));
    assert_eq!(rect_key(placement(hwnd).rcNormalPosition), normal);
    mutate(hwnd, expected_owner, SW_SHOWNA, asynchronous);
    assert!(wait_until(|| state(hwnd) == (true, false)));
    assert_eq!(rect_key(placement(hwnd).rcNormalPosition), normal);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);

    for iteration in 0..3 {
        mutate(hwnd, expected_owner, SW_HIDE, asynchronous);
        assert!(wait_until(|| state(hwnd) == (false, false)));
        let hidden_at = Instant::now();
        mutate(hwnd, expected_owner, SW_SHOWMINNOACTIVE, asynchronous);
        assert!(wait_until(|| state(hwnd) == (true, true)));
        let minimized = placement(hwnd);
        assert_eq!(rect_key(minimized.rcNormalPosition), normal);
        assert_eq!(unsafe { GetForegroundWindow() }, foreground);
        println!(
            "{label}: cycle={iteration} hidden_to_iconic_us={} state=visible/iconic normal_rect={normal:?}",
            hidden_at.elapsed().as_micros()
        );
        // Observe stable flags for 420ms, but do NOT call this a compositor
        // animation capture. IsIconic does not prove DWM pixels disappeared.
        let until = Instant::now() + Duration::from_millis(420);
        while Instant::now() < until {
            pump();
            assert_eq!(state(hwnd), (true, true));
            assert_eq!(owner(hwnd), expected_owner);
            assert_eq!(unsafe { GetForegroundWindow() }, foreground);
            std::thread::sleep(Duration::from_millis(4));
        }
        // Cancellation/restore while an independent own overlay would run.
        mutate(hwnd, expected_owner, SW_SHOWNOACTIVATE, asynchronous);
        assert!(wait_until(|| state(hwnd) == (true, false)));
        assert_eq!(rect_key(placement(hwnd).rcNormalPosition), normal);
        assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    }
    println!(
        "{label}: async={asynchronous} hide/cancel/minimize/nonactivating-normal-restore state checks passed; foreground unchanged; visual DWM equivalence UNMEASURED"
    );
}

struct Helper(Child);

impl Drop for Helper {
    fn drop(&mut self) {
        if let Some(mut input) = self.0.stdin.take() {
            let _ = writeln!(input, "cleanup");
        }
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            if self.0.try_wait().is_ok_and(|status| status.is_some()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Exact self-spawned helper only, never an existing user process.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "explicit state-only synthetic Win32 probe; creates off-screen self-owned windows"]
fn owned_hide_minimize_probe() {
    let owned = OwnedWindow::new();
    exercise(owned.0, std::process::id(), "same-process", false);
    exercise(owned.0, std::process::id(), "same-process", true);
    drop(owned);

    let test_module = module_path!().split_once("::").unwrap().1;
    let mut helper = Helper(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("{test_module}::owned_hide_minimize_helper"),
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(
                TOKEN,
                format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ),
            )
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW; all HWNDs still belong to the helper.
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let output = helper.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(output)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if let Some((_, value)) = line.split_once(READY) {
                let _ = tx.send(value.parse::<isize>());
            } else {
                println!("OWNED_HIDE_HELPER {line}");
            }
        }
    });
    let hwnd = HWND(
        rx.recv_timeout(Duration::from_secs(3))
            .expect("owned helper handshake timed out")
            .unwrap() as _,
    );
    exercise(hwnd, helper.0.id(), "cross-process self-spawned", false);
    exercise(hwnd, helper.0.id(), "cross-process self-spawned", true);
    drop(helper);
    reader.join().unwrap();
    assert!(
        !unsafe { IsWindow(Some(hwnd)).as_bool() },
        "self-spawned helper must destroy its own window"
    );
    println!(
        "HIDE_PROBE_RESULT state_compatibility=passed native_visual_dedup=unmeasured production_feature=disabled"
    );
}

#[test]
#[ignore = "internal self-spawned helper requiring private invocation token"]
fn owned_hide_minimize_helper() {
    if std::env::var_os(TOKEN).is_none() {
        return;
    }
    let owned = OwnedWindow::new();
    println!("{READY}{}", owned.0.0 as isize);
    std::io::stdout().flush().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut command = String::new();
        let _ = std::io::stdin().read_line(&mut command);
        let _ = tx.send(());
    });
    let start = Instant::now();
    while start.elapsed() < HELPER_LIFETIME {
        pump();
        if rx.try_recv().is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    println!(
        "messages show={} hide={} size={} position={} activations={}",
        SHOW_MESSAGES.load(Ordering::Relaxed),
        HIDE_MESSAGES.load(Ordering::Relaxed),
        SIZE_MESSAGES.load(Ordering::Relaxed),
        POSITION_MESSAGES.load(Ordering::Relaxed),
        ACTIVATIONS.load(Ordering::Relaxed)
    );
    drop(owned);
}
