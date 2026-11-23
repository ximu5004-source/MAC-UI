//! Process lifecycle and JSON settings transport for native widgets.
use crate::{
    app::get_app_handle,
    error::{Result, ResultLogExt},
    state::application::FULL_STATE,
    utils::constants::MACUI_COMMON,
};
use macui_core::state::{WidgetDebugInfo, WidgetStatus};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, LazyLock, Once,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Listener};

pub const DESKTOP_ID: &str = "@macui/desktop-shell";
enum Message {
    Reconcile { enabled: bool, settings: Value },
    Forward(Value),
    Stop(mpsc::SyncSender<std::result::Result<(), String>>),
    Exited(u32),
}
struct Runtime {
    child: Child,
    input: Option<ChildStdin>,
}
impl Runtime {
    fn send(&mut self, message: &Value) -> Result<()> {
        let input = self.input.as_mut().ok_or("Native widget is stopping")?;
        serde_json::to_writer(&mut *input, message)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            return Ok(());
        }
        let request = self.send(&json!({"type":"stop"}));
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                request?;
                return Err("Native desktop is still completing a file operation".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
}
#[derive(Default)]
struct Status {
    process: u32,
    info: Option<WidgetDebugInfo>,
}
struct NativeWidgetHost {
    sender: mpsc::Sender<Message>,
    status: Arc<Mutex<Status>>,
}
static HOST: LazyLock<NativeWidgetHost> = LazyLock::new(NativeWidgetHost::new);
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
impl NativeWidgetHost {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status::default()));
        let worker_status = status.clone();
        let worker_sender = sender.clone();
        std::thread::spawn(move || Self::run(receiver, worker_status, worker_sender));
        Self { sender, status }
    }
    fn run(
        receiver: mpsc::Receiver<Message>,
        status: Arc<Mutex<Status>>,
        sender: mpsc::Sender<Message>,
    ) {
        let mut runtime: Option<Runtime> = None;
        let mut restart_after_stop = None;
        while let Ok(message) = receiver.recv() {
            match message {
                Message::Reconcile { enabled, settings } => {
                    let result = (|| -> Result<()> {
                        if !enabled || EXIT_REQUESTED.load(Ordering::Acquire) {
                            restart_after_stop = None;
                            if let Some(current) = runtime.as_mut() {
                                current.stop()?;
                            }
                            runtime = None;
                            *status.lock() = Status::default();
                        } else {
                            if let Some(current) = runtime.as_mut() {
                                if current.child.try_wait()?.is_none() {
                                    if current.input.is_none() {
                                        restart_after_stop = Some(settings);
                                        return Ok(());
                                    }
                                    return current.send(&settings);
                                }
                            }
                            restart_after_stop = None;
                            runtime = Some(Self::start(settings, status.clone(), sender.clone())?);
                        }
                        Ok(())
                    })();
                    result.log_error();
                }
                Message::Forward(message) => {
                    let result: Result<()> = runtime
                        .as_mut()
                        .ok_or_else(|| "Native desktop is not running".into())
                        .and_then(|current| current.send(&message));
                    if let Err(error) = result {
                        get_app_handle()
                            .emit(
                                "macui-desktop-controls-response",
                                json!({"id":message["id"],"error":format!("{error}")}),
                            )
                            .log_error();
                    }
                }
                Message::Stop(reply) => {
                    restart_after_stop = None;
                    let result = runtime.as_mut().map_or(Ok(()), Runtime::stop);
                    if result.is_ok() {
                        runtime = None;
                        *status.lock() = Status::default();
                    }
                    let _ = reply.send(result.map_err(|error| format!("{error}")));
                }
                Message::Exited(process) => {
                    if runtime
                        .as_ref()
                        .is_some_and(|current| current.child.id() == process)
                    {
                        if let Some(mut current) = runtime.take() {
                            current.child.wait().log_error();
                        }
                        if !EXIT_REQUESTED.load(Ordering::Acquire) {
                            if let Some(settings) = restart_after_stop.take() {
                                match Self::start(settings, status.clone(), sender.clone()) {
                                    Ok(started) => runtime = Some(started),
                                    Err(error) => log::error!("Native widget restart: {error}"),
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(mut current) = runtime {
            current.stop().log_error();
        }
    }
    fn start(
        settings: Value,
        status: Arc<Mutex<Status>>,
        sender: mpsc::Sender<Message>,
    ) -> Result<Runtime> {
        let mut command = Command::new(
            MACUI_COMMON
                .app_resource_dir()
                .join("static/native/macui-desktop.exe"),
        );
        command
            .args([
                "--desktop",
                "--parent",
                &std::process::id().to_string(),
                "--data",
            ])
            .arg(MACUI_COMMON.app_data_dir().join("data/macui-desktop-shell"))
            .args(["--locale", settings["locale"].as_str().unwrap_or("en")])
            .args(["--toolbar", &settings["toolbar"].to_string()]);
        if settings["contrast"] == true {
            command.arg("--contrast");
        }
        let mut child = command
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let process = child.id();
        let input = child.stdin.take().expect("piped native widget stdin");
        let output = child.stdout.take().expect("piped native widget stdout");
        let errors = child.stderr.take().expect("piped native widget stderr");
        *status.lock() = Status {
            process,
            info: Some(WidgetDebugInfo {
                label: DESKTOP_ID.into(),
                widget_id: DESKTOP_ID.into(),
                monitor_id: None,
                instance_id: None,
                status: WidgetStatus::Creating,
                webview_window_id: None,
            }),
        };
        std::thread::spawn(move || {
            for line in BufReader::new(errors).lines() {
                match line {
                    Ok(line) => log::error!(target: "macui_native_desktop", "{line}"),
                    Err(error) => {
                        log::error!("Native widget stderr: {error}");
                        break;
                    }
                }
            }
        });
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let result = (|| -> Result<()> {
                    let message: Value = serde_json::from_str(&line?)?;
                    let payload = &message["payload"];
                    match message["type"].as_str() {
                        Some("ready") => {
                            let mut current = status.lock();
                            if current.process == process {
                                if let Some(info) = current.info.as_mut() {
                                    info.status = WidgetStatus::Ready;
                                    info.webview_window_id =
                                        payload["windows"][0].as_i64().map(|value| value as isize);
                                }
                            }
                            drop(current);
                            super::notify_widget_statuses_change();
                        }
                        Some("controls") => {
                            get_app_handle().emit("macui-desktop-controls-response", payload)?
                        }
                        Some("changed") => {
                            get_app_handle().emit("macui-desktop-controls-changed", payload)?
                        }
                        Some("open-settings") => super::show_settings_at("/desktop")?,
                        Some("error") => log::error!(target: "macui_native_desktop", "{payload}"),
                        _ => return Err("Unknown native widget response".into()),
                    }
                    Ok(())
                })();
                result.log_error();
            }
            let mut current = status.lock();
            if current.process == process {
                if let Some(info) = current.info.as_mut() {
                    info.status = WidgetStatus::CrashedOnCreation;
                    info.webview_window_id = None;
                }
            }
            drop(current);
            super::notify_widget_statuses_change();
            let _ = sender.send(Message::Exited(process));
        });
        Ok(Runtime {
            child,
            input: Some(input),
        })
    }
}
pub fn reconcile(enabled: bool) -> Result<()> {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        get_app_handle().listen("macui-desktop-controls-request", |event| {
            let result = (|| -> Result<()> {
                let mut request: Value = serde_json::from_str(event.payload())?;
                request["type"] = "controls".into();
                HOST.sender
                    .send(Message::Forward(request))
                    .map_err(|error| format!("{error}"))?;
                Ok(())
            })();
            result.log_error();
        });
    });
    let state = FULL_STATE.load();
    let settings = serde_json::to_value(&state.settings)?;
    HOST.sender.send(Message::Reconcile { enabled, settings: json!({"type":"settings", "locale":state.locale(), "contrast":settings["byWidget"]["@macui/settings"]["colorScheme"] == "dark", "toolbar":settings["byWidget"]["@macui/fancy-toolbar"]}) }).map_err(|error| format!("{error}"))?;
    Ok(())
}
pub fn stop() -> Result<()> {
    EXIT_REQUESTED.store(true, Ordering::Release);
    let (reply, result) = mpsc::sync_channel(1);
    HOST.sender
        .send(Message::Stop(reply))
        .map_err(|error| format!("{error}"))?;
    result
        .recv()
        .map_err(|error| format!("{error}"))?
        .map_err(Into::into)
}
pub fn resume_after_cancelled_exit() {
    EXIT_REQUESTED.store(false, Ordering::Release);
    super::manager::WIDGET_MANAGER.reconcile().log_error();
}
pub fn debug_status() -> Option<WidgetDebugInfo> {
    HOST.status.lock().info.clone()
}
