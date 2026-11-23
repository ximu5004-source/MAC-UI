mod policy;

use std::time::Duration;

use macui_ipc::{IPC, ServiceIpc, messages::SvcAction};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use windows::Win32::{
    Foundation::{ERROR_INVALID_PARAMETER, WAIT_FAILED, WAIT_OBJECT_0},
    System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
};
use windows_core::Owned;

use crate::{error::Result, get_tokio_handle};
use policy::ExitGate;

static EXIT_GATE: ExitGate = ExitGate::new();
const SERVICE_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const SERVICE_EXIT_TIMEOUT_MS: u32 = 4_000;

/// Only explicit, successful application exits use this path. Closing the last
/// window and relaunching retain their existing Tauri behavior.
pub fn on_user_exit(app: &tauri::AppHandle, api: tauri::ExitRequestApi) {
    if EXIT_GATE.is_ready() {
        return;
    }
    api.prevent_exit();
    if !EXIT_GATE.begin() {
        return;
    }

    log::info!("Normal exit requested; stopping MAC UI Service before closing UI.");
    crate::cli::ServicePipe::begin_shutdown();
    let app = app.clone();
    get_tokio_handle().spawn(async move {
        match stop_service().await {
            Ok(()) => {
                EXIT_GATE.complete();
                app.exit(0);
            }
            Err(err) => {
                crate::cli::ServicePipe::cancel_shutdown();
                crate::widgets::native_host::resume_after_cancelled_exit();
                log::error!("Normal exit postponed: {err}");
                EXIT_GATE.retry();
                app.dialog()
                    .message(t!("service.exit_failed_description"))
                    .title(t!("service.exit_failed_title"))
                    .kind(MessageDialogKind::Error)
                    .buttons(MessageDialogButtons::Ok)
                    .show(|_| {});
            }
        }
    });
}

async fn stop_service() -> Result<()> {
    tokio::task::spawn_blocking(crate::widgets::native_host::stop).await??;
    // Let any already-running scheduler call finish before sending Stop. After
    // begin_shutdown, no new recovery may enter this critical section.
    crate::cli::ServicePipe::wait_for_recovery_pause().await;
    let pid = match tokio::time::timeout(SERVICE_PROBE_TIMEOUT, ServiceIpc::server_process_id())
        .await
    {
        Ok(Ok(pid)) => pid,
        // The helper may already have exited. An inaccessible or unresponsive
        // pipe must not be mistaken for an absent helper.
        Ok(Err(macui_ipc::error::Error::Io(err))) if matches!(err.raw_os_error(), Some(2 | 3)) => {
            return Ok(());
        }
        Ok(Err(err)) => return Err(err.into()),
        Err(_) => return Err("Timed out identifying MAC UI Service during exit".into()),
    };

    let runtime = get_tokio_handle().clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        // Keep this exact kernel process object, not a repeatedly queried PID.
        // SYNCHRONIZE only waits for exit; it cannot terminate or modify it.
        let process = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
            Ok(handle) => unsafe { Owned::new(handle) },
            Err(err) if err.code() == ERROR_INVALID_PARAMETER.to_hresult() => return Ok(()),
            Err(err) => return Err(err.into()),
        };

        let stop_result = runtime.block_on(async {
            tokio::time::timeout(STOP_REQUEST_TIMEOUT, ServiceIpc::send(SvcAction::Stop)).await
        });

        // An older helper can terminate after handling Stop but before flushing
        // its IPC reply. The process exiting is the authoritative completion.
        match unsafe { WaitForSingleObject(*process, SERVICE_EXIT_TIMEOUT_MS) } {
            WAIT_OBJECT_0 => {
                log::info!("MAC UI Service stopped; normal exit can continue.");
                Ok(())
            }
            WAIT_FAILED => Err(windows::core::Error::from_thread().into()),
            _ => Err(format!(
                "MAC UI Service did not exit within the shutdown timeout (request: {stop_result:?})"
            )
            .into()),
        }
    })
    .await?
}
