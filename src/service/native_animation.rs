//! A reversible minimize/restore preference lease, owned by an exact app process.
//! No profile/registry write is used. The service remains the custodian if the
//! UI crashes or is killed while its custom window animation is enabled.
use std::{mem::size_of, path::PathBuf, sync::LazyLock, time::Duration};

use parking_lot::Mutex;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            RemoteDesktop::ProcessIdToSessionId,
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW, WaitForSingleObject,
            },
        },
        UI::WindowsAndMessaging::{
            ANIMATIONINFO, SPI_GETANIMATION, SPI_SETANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
            SystemParametersInfoW,
        },
    },
    core::PWSTR,
};

use crate::{error::Result, windows_api::WindowsApi};

// Store the kernel handle as an integer so ownership can move between service
// workers; the mutex serializes access and Drop closes it exactly once.
struct AppProcess {
    pid: u32,
    handle: isize,
}
impl AppProcess {
    fn raw(&self) -> HANDLE {
        HANDLE(self.handle as *mut _)
    }
    fn alive(&self) -> bool {
        unsafe { WaitForSingleObject(self.raw(), 0) == WAIT_TIMEOUT }
    }
    fn exited(&self) -> bool {
        unsafe { WaitForSingleObject(self.raw(), 0) == WAIT_OBJECT_0 }
    }

    fn open(pid: u32) -> Result<Self> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            )?
        };
        let process = Self {
            pid,
            handle: handle.0 as isize,
        };
        if !process.alive() {
            return Err("Animation owner has already exited".into());
        }
        let mut session = 0;
        unsafe {
            ProcessIdToSessionId(pid, &mut session)?;
        }
        if session != WindowsApi::current_session_id() {
            return Err("Animation owner belongs to another session".into());
        }
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                process.raw(),
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )?;
        }
        let image = PathBuf::from(String::from_utf16_lossy(&path[..length as usize]));
        let service = std::env::current_exe()?;
        let installation = service
            .parent()
            .ok_or("Missing service installation directory")?;
        if !macui_utils::app_identity::is_app_executable_in(&image, installation) {
            return Err("Animation owner is not this installation's app".into());
        }
        Ok(process)
    }
}
impl Drop for AppProcess {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.raw()) };
    }
}

struct Lease {
    owner: AppProcess,
    previous: i32,
}
static LEASE: LazyLock<Mutex<Option<Lease>>> = LazyLock::new(|| Mutex::new(None));

fn read_animation() -> Result<i32> {
    let mut info = ANIMATIONINFO {
        cbSize: size_of::<ANIMATIONINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETANIMATION,
            info.cbSize,
            Some((&mut info as *mut ANIMATIONINFO).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
    }
    Ok(info.iMinAnimate)
}
fn write_animation(value: i32) -> Result<()> {
    let mut info = ANIMATIONINFO {
        cbSize: size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: value,
    };
    // Deliberately omit SPIF_UPDATEINIFILE and SPIF_SENDCHANGE. No synchronous
    // WM_SETTINGCHANGE is broadcast while the service's lease mutex is held.
    unsafe {
        SystemParametersInfoW(
            SPI_SETANIMATION,
            info.cbSize,
            Some((&mut info as *mut ANIMATIONINFO).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
    }
    Ok(())
}

fn restoration_value(previous: i32, current: i32) -> Option<i32> {
    // Respect an already-disabled preference and any later external change.
    (previous != 0 && current == 0).then_some(previous)
}
fn restore_locked(lease: &mut Option<Lease>) -> Result<()> {
    if let Some(saved) = lease.as_ref() {
        if let Some(value) = restoration_value(saved.previous, read_animation()?) {
            write_animation(value)?;
        }
        log::info!(
            "Released native window animation lease for process {}",
            saved.owner.pid
        );
        *lease = None;
    }
    Ok(())
}

fn ensure_watchdog() -> Result<()> {
    static WATCHDOG: LazyLock<std::result::Result<(), String>> = LazyLock::new(|| {
        std::thread::Builder::new()
            .name("Native animation custodian".into())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_millis(200));
                    let mut lease = LEASE.lock();
                    let exiting = crate::EXITING.load(std::sync::atomic::Ordering::Acquire);
                    if exiting || lease.as_ref().is_some_and(|saved| saved.owner.exited()) {
                        if let Err(error) = restore_locked(&mut lease) {
                            log::error!("Native window animation restoration failed: {error}");
                        }
                    }
                    if exiting {
                        break;
                    }
                }
            })
            .map(|_| ())
            .map_err(|error| error.to_string())
    });
    WATCHDOG
        .as_ref()
        .map_err(|error| error.clone().into())
        .copied()
}

pub fn set_suppressed(owner_pid: u32, suppressed: bool) -> Result<()> {
    if crate::EXITING.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Service is shutting down".into());
    }
    // Pipe transport also verifies owner_pid equals the connected client PID.
    // Check image/session against a held kernel handle, not a later PID lookup.
    let candidate = AppProcess::open(owner_pid)?;
    ensure_watchdog()?;
    let mut lease = LEASE.lock();
    if crate::EXITING.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Service is shutting down".into());
    }
    if let Some(saved) = lease.as_ref() {
        if saved.owner.exited() {
            restore_locked(&mut lease)?;
        }
    }
    if let Some(saved) = lease.as_ref() {
        if saved.owner.pid != owner_pid {
            return Err("Native animation is leased by another live app".into());
        }
        if !suppressed {
            restore_locked(&mut lease)?;
        }
        return Ok(());
    }
    if suppressed {
        let previous = read_animation()?;
        if previous != 0 {
            write_animation(0)?;
        }
        *lease = Some(Lease {
            owner: candidate,
            previous,
        });
        log::info!("Acquired session-only native window animation lease for process {owner_pid}");
    }
    Ok(())
}

pub fn restore() -> Result<()> {
    restore_locked(&mut LEASE.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restoration_preserves_the_actual_original_and_later_external_changes() {
        assert_eq!(restoration_value(1, 0), Some(1));
        assert_eq!(restoration_value(-1, 0), Some(-1));
        assert_eq!(restoration_value(0, 0), None);
        assert_eq!(restoration_value(0, 1), None);
        assert_eq!(restoration_value(1, 1), None);
        assert_eq!(restoration_value(1, 2), None);
    }

    #[test]
    fn a_service_test_process_cannot_claim_the_app_preference_lease() {
        assert!(AppProcess::open(std::process::id()).is_err());
        assert!(AppProcess::open(0).is_err());
    }

    #[test]
    #[ignore = "only launched by the explicit custodian probe; no UI or file actions"]
    fn animation_lease_child() {
        std::thread::sleep(Duration::from_millis(750));
    }

    #[test]
    #[ignore = "explicit reversible SPI/kernel-handle probe, temporarily suppresses session animation; always restores the original"]
    fn owned_process_exit_restores_the_session_preference() {
        use std::{
            os::windows::process::CommandExt,
            process::{Command, Stdio},
            time::Instant,
        };
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;

        let original = read_animation().unwrap();
        struct Restore(i32);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = write_animation(self.0);
            }
        }
        let restore = Restore(original);
        assert!(LEASE.lock().is_none());
        ensure_watchdog().unwrap();
        for previous in [0, 1] {
            write_animation(previous).unwrap();
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "native_animation::tests::animation_lease_child",
                    "--test-threads=1",
                ])
                .creation_flags(CREATE_NO_WINDOW.0)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let handle = unsafe {
                OpenProcess(
                    PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    child.id(),
                )
                .unwrap()
            };
            let owner = AppProcess {
                pid: child.id(),
                handle: handle.0 as isize,
            };
            assert!(owner.alive());
            // The probe supplies a self-owned inert child directly; production IPC
            // still requires its connected PID, same session and exact app image.
            {
                let mut lease = LEASE.lock();
                write_animation(0).unwrap();
                *lease = Some(Lease { owner, previous });
            }
            assert_eq!(read_animation().unwrap(), 0);
            assert!(child.wait().unwrap().success());
            let until = Instant::now() + Duration::from_secs(3);
            while LEASE.lock().is_some() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(25));
            }
            assert!(
                LEASE.lock().is_none(),
                "exact-process watchdog did not release the lease"
            );
            assert_eq!(read_animation().unwrap(), previous);
            println!(
                "NATIVE_ANIMATION_CUSTODIAN_PROBE original={previous} restored={previous} exact_process_exit=true"
            );
        }
        drop(restore);
        assert_eq!(read_animation().unwrap(), original);
        println!("User session preference restored to actual original={original}");
    }
}
