use std::{
    io::Write,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
    thread::JoinHandle,
    time::Instant,
};

use windows::{
    Win32::{
        Foundation::{FILETIME, HANDLE, HMODULE, HWND, LPARAM, LRESULT, WAIT_TIMEOUT, WPARAM},
        System::{
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            RemoteDesktop::ProcessIdToSessionId,
            SystemInformation::GetWindowsDirectoryW,
            Threading::{
                GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
                WaitForSingleObject,
            },
        },
        UI::WindowsAndMessaging::{GetShellWindow, HHOOK, SetWindowsHookExW, WH_CALLWNDPROC},
    },
    core::{PCWSTR, PWSTR},
};
use windows_core::Owned;

use super::{
    hook_lifecycle::{PROBE_INTERVAL, ReconnectPolicy},
    util::Util,
};
use crate::{
    error::{Result, ResultLogExt},
    utils::constants::MACUI_COMMON,
    windows_api::WindowsApi,
};

type GetMsgProcFn = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

/// Tray-local identity check. Does not participate in startup, permissions,
/// installation ownership or desktop recovery.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ExplorerProcessIdentity {
    pid: u32,
    created: u64,
    session: u32,
    image: String,
}

impl ExplorerProcessIdentity {
    fn from_handle(pid: u32, handle: HANDLE) -> Result<Self> {
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user)? };
        let mut image = vec![0u16; 32_768];
        let mut length = image.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(image.as_mut_ptr()),
                &mut length,
            )?
        };
        let mut session = 0;
        unsafe { ProcessIdToSessionId(pid, &mut session)? };
        let image = PathBuf::from(std::ffi::OsString::from_wide(&image[..length as usize]))
            .canonicalize()?
            .to_string_lossy()
            .to_lowercase();
        Ok(Self {
            pid,
            created: ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64,
            session,
            image,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ShellIdentity {
    process: ExplorerProcessIdentity,
    window: isize,
    thread: u32,
}

struct ShellTarget {
    identity: ShellIdentity,
    // Retaining this exact kernel handle prevents a recycled PID from becoming
    // permission to attach to an unrelated process during DLL loading.
    process: Owned<HANDLE>,
}

impl ShellTarget {
    fn current() -> Result<Self> {
        let window = WindowsApi::find_window(None, None, None, Some("Shell_TrayWnd"))?;
        let (pid, thread) = WindowsApi::window_thread_process_id(window);
        if pid == 0 || thread == 0 {
            return Err("Windows tray thread is unavailable; no hook installed".into());
        }
        let process = unsafe {
            Owned::new(OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )?)
        };
        let identity = ExplorerProcessIdentity::from_handle(pid, *process)?;
        let mut directory = vec![0u16; 32_768];
        let length = unsafe { GetWindowsDirectoryW(Some(&mut directory)) } as usize;
        if length == 0 || length >= directory.len() {
            return Err("Windows directory is unavailable; no tray hook installed".into());
        }
        let expected = PathBuf::from(std::ffi::OsString::from_wide(&directory[..length]))
            .join("explorer.exe")
            .canonicalize()?;
        if identity.image != expected.to_string_lossy().to_lowercase()
            || identity.session == 0
            || identity.session != WindowsApi::current_session_id()
        {
            return Err("Tray owner is not this session's Windows Explorer".into());
        }
        let target = Self {
            identity: ShellIdentity {
                process: identity,
                window: window.0 as isize,
                thread,
            },
            process,
        };
        target.ensure_current()?;
        Ok(target)
    }

    fn ensure_current(&self) -> Result<()> {
        let window = HWND(self.identity.window as _);
        let (pid, thread) = WindowsApi::window_thread_process_id(window);
        let shell = unsafe { GetShellWindow() };
        let (shell_pid, _) = WindowsApi::window_thread_process_id(shell);
        if unsafe { WaitForSingleObject(*self.process, 0) } != WAIT_TIMEOUT
            || shell.is_invalid()
            || pid != self.identity.process.pid
            || shell_pid != pid
            || thread == 0
            || thread != self.identity.thread
            || WindowsApi::get_class(window)? != "Shell_TrayWnd"
        {
            return Err("Windows tray identity changed; no hook installed".into());
        }
        Ok(())
    }
}

/// Only the worker owns native hook handles. Drop wakes its bounded wait,
/// joins the worker and unloads the hook before releasing its DLL reference.
pub struct TrayHookLoader {
    stop: Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl TrayHookLoader {
    pub fn new(reset_snapshot: fn()) -> Result<Self> {
        let (stop, receiver) = channel();
        let worker = std::thread::Builder::new()
            .name("TrayHookLifecycle".into())
            .spawn(move || supervise(receiver, reset_snapshot))?;
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for TrayHookLoader {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn supervise(stop: Receiver<()>, reset_snapshot: fn()) {
    let mut policy = ReconnectPolicy::new();
    let mut installed: Option<InstalledTrayHook> = None;
    let started = Instant::now();
    let mut probe_failed = false;
    loop {
        if !matches!(stop.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)) {
            break;
        }
        let target = match ShellTarget::current() {
            Ok(target) => {
                probe_failed = false;
                Some(target)
            }
            Err(error) => {
                if !probe_failed {
                    log::warn!(
                        "Windows tray unavailable; waiting for a verified Explorer: {error}"
                    );
                }
                probe_failed = true;
                None
            }
        };
        let decision = policy.observe(
            target.as_ref().map(|value| value.identity.clone()),
            started.elapsed(),
        );
        if decision.detach {
            // Unhook and release the old DLL before attempting any replacement.
            drop(installed.take());
            reset_snapshot();
        }
        if decision.attach {
            if let Some(target) = target {
                match InstalledTrayHook::new(target) {
                    Ok(hook) => {
                        installed = Some(hook);
                        policy.connected();
                        reset_snapshot();
                        Util::refresh_icons().log_error();
                        log::info!(
                            "Verified Explorer tray hook connected; native snapshot requested"
                        );
                    }
                    Err(error) => {
                        let delay = policy.failed(started.elapsed());
                        log::warn!(
                            "Tray hook unavailable; retry in {}s: {error}",
                            delay.as_secs()
                        );
                    }
                }
            }
        }
        match stop.recv_timeout(PROBE_INTERVAL) {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            _ => break,
        }
    }
    drop(installed);
}

struct InstalledTrayHook {
    // Declaration order: uninstall the hook, then free our DLL reference.
    _hook_handle: Owned<HHOOK>,
    _dll_handle: Owned<HMODULE>,
    _target: ShellTarget,
}

impl InstalledTrayHook {
    fn new(target: ShellTarget) -> Result<Self> {
        let dll_path = Self::get_dll_path()?;
        let dll_handle = Self::load_dll(&dll_path)?;
        let procedure = Self::get_proc_address::<GetMsgProcFn>(*dll_handle, "CallWndProc")?;
        target.ensure_current()?;
        // Never use thread 0 (global injection). This is strictly the existing
        // Windows Explorer tray thread, not third-party application threads.
        let hook_handle = unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(procedure),
                Some((*dll_handle).into()),
                target.identity.thread,
            )?
        };
        let installed = Self {
            _hook_handle: unsafe { Owned::new(hook_handle) },
            _dll_handle: dll_handle,
            _target: target,
        };
        installed._target.ensure_current()?;
        Ok(installed)
    }

    fn get_dll_path() -> Result<PathBuf> {
        let exe_path = std::env::current_exe()?;
        let source = exe_path
            .parent()
            .ok_or("Missing executable directory")?
            .join("macui-hook.dll");
        let content = std::fs::read(&source)?;
        let digest = macui_utils::checksums::calculate_sha256(&content);
        let temp_dir = MACUI_COMMON.app_temp_dir();
        std::fs::create_dir_all(temp_dir)?;
        // Never overwrite a DLL still loaded in an exiting Explorer. Identical
        // verified content can be reused after transient attachment failures.
        let target = temp_dir.join(format!("macui-hook-{digest}.dll"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&content) {
                    drop(file);
                    // This exact cache file was just created by us, not supplied
                    // by the user. Do not leave a truncated DLL for later retries.
                    let _ = std::fs::remove_file(&target);
                    return Err(error.into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::metadata(&target)?.len() != content.len() as u64
                    || std::fs::read(&target)? != content
                {
                    return Err("Cached tray hook content differs from this installation".into());
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(target)
    }

    fn load_dll(path: &Path) -> Result<Owned<HMODULE>> {
        let wide: Vec<u16> = path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { Ok(Owned::new(LoadLibraryW(PCWSTR(wide.as_ptr()))?)) }
    }

    fn get_proc_address<F>(dll_handle: HMODULE, name: &str) -> Result<F> {
        let name = std::ffi::CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            let address = GetProcAddress(dll_handle, windows::core::PCSTR(name.as_ptr() as _))
                .ok_or("Tray hook export is missing")?;
            Ok(std::mem::transmute_copy(&address))
        }
    }
}
