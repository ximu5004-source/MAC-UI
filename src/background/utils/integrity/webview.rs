use base64::Engine;
use tauri::webview_version;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_shell::ShellExt;

use crate::{app::get_app_handle, error::Result};

use super::IntegrityError;

pub fn validate_webview_runtime() -> std::result::Result<(), IntegrityError> {
    match webview_version() {
        Ok(version) => {
            let major: u32 = version
                .split('.')
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            if major < 110 {
                Err(IntegrityError::WebviewRuntimeOutdated)
            } else {
                Ok(())
            }
        }
        Err(_) => Err(IntegrityError::WebviewRuntimeNotInstalled),
    }
}

pub fn show_not_installed_dialog(app: &tauri::AppHandle) -> Result<()> {
    let ok_pressed = app
        .dialog()
        .message(t!("runtime.not_found_description"))
        .title(t!("runtime.not_found"))
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCustom(
            t!("runtime.download").to_string(),
        ))
        .blocking_show();
    if ok_pressed {
        open_webview2_download(app)?;
    }
    Ok(())
}

pub fn show_outdated_dialog(app: &tauri::AppHandle) -> Result<()> {
    let ok_pressed = app
        .dialog()
        .message(t!("runtime.outdated_description", min_version = "110"))
        .title(t!("runtime.outdated"))
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCustom(
            t!("runtime.download").to_string(),
        ))
        .blocking_show();
    if ok_pressed {
        open_webview2_download(app)?;
    }
    Ok(())
}

fn open_webview2_download(app: &tauri::AppHandle) -> Result<()> {
    let url = "https://developer.microsoft.com/en-us/microsoft-edge/webview2/?form=MA13LH#download";
    #[allow(deprecated)]
    app.shell().open(url, None)?;
    Ok(())
}

/// Try creating a webview window, tauri for some reason could panic stopping the setup hook and for some reason
/// the panic hook is not catching this so this implementation is a workaround for that.
pub async fn check_for_webview_optimal_state() -> bool {
    log::info!("Testing webview optimal state...");

    let (tx, rx) = tokio::sync::oneshot::channel();

    let worker = std::thread::Builder::new()
        .name("WebviewStartupProbe".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                let label =
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("@macui/integrity");
                let window = tauri::WebviewWindowBuilder::new(
                    get_app_handle(),
                    &label,
                    tauri::WebviewUrl::App("vanilla/integrity/index.html".into()),
                )
                .visible(false)
                .build()?;
                window.hwnd()?; // A usable native handle is required as well.
                window.destroy()?;
                Ok(())
            })();
            let _ = tx.send(result);
        });
    if let Err(error) = worker {
        log::error!("Could not start WebView startup probe: {error}");
        return false;
    }

    wait_for_webview_probe(rx, std::time::Duration::from_secs(3)).await
}

async fn wait_for_webview_probe(
    rx: tokio::sync::oneshot::Receiver<Result<()>>,
    timeout: std::time::Duration,
) -> bool {
    tokio::select! {
        result = rx => {
            match result {
                Ok(Ok(())) => {},
                Ok(Err(error)) => {
                    log::error!("WebView startup probe failed: {error}");
                    return false;
                }
                Err(error) => {
                    // A panic/early worker exit closes the channel. That is not
                    // a successful probe and must not be logged as confirmed.
                    log::error!("WebView startup probe ended without a result: {error}");
                    return false;
                }
            }
            log::info!("Webview optimal state confirmed.");
            true
        }
        _ = tokio::time::sleep(timeout) => {
            log::error!("Webview optimal state check timed out.");
            false
        }
    }
}

#[cfg(test)]
mod startup_probe_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn only_an_explicit_success_confirms_the_webview_probe() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tx.send(Ok(())).unwrap();
        assert!(wait_for_webview_probe(rx, Duration::from_secs(1)).await);
    }

    #[tokio::test]
    async fn probe_failure_and_missing_result_are_not_reported_as_success() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tx.send(Err("Native WebView creation failed".into()))
            .unwrap();
        assert!(!wait_for_webview_probe(rx, Duration::from_secs(1)).await);

        let (tx, rx) = tokio::sync::oneshot::channel();
        drop(tx);
        assert!(!wait_for_webview_probe(rx, Duration::from_secs(1)).await);
    }

    #[tokio::test]
    async fn a_stalled_probe_keeps_the_bounded_startup_deadline() {
        let (_tx, rx) = tokio::sync::oneshot::channel();
        assert!(!wait_for_webview_probe(rx, Duration::from_millis(1)).await);
    }
}
