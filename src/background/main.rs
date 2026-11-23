// The desktop app is a GUI in both self-contained debug and release builds.
// The separate macui-cli binary provides console commands.
#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]
#![feature(never_type)]

mod app;
mod backups;
mod cli;
mod error;
mod exit_flow;
mod exposed;
mod hook;
mod local_migration;
mod logger;
mod migrations;
mod modules;
mod resources;
mod server;
mod session;
mod state;
mod tauri_context;
mod tauri_plugins;
mod utils;
mod virtual_desktops;
mod widgets;
mod windows_api;

#[macro_use]
extern crate rust_i18n;
i18n!("background/i18n", fallback = "en");

use std::sync::{OnceLock, atomic::AtomicBool};

use app::MacUi;
use cli::{SelfPipe, ServicePipe, handle_console_client};
use error::Result;
use exposed::register_invoke_handler;
use logger::MacUiLogger;
use tauri_plugins::register_plugins;
use utils::{
    integrity::{is_already_running, print_initial_information, restart_as_appx, warn_if_elevated},
    is_running_as_appx, was_installed_using_msix,
};

use crate::{app::get_app_handle, error::ResultLogExt, utils::constants::MACUI_COMMON};

static APP_HANDLE: OnceLock<tauri::AppHandle<tauri::Wry>> = OnceLock::new();
static TOKIO_RUNTIME_HANDLE: OnceLock<tokio::runtime::Handle> = OnceLock::new();

static SILENT: AtomicBool = AtomicBool::new(false);
static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn is_local_dev() -> bool {
    cfg!(dev)
}

pub fn get_tokio_handle() -> &'static tokio::runtime::Handle {
    TOKIO_RUNTIME_HANDLE
        .get()
        .expect("Tokio runtime was not initialized")
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    if let Err(err) = MacUiLogger::init() {
        let fallback = std::env::temp_dir().join("mac-ui-logger-error.log");
        let _ = std::fs::write(&fallback, format!("Failed to initialize logger: {err:?}"));
        return std::process::ExitCode::from(1);
    }

    if let Err(err) = handle_console_client().await {
        log::error!("Failed to execute command: {err:?}");
        return std::process::ExitCode::from(1);
    };

    if is_already_running() {
        if let Err(error) = ServicePipe::recover_for_existing_instance().await {
            log::warn!("Could not restore the existing instance's helper: {error}");
        }
        return match SelfPipe::request_open_settings().await {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                log::error!(
                    "Existing MAC UI instance did not acknowledge opening settings; refusing a duplicate desktop: {error}"
                );
                std::process::ExitCode::from(1)
            }
        };
    }

    if was_installed_using_msix() && !is_running_as_appx() {
        log::info!("GUI was installed using MSIX, restarting as appx...");
        restart_as_appx().log_error();
        return std::process::ExitCode::from(0);
    }

    TOKIO_RUNTIME_HANDLE
        .set(tokio::runtime::Handle::current())
        .expect("Failed to set runtime handle");

    rust_i18n::set_locale(&macui_core::state::Settings::get_app_language());

    let mut app_builder = tauri::Builder::default();
    app_builder = register_plugins(app_builder);
    app_builder = register_invoke_handler(app_builder);

    // if no custom runtime is present, the app will use the installed with the system
    if let Some(path) = crate::utils::get_fixed_runtime_path() {
        unsafe { std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", path) };
    }

    let app = app_builder
        .setup(|app| {
            APP_HANDLE.set(app.handle().to_owned()).unwrap();
            tokio::spawn(async move {
                let handle = get_app_handle();
                if let Err(err) = crate::measure!("Setup", setup(handle).await) {
                    log::error!("Error while setting up: {err:?}");
                    handle.exit(1);
                }
            });
            Ok(())
        })
        .build(tauri_context::get_context())
        .expect("Error while building tauri application");

    // share the current runtime with Tauri
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let exit_code = app.run_return(app_callback);
    std::process::ExitCode::from(exit_code as u8)
}

async fn setup(app_handle: &tauri::AppHandle<tauri::Wry>) -> Result<()> {
    print_initial_information();
    local_migration::migrate_legacy_data(app_handle)?;
    create_main_folders()?;

    SelfPipe::start_listener()?;
    crate::measure!("Service readiness", {
        ServicePipe::start_service().await?;
    });

    if let Err(err) = utils::integrity::validate_webview_runtime() {
        match err {
            utils::integrity::IntegrityError::WebviewRuntimeNotInstalled => {
                utils::integrity::show_not_installed_dialog(get_app_handle())?;
            }
            utils::integrity::IntegrityError::WebviewRuntimeOutdated => {
                utils::integrity::show_outdated_dialog(get_app_handle())?;
            }
        }
        return Err(format!("Integrity check failed: {err:?}").into());
    }

    if let Err(err) = crate::measure!(
        "Integrity check",
        utils::integrity::ensure_bundle_files_integrity(get_app_handle()).await
    ) {
        utils::integrity::show_bundle_integrity_dialog(get_app_handle());
        return Err(err);
    }

    MacUi::pre_start().await?;
    crate::measure!("Widget startup", MacUi::start().await)?;
    ServicePipe::start_health_monitor();
    warn_if_elevated(app_handle);
    server::http::start_http_server();
    Ok(())
}

fn create_main_folders() -> Result<()> {
    std::fs::create_dir_all(MACUI_COMMON.app_temp_dir())?;
    std::fs::create_dir_all(MACUI_COMMON.app_data_dir())?;
    std::fs::create_dir_all(MACUI_COMMON.app_cache_dir())?;

    std::fs::create_dir_all(MACUI_COMMON.user_themes_path())?;
    std::fs::create_dir_all(MACUI_COMMON.user_icons_path())?;
    std::fs::create_dir_all(MACUI_COMMON.user_wallpapers_path())?;
    std::fs::create_dir_all(MACUI_COMMON.user_sounds_path())?;
    std::fs::create_dir_all(MACUI_COMMON.user_plugins_path())?;
    std::fs::create_dir_all(MACUI_COMMON.user_widgets_path())?;
    Ok(())
}

fn app_callback(app: &tauri::AppHandle<tauri::Wry>, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::Ready => {
            log::info!("Tauri Application is ready.");
        }
        tauri::RunEvent::Resumed => {
            log::info!("Tauri Event Loop was resumed.");
        }
        tauri::RunEvent::ExitRequested { api, code, .. } => match code {
            Some(0) => exit_flow::on_user_exit(app, api),
            Some(_) => {}
            // prevent close background on webview windows closing
            None => api.prevent_exit(),
        },
        tauri::RunEvent::Exit => {
            ServicePipe::begin_shutdown();
            crate::widgets::weg::genie::shutdown();
            log::info!("───────────────────── Exiting MAC UI ─────────────────────");
        }
        _ => {}
    }
}
