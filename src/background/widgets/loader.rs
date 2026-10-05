use std::{sync::Arc, time::Duration};

use seelen_core::state::{Widget, WidgetInstanceMode, WidgetStatus};
use slu_utils::widget_health::{HealthAction, WidgetHealth};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use uuid::Uuid;

use crate::{
    app::get_app_handle,
    get_tokio_handle,
    modules::monitors::MonitorManager,
    resources::RESOURCES,
    state::application::FULL_STATE,
    utils::lock_free::SyncHashMap,
    widgets::{
        WidgetWebviewLabel, flush_pending_trigger,
        manager::WIDGET_MANAGER,
        notify_widget_statuses_change,
        pod_readiness::{EarlyWindowStatus, StatusAdmission, status_admission},
        webview::WidgetWebview,
    },
    windows_api::event_window::{IS_DISPLAY_ON, IS_INTERACTIVE_SESSION},
};

pub enum PodSource {
    Static,
    Runtime,
}

const LIVENESS_PROVE_INTERVAL: Duration = Duration::from_secs(5);
const LIVENESS_PROVE_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
const LIVENESS_PROVE_MAX_RETRIES: u8 = 5;
// Grace period after session resume or soft_restart to let the webview finish reloading.
const LIVENESS_RELOAD_GRACE_PERIOD: Duration = Duration::from_secs(10);

pub struct WidgetDeployment {
    pub definition: Arc<Widget>,
    pub pods: SyncHashMap<WidgetWebviewLabel, WidgetPod>,
}

impl WidgetDeployment {
    pub fn new(definition: Arc<Widget>) -> Self {
        log::trace!("Registering widget: {}", definition.id);
        Self {
            definition,
            pods: SyncHashMap::new(),
        }
    }

    /// Will revaluate all widget instances and remove or add them based on current user settings
    pub fn reconcile(&self) {
        match self.definition.instances {
            WidgetInstanceMode::Single => {
                if self.pods.is_empty() {
                    let label = WidgetWebviewLabel::new(&self.definition.id, None, None);
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }
            }
            WidgetInstanceMode::Multiple => {
                let nil_id = Uuid::nil();
                if !self.definition.lazy && self.pods.is_empty() {
                    let label = WidgetWebviewLabel::new(&self.definition.id, None, Some(&nil_id));
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }

                let replicas_ids = FULL_STATE
                    .load()
                    .get_widget_instances_ids(&self.definition.id);

                // Remove deleted static instances; runtime pods are never evicted by reconcile.
                self.pods.retain(|(label, pod)| {
                    if matches!(pod.source, PodSource::Runtime) {
                        return true;
                    }
                    let instance_id = label.instance_id.expect("Missing instance id");
                    instance_id == nil_id || replicas_ids.contains(&instance_id)
                });

                // Add new instances
                for replica_id in replicas_ids {
                    if !self
                        .pods
                        .any(|(label, _)| label.instance_id == Some(replica_id))
                    {
                        let label =
                            WidgetWebviewLabel::new(&self.definition.id, None, Some(&replica_id));
                        let instance = WidgetPod::create(label, None, PodSource::Static);
                        self.pods.upsert(instance.label.clone(), instance);
                    }
                }
            }
            WidgetInstanceMode::ReplicaByMonitor => {
                let configs = FULL_STATE.load();
                let connected_ids = MonitorManager::instance().get_cached_ids();

                // Remove disabled or disconnected instances
                self.pods.retain(|(label, _)| {
                    let monitor_id = label.monitor_id.as_ref().expect("Missing monitor id");
                    connected_ids.contains(monitor_id)
                        && configs.is_widget_enable_on_monitor(&self.definition.id, monitor_id)
                });

                // Add new/enabled instances
                for monitor_id in connected_ids {
                    if self
                        .pods
                        .any(|(label, _)| label.monitor_id.as_ref() == Some(&monitor_id))
                    {
                        continue;
                    }

                    if !configs.is_widget_enable_on_monitor(&self.definition.id, &monitor_id) {
                        continue;
                    }

                    let label =
                        WidgetWebviewLabel::new(&self.definition.id, Some(&monitor_id), None);
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }
            }
        }
    }

    pub fn start_all_webviews(&self) {
        self.pods.for_each(|(_k, pod)| {
            pod.run(&self.definition);
        });
    }

    pub fn start_webview(&self, label: &WidgetWebviewLabel) {
        self.pods.get(label, |pod| {
            pod.run(&self.definition);
        });
    }

    pub fn create_runtime_instance(&self, instance_id: &Uuid, owner_hwnd: Option<isize>) {
        let label = WidgetWebviewLabel::new(&self.definition.id, None, Some(instance_id));
        let instance = WidgetPod::create(label, owner_hwnd, PodSource::Runtime);
        self.pods.upsert(instance.label.clone(), instance);
    }

    pub fn kill_pod(&self, label: &WidgetWebviewLabel) {
        self.pods.remove(label);
    }
}

pub struct WidgetPod {
    pub label: WidgetWebviewLabel,
    pub source: PodSource,

    window: Option<WidgetWebview>,
    _status: WidgetStatus,
    owner_hwnd: Option<isize>,
    generation: Uuid,
    early_status: EarlyWindowStatus<WidgetStatus>,

    live: Arc<tokio::sync::Notify>,
    liveness_prove_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WidgetPod {
    fn create(label: WidgetWebviewLabel, owner_hwnd: Option<isize>, source: PodSource) -> Self {
        Self {
            label,
            source,
            window: None,
            _status: WidgetStatus::Pending,
            owner_hwnd,
            generation: Uuid::new_v4(),
            early_status: EarlyWindowStatus::default(),
            live: Arc::new(tokio::sync::Notify::new()),
            liveness_prove_handle: None,
        }
    }

    pub fn status(&self) -> &WidgetStatus {
        &self._status
    }

    pub fn hwnd(&self) -> Option<isize> {
        self.window.as_ref()?.0.hwnd().ok().map(|h| h.0 as isize)
    }

    pub fn set_status(&mut self, status: WidgetStatus) {
        log::trace!(target: &self.label.decoded, "status changed to: {status:?}");
        self._status = status;
        notify_widget_statuses_change();
    }

    pub fn is_ready(&self) -> bool {
        self.window.is_some() && self.status() == &WidgetStatus::Ready
    }

    pub fn report_status(&mut self, sender_hwnd: isize, status: WidgetStatus) -> bool {
        match status_admission(
            self.hwnd(),
            self.status() == &WidgetStatus::Creating,
            sender_hwnd,
        ) {
            StatusAdmission::Apply => self.set_status(status),
            StatusAdmission::Buffer => self.early_status.remember(sender_hwnd, status),
            StatusAdmission::Reject => return false,
        }
        self.is_ready()
    }

    pub fn soft_restart(&mut self) {
        if self.window.is_none() {
            // Pod was never started; leave it in Pending so run() can initialize it.
            return;
        }
        self.set_status(WidgetStatus::Restarting);
        if let Some(window) = &self.window {
            window.reload();
        }
    }

    fn run(&mut self, definition: &Widget) {
        if self.status() != &WidgetStatus::Pending {
            return;
        }

        self.set_status(WidgetStatus::Creating);
        let label = self.label.clone();
        let generation = self.generation;
        let definition = definition.clone();
        let owner_hwnd = self.owner_hwnd;

        // run() is called while both widget maps are locked. Window construction waits
        // for the UI thread, which can itself be handling widget commands. Never hold
        // either map across that wait (the old path timed out every other widget).
        std::thread::spawn(move || {
            let is_current = || {
                WIDGET_MANAGER
                    .deployments
                    .get(&label.widget_id, |deployment| {
                        deployment
                            .pods
                            .get(&label, |pod| pod.generation == generation)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            };
            let mut created = None;
            // A retired window may still be awaiting Destroyed on the UI thread.
            // Do not create another native window with the same label in that gap.
            for _ in 0..60 {
                if !is_current() {
                    return;
                }
                if get_app_handle().get_webview_window(&label.raw).is_some() {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                let result = WidgetWebview::create(&definition, &label, owner_hwnd);
                if result.is_err() && get_app_handle().get_webview_window(&label.raw).is_some() {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                created = Some(result);
                break;
            }
            let window = match created
                .unwrap_or_else(|| Err("Previous widget window did not close".into()))
            {
                Ok(window) => window,
                Err(err) => {
                    log::error!("Failed to create webview {label}: {err}");
                    WIDGET_MANAGER
                        .deployments
                        .get(&label.widget_id, |deployment| {
                            deployment.pods.get(&label, |pod| {
                                if pod.generation == generation {
                                    pod.set_status(WidgetStatus::CrashedOnCreation);
                                }
                            });
                        });
                    return;
                }
            };
            let destroyed_label = label.clone();
            window.0.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    let label = destroyed_label.clone();
                    std::thread::spawn(move || {
                        let removed = WIDGET_MANAGER
                            .deployments
                            .get(&label.widget_id, |deploy| {
                                let matches = deploy
                                    .pods
                                    .get(&label, |pod| pod.generation == generation)
                                    .unwrap_or(false);
                                if matches {
                                    deploy.kill_pod(&label);
                                }
                                matches
                            })
                            .unwrap_or(false);
                        if removed {
                            if let Err(err) = WIDGET_MANAGER.reconcile() {
                                log::error!("Failed to reconcile destroyed widget: {err}");
                            }
                        }
                    });
                }
            });
            if definition.debug {
                window.0.open_devtools();
            }
            let mut window = Some(window);
            let ready = WIDGET_MANAGER
                .deployments
                .get(&label.widget_id, |deployment| {
                    deployment
                        .pods
                        .get(&label, |pod| {
                            if pod.generation == generation {
                                pod.window = window.take();
                                let early =
                                    pod.hwnd().and_then(|hwnd| pod.early_status.take_for(hwnd));
                                pod.set_status(early.unwrap_or(WidgetStatus::Mounting));
                                pod.start_liveness_prove();
                                notify_widget_statuses_change();
                                return pod.is_ready();
                            }
                            false
                        })
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            // If removed/replaced during creation, dispose outside the widget locks.
            drop(window);
            if ready {
                if let Err(err) = flush_pending_trigger(&label) {
                    log::error!("Failed to deliver early widget trigger for {label}: {err}");
                }
            }
        });
    }

    fn start_liveness_prove(&mut self) {
        if let Some(window) = &self.window {
            let live = self.live.clone();
            window.0.listen("internal::liveness-pong", move |_event| {
                live.notify_one();
            });
        }

        let live = self.live.clone();
        let label = self.label.clone();
        let generation = self.generation;

        let handle = get_tokio_handle().spawn(async move {
            let app = get_app_handle();
            let mut health = WidgetHealth::default();
            let mut was_suspended = false;
            let mut recovery_generation = MonitorManager::recovery_generation();
            tokio::time::sleep(LIVENESS_RELOAD_GRACE_PERIOD).await;

            loop {
                tokio::time::sleep(LIVENESS_PROVE_INTERVAL).await;
                if !IS_INTERACTIVE_SESSION.load(std::sync::atomic::Ordering::Acquire)
                    || !IS_DISPLAY_ON.load(std::sync::atomic::Ordering::Acquire)
                {
                    was_suspended = true;
                    continue;
                }
                let current_recovery = MonitorManager::recovery_generation();
                if was_suspended || current_recovery != recovery_generation {
                    recovery_generation = current_recovery;
                    was_suspended = false;
                    health.reset();
                    tokio::time::sleep(LIVENESS_RELOAD_GRACE_PERIOD).await;
                    continue;
                }

                // Drain an old permit, then subscribe BEFORE emitting. A fast pong
                // previously arrived before notified() was polled and was lost.
                let _ = tokio::time::timeout(Duration::ZERO, live.notified()).await;
                let reply = live.notified();
                tokio::pin!(reply);
                reply.as_mut().enable();
                if app.emit_to(&label.raw, "internal::liveness-ping", ()).is_err() {
                    continue;
                }
                let responded = tokio::time::timeout(LIVENESS_PROVE_WAIT_TIMEOUT, reply).await.is_ok();
                if !IS_INTERACTIVE_SESSION.load(std::sync::atomic::Ordering::Acquire)
                    || !IS_DISPLAY_ON.load(std::sync::atomic::Ordering::Acquire)
                    || MonitorManager::recovery_generation() != recovery_generation
                {
                    was_suspended = true;
                    health.reset();
                    continue;
                }
                if responded {
                    health.reset();
                    continue;
                }
                match health.missed_reply() {
                    HealthAction::Retry => continue,
                    HealthAction::Reload(attempt) => {
                        log::warn!("Liveness failed repeatedly for {label} (reload {attempt}/{LIVENESS_PROVE_MAX_RETRIES})");
                        WIDGET_MANAGER.deployments.get(&label.widget_id, |deployment| {
                            deployment.pods.get(&label, |pod| {
                                if pod.generation == generation { pod.soft_restart(); }
                            });
                        });
                        tokio::time::sleep(LIVENESS_RELOAD_GRACE_PERIOD).await;
                    }
                    HealthAction::GiveUp => {
                        log::error!("Liveness failed for {label} after repeated reloads");
                        let lang = rust_i18n::locale();
                        let widget_name = RESOURCES.widgets.read_async(&label.widget_id, |_, w| {
                            w.metadata.display_name.get(&lang).to_string()
                        }).await.unwrap_or_else(|| label.widget_id.to_string());
                        app.dialog()
                            .message(t!("widget_liveness.failed_description", widget_name = widget_name))
                            .title(t!("widget_liveness.failed_title"))
                            .kind(MessageDialogKind::Error)
                            .buttons(MessageDialogButtons::Ok)
                            .show(|_| {});
                        break;
                    }
                }
            }
        });
        self.liveness_prove_handle = Some(handle);
    }
}

impl Drop for WidgetPod {
    fn drop(&mut self) {
        log::trace!(target: &self.label.decoded, "dropped");
        if let Some(handle) = self.liveness_prove_handle.take() {
            handle.abort();
        }
    }
}
