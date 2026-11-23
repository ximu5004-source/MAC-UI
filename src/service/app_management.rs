use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use macui_ipc::{AppIpc, IPC};
use sysinfo::ProcessesToUpdate;

use crate::{enviroment::was_installed_using_msix, error::Result, exit};

/// How long to wait for the app to establish its IPC connection after each launch attempt.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);

/// How long to wait for the app to restore its IPC pipe after a system resume.
/// Tauri's async runtime can take several seconds to fully re-initialize.
const RESUME_GRACE: Duration = Duration::from_secs(10);

/// Max consecutive failures before giving up and stopping the service.
/// In debug builds we stop immediately after the first failure.
const MAX_CRASHES: u32 = if cfg!(debug_assertions) { 0 } else { 5 };

/// Set to `true` while the system is suspended; cleared on resume.
/// Written by the power monitor thread, read by the app monitoring thread.
pub static SYSTEM_SUSPENDED: AtomicBool = AtomicBool::new(false);

fn startup_deadline(app_just_launched: bool, now: Instant) -> Option<Instant> {
    app_just_launched.then(|| now + STARTUP_TIMEOUT)
}

fn within_grace(deadline: Option<Instant>, now: Instant) -> bool {
    deadline.is_some_and(|deadline| now < deadline)
}

pub fn on_system_suspend() {
    log::info!("System suspending — pausing app health checks.");
    SYSTEM_SUSPENDED.store(true, Ordering::SeqCst);
}

pub fn on_system_resume() {
    log::info!("System resuming — will restore app health checks after grace window.");
    SYSTEM_SUSPENDED.store(false, Ordering::SeqCst);
}

/// Starts monitoring the MAC UI app for the current session and restarts it automatically
/// when it crashes or fails to start.
///
/// `app_just_launched` should be `true` when the caller already launched the app so that the
/// monitor gives it a startup grace window before counting a failure.
pub fn start_app_monitoring(app_just_launched: bool) {
    std::thread::spawn(move || {
        let mut crash_count = 0u32;
        let mut app_was_connected = true;
        // Absolute deadline until which IPC failures are ignored (startup / resume grace).
        let mut grace_until = startup_deadline(app_just_launched, Instant::now());
        let mut was_suspended = false;

        loop {
            std::thread::sleep(Duration::from_secs(1));

            if crate::EXITING.load(Ordering::SeqCst) {
                return;
            }

            // ── Suspend / resume handling ─────────────────────────────────────────────

            if SYSTEM_SUSPENDED.load(Ordering::SeqCst) {
                was_suspended = true;
                continue;
            }

            if was_suspended {
                // First iteration after waking up: reset failure state and grant a generous
                // grace window so the app has time to restore its IPC pipe.
                was_suspended = false;
                crash_count = 0;
                app_was_connected = true;
                grace_until = Some(Instant::now() + RESUME_GRACE);
                log::info!(
                    "Resumed from sleep — crash counter reset, granting {}s grace window.",
                    RESUME_GRACE.as_secs()
                );
                continue;
            }

            // ── Normal health check ───────────────────────────────────────────────────

            if AppIpc::can_stablish_connection() {
                if !app_was_connected {
                    log::info!("MAC UI reconnected successfully, resetting crash counter.");
                    crash_count = 0;
                }
                // A successful connection ends the startup allowance at once.
                // A later crash retains normal immediate recovery behavior.
                grace_until = None;
                app_was_connected = true;
                continue;
            }

            // A stop can arrive while the IPC probe is waiting for a reply.
            if crate::EXITING.load(Ordering::SeqCst) {
                return;
            }

            // ── App is not reachable via IPC ──────────────────────────────────────────

            // If we are still within a grace window (post-launch or post-resume), do not
            // count the absence as a failure yet.
            if within_grace(grace_until, Instant::now()) {
                continue;
            }

            // Failure: either the app was connected and just disappeared (runtime crash),
            // or a grace window expired without the app establishing IPC.
            app_was_connected = false;
            crash_count += 1;
            log::warn!("MAC UI is not running (failure #{crash_count}/{MAX_CRASHES}).");

            if crash_count > MAX_CRASHES {
                log::error!("MAC UI failed {crash_count} times in a row, stopping service.");
                break;
            }

            crate::log_error!(kill_all_macui_processes());

            // Linear back-off: 500 ms, 1000 ms, 1500 ms, …
            let backoff = Duration::from_millis(500 * crash_count as u64);
            log::info!("Restarting MAC UI in {}ms…", backoff.as_millis());
            std::thread::sleep(backoff);
            // Do not relaunch a GUI after the user requested a normal exit.
            if crate::EXITING.load(Ordering::SeqCst) {
                return;
            }
            crate::log_error!(launch_macui());
            grace_until = Some(Instant::now() + STARTUP_TIMEOUT);
        }

        log::error!("MAC UI monitoring stopped after repeated failures.");
        exit(1);
    });
}

#[cfg(test)]
mod startup_grace_tests {
    use super::*;

    #[test]
    fn freshly_launched_app_gets_twenty_seconds_for_tauri_and_ipc_startup() {
        let now = Instant::now();
        let deadline = startup_deadline(true, now);
        assert_eq!(deadline, Some(now + Duration::from_secs(20)));
        assert!(within_grace(deadline, now + Duration::from_secs(3)));
        assert!(within_grace(deadline, now + Duration::from_millis(19_999)));
        assert!(!within_grace(deadline, now + Duration::from_secs(20)));
    }

    #[test]
    fn already_connected_app_has_no_initial_or_post_connect_crash_delay() {
        let now = Instant::now();
        assert_eq!(startup_deadline(false, now), None);
        assert!(!within_grace(None, now));
        assert!(!within_grace(None, now + Duration::from_secs(1)));
    }
}

/// will start the app on the interactive session
pub fn launch_macui() -> Result<()> {
    let path = if was_installed_using_msix() {
        format!(
            "shell:AppsFolder\\{}!App",
            windows::ApplicationModel::Package::Current()?
                .Id()?
                .FamilyName()?
        )
    } else {
        let executable = std::env::current_exe()?;
        macui_utils::app_identity::app_executable_in(
            executable
                .parent()
                .ok_or("Missing installation directory")?,
        )
        .to_string_lossy()
        .to_string()
    };
    // Use explorer.exe to spawn the app de-elevated (with the interactive user's token),
    // since the service runs elevated (TASK_RUNLEVEL_HIGHEST). ShellExecuteExW called
    // from an elevated process inherits the elevated token, causing an infinite restart loop.
    std::process::Command::new("explorer.exe")
        .arg(&path)
        .spawn()?;
    Ok(())
}

pub fn kill_all_macui_processes() -> Result<()> {
    log::info!("Killing all MAC UI processes in current session");
    let current_session = crate::windows_api::WindowsApi::current_session_id();
    let service_exe = std::env::current_exe()?;
    let installation = service_exe
        .parent()
        .ok_or("Missing installation directory")?;

    let mut sys = sysinfo::System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let instances: Vec<_> = sys
        .processes()
        .values()
        .filter(|p| {
            if !p.exe().is_some_and(|path| {
                macui_utils::app_identity::is_app_executable_in(path, installation)
            }) {
                return false;
            }
            let mut session_id = 0u32;
            let in_session = unsafe {
                windows::Win32::System::RemoteDesktop::ProcessIdToSessionId(
                    p.pid().as_u32(),
                    &mut session_id,
                )
                .is_ok()
            };
            in_session && session_id == current_session
        })
        .collect();
    for instance in instances {
        instance.kill();
    }
    Ok(())
}
