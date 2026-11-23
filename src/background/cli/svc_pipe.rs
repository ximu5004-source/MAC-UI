use std::{
    path::PathBuf,
    sync::{
        LazyLock, Once,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use macui_ipc::{ServiceIpc, messages::SvcAction};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use windows::Win32::{
    System::TaskScheduler::{IExecAction2, ITaskService, TaskScheduler},
    UI::Shell::FOLDERID_LocalAppData,
};
use windows_core::Interface;

use crate::{
    app::get_app_handle,
    error::Result,
    get_tokio_handle,
    utils::was_installed_using_msix,
    windows_api::{Com, WindowsApi},
};

pub struct ServicePipe;

const RECOVERY_BACKOFF: Duration = Duration::from_secs(30);
static STOPPING: AtomicBool = AtomicBool::new(false);
static RECOVERING: AtomicBool = AtomicBool::new(false);
static LAST_RECOVERY: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);
static START_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static STATE_SEND_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static DESIRED: LazyLock<parking_lot::Mutex<DesiredState>> = LazyLock::new(Default::default);

#[derive(Default)]
struct DesiredState {
    revision: u64,
    entries: [Option<(u64, SvcAction)>; 3],
}
fn state_slot(message: &SvcAction) -> Option<usize> {
    match message {
        SvcAction::HideNativeTaskbar | SvcAction::RestoreNativeTaskbar => Some(0),
        SvcAction::SetShortcuts(_) => Some(1),
        SvcAction::SetStartup(_) => Some(2),
        _ => None,
    }
}
impl DesiredState {
    fn record(&mut self, message: &SvcAction) -> Option<(usize, u64)> {
        let slot = state_slot(message)?;
        self.revision = self.revision.wrapping_add(1);
        self.entries[slot] = Some((self.revision, message.clone()));
        Some((slot, self.revision))
    }
    fn current(&self, ticket: (usize, u64)) -> bool {
        self.entries[ticket.0]
            .as_ref()
            .is_some_and(|(revision, _)| *revision == ticket.1)
    }
}
fn transport_failure(error: &macui_ipc::error::Error) -> bool {
    matches!(
        error,
        macui_ipc::error::Error::Io(_)
            | macui_ipc::error::Error::Timeout(_)
            | macui_ipc::error::Error::Windows(_)
    )
}
struct RecoveringGuard;
impl Drop for RecoveringGuard {
    fn drop(&mut self) {
        RECOVERING.store(false, Ordering::Release);
    }
}

impl ServicePipe {
    /// will ignore any response
    pub fn request(message: SvcAction) -> Result<()> {
        let ticket = DESIRED.lock().record(&message);
        get_tokio_handle().spawn(async move {
            if let Err(err) = Self::send_current(message.clone(), ticket).await {
                if transport_failure(&err) && !matches!(message, SvcAction::Stop) {
                    Self::schedule_recovery();
                    log::debug!("Service connection unavailable; recovery requested: {err}; action={message:?}");
                } else {
                    log::error!("Service rejected request: {err}; action={message:?}");
                }
            }
        });
        Ok(())
    }

    pub fn request_sync(message: SvcAction) -> Result<()> {
        match ServiceIpc::send_blocking(message.clone()).and_then(|response| response.ok()) {
            Ok(()) => Ok(()),
            Err(error) => {
                if transport_failure(&error) && !matches!(message, SvcAction::Stop) {
                    Self::schedule_recovery();
                }
                Err(error.into())
            }
        }
    }

    async fn send_current(
        message: SvcAction,
        ticket: Option<(usize, u64)>,
    ) -> macui_ipc::error::Result<()> {
        if let Some(ticket) = ticket {
            let _sending = STATE_SEND_LOCK.lock().await;
            if STOPPING.load(Ordering::Acquire) || !DESIRED.lock().current(ticket) {
                return Ok(());
            }
            ServiceIpc::send(message).await
        } else {
            ServiceIpc::send(message).await
        }
    }

    pub fn begin_shutdown() {
        STOPPING.store(true, Ordering::Release);
    }
    pub fn cancel_shutdown() {
        STOPPING.store(false, Ordering::Release);
    }
    pub async fn wait_for_recovery_pause() {
        let _start = START_LOCK.lock().await;
    }

    pub fn start_health_monitor() {
        static STARTED: Once = Once::new();
        STARTED.call_once(|| {
            get_tokio_handle().spawn(async {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if STOPPING.load(Ordering::Acquire) {
                        continue;
                    }
                    if let Err(error) = ServiceIpc::ping().await
                        && transport_failure(&error)
                    {
                        Self::schedule_recovery();
                    }
                }
            });
        });
    }

    fn schedule_recovery() {
        if STOPPING.load(Ordering::Acquire)
            || RECOVERING
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return;
        }
        get_tokio_handle().spawn(async {
            let _recovering = RecoveringGuard;
            let _start = START_LOCK.lock().await;
            if STOPPING.load(Ordering::Acquire) { return; }
            {
                let mut last = LAST_RECOVERY.lock();
                if last.is_some_and(|last| last.elapsed() < RECOVERY_BACKOFF) { return; }
                *last = Some(Instant::now());
            }
            if ServiceIpc::ping().await.is_err() {
                log::warn!("MAC UI Service disconnected; restarting this installation's scheduled helper");
                let launched = tokio::task::spawn_blocking(Self::try_start_from_task_scheduler).await;
                if !matches!(launched, Ok(Ok(()))) {
                    log::warn!("Scheduled service recovery could not launch; no elevation prompt will be repeated: {launched:?}");
                    return;
                }
                if !Self::wait_ready(Duration::from_secs(10)).await {
                    log::warn!("Scheduled service recovery did not reach IPC readiness; will retry after backoff");
                    return;
                }
            }
            if STOPPING.load(Ordering::Acquire) { return; }
            let desired: Vec<_> = DESIRED.lock().entries.iter().enumerate().filter_map(|(slot, state)| {
                state.as_ref().map(|(revision, message)| ((slot, *revision), message.clone()))
            }).collect();
            for (ticket, message) in desired {
                if STOPPING.load(Ordering::Acquire) { return; }
                if let Err(error) = Self::send_current(message, Some(ticket)).await {
                    log::warn!("Could not restore current helper state after reconnect: {error}");
                }
            }
            crate::widgets::weg::genie::service_recovered();
            log::info!("MAC UI Service IPC recovered; current-state resynchronization finished");
        });
    }

    async fn wait_ready(timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if STOPPING.load(Ordering::Acquire) {
                return false;
            }
            if matches!(
                tokio::time::timeout_at(deadline, ServiceIpc::ping()).await,
                Ok(Ok(()))
            ) {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// A second launcher has no Tauri AppHandle yet. It can wake the installed
    /// scheduled helper silently, without dialogs or terminating the existing UI.
    pub async fn recover_for_existing_instance() -> Result<()> {
        let _start = START_LOCK.lock().await;
        if ServiceIpc::ping().await.is_ok() {
            return Ok(());
        }
        tokio::task::spawn_blocking(Self::try_start_from_task_scheduler).await??;
        if Self::wait_ready(Duration::from_secs(10)).await {
            Ok(())
        } else {
            Err("Existing MAC UI helper did not reach IPC readiness".into())
        }
    }

    pub fn service_path() -> Result<PathBuf> {
        let service_path = if was_installed_using_msix() {
            WindowsApi::known_folder(FOLDERID_LocalAppData)?
                .join("Microsoft\\WindowsApps\\macui-service.exe")
        } else {
            std::env::current_exe()?.with_file_name("macui-service.exe")
        };
        Ok(service_path)
    }

    fn try_start_from_task_scheduler() -> Result<()> {
        Com::run_with_context(|| unsafe {
            let task_service: ITaskService = Com::create_instance(&TaskScheduler)?;
            task_service.Connect(
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )?;
            let folder = task_service.GetFolder(&"\\MAC UI".into())?;
            let task = folder.GetTask(&"MAC UI Service".into())?;

            let actions = task.Definition()?.Actions()?;
            // ask to microsoft what that hell this start counting from 1 instead 0
            let action: IExecAction2 = actions.get_Item(1)?.cast()?;
            let mut action_path = windows_core::BSTR::new();
            let mut action_arguments = windows_core::BSTR::new();
            action.Path(&mut action_path)?;
            action.Arguments(&mut action_arguments)?;
            let mut count = 0;
            actions.Count(&mut count)?;

            let service_path = Self::service_path()?.to_string_lossy().to_lowercase();
            match count == 1
                && action_path.to_string().to_lowercase() == service_path
                && matches!(action_arguments.to_string().trim(), "" | "--startup")
            {
                true => {
                    if STOPPING.load(Ordering::Acquire) {
                        return Err("Service recovery paused for shutdown".into());
                    }
                    task.Run(&Default::default())?;
                    Ok(())
                }
                false => {
                    Err("Service task action does not match this installation's helper".into())
                }
            }
        })
    }

    // the service should be running since installer will start it or startup task scheduler
    // so if the service is not running, we need to start it (common on msix setup)
    pub async fn start_service() -> Result<()> {
        let _start = START_LOCK.lock().await;
        if ServiceIpc::ping().await.is_ok() {
            return Ok(());
        }
        let task_result = tokio::task::spawn_blocking(Self::try_start_from_task_scheduler).await?;
        let ready = if task_result.is_ok() {
            Self::wait_ready(Duration::from_secs(10)).await
        } else {
            false
        };
        if !ready {
            log::warn!(
                "Scheduled helper did not become ready; trying elevated helper startup. Task result: {task_result:?}"
            );
            let service_path = Self::service_path()?;
            // A local debug build must leave the installed login task alone.
            // The service exposes this switch only in debug builds.
            let service_args = if cfg!(debug_assertions) {
                Some("--no-startup-task".to_string())
            } else {
                None
            };
            if let Err(err) = WindowsApi::execute(
                service_path.to_string_lossy().to_string(),
                service_args.clone(),
                None,
                true,
            ) {
                log::error!("First attempt to start service failed: {err}");
                let try_again = get_app_handle()
                    .dialog()
                    .message(t!("service.not_running_description"))
                    .title(t!("service.not_running"))
                    .kind(MessageDialogKind::Info)
                    .buttons(MessageDialogButtons::OkCustom(
                        t!("service.not_running_ok").to_string(),
                    ))
                    .blocking_show();

                if !try_again {
                    return Err(err);
                }
                // A successful user-approved retry must proceed to readiness,
                // not return the stale error from the cancelled first attempt.
                WindowsApi::execute(
                    service_path.to_string_lossy().to_string(),
                    service_args,
                    None,
                    true,
                )?;
            }
        }

        // Task Scheduler also returns before the process has initialized IPC.
        // Allow slow machines / antivirus scanning time without skipping readiness.
        if !Self::wait_ready(Duration::from_secs(10)).await {
            get_app_handle()
                .dialog()
                .message(t!("service.not_running_description"))
                .title(t!("service.not_running"))
                .kind(MessageDialogKind::Error)
                .buttons(MessageDialogButtons::Ok)
                .blocking_show();
            return Err("Service not running".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn recovery_retains_latest_desired_state_and_never_replays_window_commands_or_stop() {
        let mut state = DesiredState::default();
        let hide = state.record(&SvcAction::HideNativeTaskbar).unwrap();
        let restore = state.record(&SvcAction::RestoreNativeTaskbar).unwrap();
        assert!(!state.current(hide));
        assert!(state.current(restore));
        assert!(matches!(
            state.entries[0],
            Some((_, SvcAction::RestoreNativeTaskbar))
        ));
        assert!(state.record(&SvcAction::SetShortcuts(vec![])).is_some());
        assert!(state.record(&SvcAction::SetStartup(false)).is_some());
        for action in [
            SvcAction::Stop,
            SvcAction::SetForeground(1),
            SvcAction::ShowWindow {
                hwnd: 1,
                command: 6,
            },
            SvcAction::ShowWindowAsync {
                hwnd: 1,
                command: 9,
            },
            SvcAction::StartShortcutRegistration {
                request_id: uuid::Uuid::nil(),
            },
        ] {
            assert!(state.record(&action).is_none());
        }
        assert_eq!(
            state.entries.iter().filter(|entry| entry.is_some()).count(),
            3
        );
    }

    #[test]
    fn service_rejection_is_not_mistaken_for_a_missing_service() {
        use macui_ipc::error::Error;
        assert!(transport_failure(&Error::Io(
            std::io::Error::from_raw_os_error(2)
        )));
        assert!(transport_failure(&Error::Timeout("owned test".into())));
        assert!(!transport_failure(&Error::IpcResponse(
            "Unauthorized connection".into()
        )));
        assert!(!transport_failure(&Error::IpcResponse(
            "Windows HRESULT access denied".into()
        )));
    }
}
