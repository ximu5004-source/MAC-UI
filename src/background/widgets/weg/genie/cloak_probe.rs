//! Explicit, synthetic-window-only integration probe. Never used by the app.
//! A passing probe establishes API/state compatibility, not visual acceptance.

use std::{
    io::{BufRead, BufReader, Write},
    mem::size_of,
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{Once, mpsc},
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Dwm::{
            DWM_CLOAKED_APP, DWMWA_CLOAK, DWMWA_CLOAKED, DwmGetWindowAttribute,
            DwmSetWindowAttribute,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
            GetSystemMetrics, GetWindowPlacement, GetWindowThreadProcessId, IsIconic, IsWindow,
            IsWindowVisible, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SM_CXVIRTUALSCREEN,
            SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_MINIMIZE, SW_SHOWNOACTIVATE, ShowWindow,
            ShowWindowAsync, TranslateMessage, WINDOWPLACEMENT, WNDCLASSW, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_OVERLAPPEDWINDOW,
        },
    },
    core::{Result, w},
};

use crate::windows_api::WindowsApi;

const WAIT: Duration = Duration::from_millis(700);
const HELPER_LIFETIME: Duration = Duration::from_secs(10);
const READY: &str = "MAC_UI_OWNED_CLOAK_READY ";
const HELPER_TOKEN: &str = "MAC_UI_OWNED_CLOAK_HELPER_TOKEN";

unsafe extern "system" fn probe_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Only this constructor can create a locally owned probe window. Off-screen,
/// tool-window and NOACTIVATE prevent taskbar entries and foreground changes.
struct OwnedWindow(HWND);

impl OwnedWindow {
    fn new() -> Self {
        static REGISTER: Once = Once::new();
        let instance = WindowsApi::module_handle_w().unwrap().into();
        REGISTER.call_once(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    hInstance: instance,
                    lpfnWndProc: Some(probe_window_proc),
                    lpszClassName: w!("MAC UI owned cloak integration probe"),
                    ..Default::default()
                }),
                0
            );
        });
        let hwnd = unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN)
                .saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN))
                .saturating_add(1000);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN).saturating_add(1000);
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("MAC UI owned cloak integration probe"),
                w!("MAC UI synthetic cloak probe only"),
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
        let owned = Self(hwnd);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        pump();
        assert_eq!(owner(hwnd), std::process::id());
        assert_ne!(unsafe { GetForegroundWindow() }, hwnd);
        owned
    }
}

impl Drop for OwnedWindow {
    fn drop(&mut self) {
        // Always remove our own cloak before destruction, even after an assert.
        // Process termination also destroys its own HWND if unwinding fails.
        unsafe {
            let _ = set_cloak(self.0, false);
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
    let started = Instant::now();
    loop {
        pump();
        if ready() {
            return true;
        }
        if started.elapsed() >= WAIT {
            return false;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

fn cloaked(hwnd: HWND) -> Result<u32> {
    let mut value = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut value as *mut u32).cast(),
            size_of::<u32>() as u32,
        )?;
    }
    Ok(value)
}

unsafe fn set_cloak(hwnd: HWND, enabled: bool) -> Result<()> {
    let value = windows_core::BOOL::from(enabled);
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            (&value as *const windows_core::BOOL).cast(),
            size_of::<windows_core::BOOL>() as u32,
        )
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

/// Foreign targets must have been created by the exact helper we spawned.
/// The helper owns cleanup and has a deadline independent of the parent.
fn exercise(hwnd: HWND, expected_owner: u32, label: &str) -> bool {
    assert_ne!(expected_owner, 0);
    assert_eq!(owner(hwnd), expected_owner);
    let set = |enabled| {
        // Recheck before every mutation: an expired helper's HWND must never
        // be treated as an unrelated window reusing the handle.
        assert_eq!(owner(hwnd), expected_owner);
        unsafe { set_cloak(hwnd, enabled) }
    };
    assert!(unsafe { IsWindow(Some(hwnd)).as_bool() });
    assert_eq!(
        cloaked(hwnd).unwrap(),
        0,
        "only initially uncloaked synthetic windows are eligible"
    );
    assert!(!unsafe { IsIconic(hwnd).as_bool() });
    assert!(unsafe { IsWindowVisible(hwnd).as_bool() });
    let before = placement(hwnd);

    if let Err(error) = set(true) {
        assert_eq!(cloaked(hwnd).unwrap(), 0);
        assert_eq!(placement(hwnd), before);
        println!(
            "{label}: DWMWA_CLOAK unsupported, HRESULT={:?}; state unchanged",
            error.code()
        );
        return false;
    }
    assert!(wait_until(|| cloaked(hwnd).ok() == Some(DWM_CLOAKED_APP)));
    // Cancellation before minimizing must restore exactly the old placement.
    set(false).unwrap();
    assert!(wait_until(|| cloaked(hwnd).ok() == Some(0)));
    assert_eq!(placement(hwnd), before);
    assert!(unsafe { IsWindowVisible(hwnd).as_bool() });
    assert!(!unsafe { IsIconic(hwnd).as_bool() });

    assert_eq!(owner(hwnd), expected_owner);
    set(true).unwrap();
    assert!(wait_until(|| cloaked(hwnd).ok() == Some(DWM_CLOAKED_APP)));
    // This is the unchanged native minimize command, not a hide substitution.
    unsafe {
        assert_eq!(owner(hwnd), expected_owner);
        let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
    }
    assert!(wait_until(|| unsafe { IsIconic(hwnd).as_bool() }));
    assert_eq!(cloaked(hwnd).unwrap(), DWM_CLOAKED_APP);
    let minimized = placement(hwnd);
    assert_eq!(
        rect_key(minimized.rcNormalPosition),
        rect_key(before.rcNormalPosition)
    );

    set(false).unwrap();
    assert!(wait_until(|| cloaked(hwnd).ok() == Some(0)));
    assert!(unsafe { IsIconic(hwnd).as_bool() });
    unsafe {
        // SW_RESTORE explicitly activates even with WS_EX_NOACTIVATE. Keep
        // this off-screen probe on the documented nonactivating normal path.
        assert_eq!(owner(hwnd), expected_owner);
        let _ = ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
    }
    assert!(wait_until(|| !unsafe { IsIconic(hwnd).as_bool() }));
    assert_eq!(
        rect_key(placement(hwnd).rcNormalPosition),
        rect_key(before.rcNormalPosition)
    );
    assert!(unsafe { IsWindowVisible(hwnd).as_bool() });
    assert_ne!(unsafe { GetForegroundWindow() }, hwnd);

    // Cancellation/restoration while a replacement animation would be running.
    set(true).unwrap();
    unsafe {
        assert_eq!(owner(hwnd), expected_owner);
        let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
    }
    assert!(wait_until(|| unsafe { IsIconic(hwnd).as_bool() }));
    unsafe {
        assert_eq!(owner(hwnd), expected_owner);
        let _ = ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
    }
    assert!(wait_until(|| !unsafe { IsIconic(hwnd).as_bool() }));
    set(false).unwrap();
    assert!(wait_until(|| cloaked(hwnd).ok() == Some(0)));
    assert_eq!(
        rect_key(placement(hwnd).rcNormalPosition),
        rect_key(before.rcNormalPosition)
    );
    assert!(unsafe { IsWindowVisible(hwnd).as_bool() });
    assert_ne!(unsafe { GetForegroundWindow() }, hwnd);
    println!(
        "{label}: cloak/minimize/uncloak/restore and cancellation passed; placement preserved; probe never foreground"
    );
    true
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
        // Exact spawned child only; its HWND is destroyed on process exit.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "explicit synthetic Win32 integration probe; creates only off-screen owned windows"]
fn owned_cloak_probe() {
    let owned = OwnedWindow::new();
    let local = exercise(owned.0, std::process::id(), "same-process owned top-level");
    drop(owned);

    let test_module = module_path!().split_once("::").unwrap().1;
    let mut helper = Helper(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("{test_module}::owned_cloak_helper"),
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(
                HELPER_TOKEN,
                format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ),
            )
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: independent test helper.
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
            }
        }
    });
    let hwnd = HWND(
        rx.recv_timeout(Duration::from_secs(3))
            .expect("owned helper handshake timed out")
            .unwrap() as _,
    );
    let foreign = exercise(hwnd, helper.0.id(), "cross-process self-spawned top-level");
    drop(helper);
    reader.join().unwrap();
    assert!(
        !unsafe { IsWindow(Some(hwnd)).as_bool() },
        "helper cleanup must destroy its own HWND"
    );
    println!(
        "CLOAK_PROBE_RESULT same_process={local} cross_process={foreign}; no production path changed"
    );
}

#[test]
#[ignore = "internal self-spawned helper; requires private invocation token"]
fn owned_cloak_helper() {
    if std::env::var_os(HELPER_TOKEN).is_none() {
        return;
    }
    let owned = OwnedWindow::new();
    println!("{READY}{}", owned.0.0 as isize);
    std::io::stdout().flush().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut command = String::new();
        let _ = std::io::stdin().read_line(&mut command);
        let _ = tx.send(()); // command, EOF or failure all request cleanup.
    });
    let started = Instant::now();
    while started.elapsed() < HELPER_LIFETIME {
        pump();
        if rx.try_recv().is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    drop(owned); // owning thread resets cloak and destroys HWND on every exit.
}
