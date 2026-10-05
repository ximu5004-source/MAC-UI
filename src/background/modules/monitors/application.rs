use std::{
    collections::HashMap,
    sync::{
        LazyLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use seelen_core::system_state::{MonitorId, PhysicalMonitor};
use slu_utils::display_recovery::{StableDisplaySnapshot, should_refresh_display_layout};
use slu_utils::{Debounce, debounce};
use windows::{
    Devices::Display::Core::{
        DisplayManager, DisplayManagerChangedEventArgs, DisplayManagerDisabledEventArgs,
        DisplayManagerEnabledEventArgs, DisplayManagerOptions,
        DisplayManagerPathsFailedOrInvalidatedEventArgs,
    },
    Foundation::TypedEventHandler,
    Win32::UI::WindowsAndMessaging::{
        PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, WM_DISPLAYCHANGE, WM_DWMCOMPOSITIONCHANGED,
        WM_POWERBROADCAST,
    },
};

use crate::{
    error::{Result, ResultLogExt},
    event_manager,
    modules::system_settings::application::{SystemSettings, SystemSettingsEvent},
    utils::lock_free::{SyncHashMap, SyncVec},
    windows_api::{
        MonitorEnumerator, event_window::subscribe_to_background_window, monitor::DisplayView,
    },
};

/// Builds a stable-ordered snapshot of the current physical monitors (rect/scale/primary),
/// used to detect real changes vs spurious WM_DISPLAYCHANGE/WinRT notifications that carry
/// no actual diff (e.g. the same set of targets, but re-reported without any property change).
fn snapshot_physical_monitors() -> Result<Vec<PhysicalMonitor>> {
    // A failed conversion is a failed sample, not a disconnected monitor.
    let mut monitors = MonitorEnumerator::enumerate_win32()?
        .into_iter()
        .map(PhysicalMonitor::try_from)
        .collect::<Result<Vec<_>>>()?;
    monitors.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Ok(monitors)
}

static FORCE_LAYOUT_REFRESH: AtomicBool = AtomicBool::new(false);
static DISPLAY_STACK_ENABLED: AtomicBool = AtomicBool::new(true);
static RECOVERY_GENERATION: AtomicU64 = AtomicU64::new(0);

pub struct MonitorManager {
    state_views: SyncHashMap<MonitorId, DisplayView>,
    state_data: SyncVec<PhysicalMonitor>,
    /// DisplayManager manages critical hardware so be sure to be correctly used, or will make the app crash.
    /// https://learn.microsoft.com/en-us/uwp/api/windows.devices.display.core.displaymanager
    display_manager: DisplayManager,
    enabled_token: Option<i64>,
    disabled_token: Option<i64>,
    changed_token: Option<i64>,
    paths_failed_or_invalidated_token: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorManagerEvent {
    /// the id used is the view primary target id
    ViewAdded(MonitorId),
    /// the id used is the view primary target id
    ViewRemoved(MonitorId),
    ViewsChanged,
}

event_manager!(MonitorManager, MonitorManagerEvent);

impl MonitorManager {
    fn create() -> Result<MonitorManager> {
        let display_manager = DisplayManager::Create(DisplayManagerOptions::None)?;
        let state = display_manager
            .TryReadCurrentStateForAllTargets()?
            .State()?;

        let mut state_views = HashMap::new();
        for view in state.Views()? {
            let view = DisplayView::from(view);
            if !view.is_active()? {
                continue;
            }
            state_views.insert(view.primary_target()?.stable_id()?, view);
        }

        Ok(MonitorManager {
            display_manager,
            state_views: SyncHashMap::from(state_views),
            state_data: SyncVec::from(snapshot_physical_monitors().unwrap_or_else(|error| {
                // No prior snapshot exists at startup. Let the enabled/recovery
                // notifications populate it instead of failing the entire app.
                log::warn!("Initial monitor geometry unavailable: {error}");
                Vec::new()
            })),
            enabled_token: None,
            disabled_token: None,
            changed_token: None,
            paths_failed_or_invalidated_token: None,
        })
    }

    pub fn instance() -> &'static MonitorManager {
        static MONITOR_MANAGER: LazyLock<MonitorManager> = LazyLock::new(|| {
            let mut m = MonitorManager::create().expect("Failed to create monitor manager");
            m.initialize().log_error();
            m
        });
        &MONITOR_MANAGER
    }

    fn initialize(&mut self) -> Result<()> {
        // DisplayManager.Start() requires subscribing to all events first
        // See: https://learn.microsoft.com/en-us/uwp/api/windows.devices.display.core.displaymanager.start
        self.enabled_token = self
            .display_manager
            .Enabled(&TypedEventHandler::new(Self::on_enabled))
            .ok();

        self.disabled_token = self
            .display_manager
            .Disabled(&TypedEventHandler::new(Self::on_disabled))
            .ok();

        self.changed_token = self
            .display_manager
            .Changed(&TypedEventHandler::new(Self::on_changed))
            .ok();

        self.paths_failed_or_invalidated_token = self
            .display_manager
            .PathsFailedOrInvalidated(&TypedEventHandler::new(
                Self::on_paths_failed_or_invalidated,
            ))
            .ok();

        self.display_manager.Start()?;
        SystemSettings::subscribe(|event| {
            if event == SystemSettingsEvent::TextScaleChanged {
                log::debug!("Text scale changed, re-emitting ViewsChanged");
                Self::request_display_state_refresh(true);
            }
        });

        subscribe_to_background_window(|event, w_param, _l_param| {
            if matches!(event, WM_DISPLAYCHANGE | WM_DWMCOMPOSITIONCHANGED) {
                // Win8+ normally does not send the composition message. Display
                // power On remains the primary path for modern DWM recovery.
                log::debug!("Display/composition changed");
                Self::refresh_after_resume();
            } else if event == WM_POWERBROADCAST
                && matches!(
                    w_param as u32,
                    PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND
                )
            {
                Self::refresh_after_resume();
            }
            Ok(())
        });
        Ok(())
    }

    /// Coalesces bursts of display notifications (WM_DISPLAYCHANGE + the several
    /// WinRT DisplayManager events can all fire for a single physical connect/disconnect)
    /// and waits for the Win32 topology to settle before diffing state, since Windows
    /// can take a moment after WinRT reports a target as connected to finish applying
    /// the mode/DPI/work-area for a newly attached monitor.
    fn request_display_state_refresh(force_layout: bool) {
        if force_layout {
            FORCE_LAYOUT_REFRESH.store(true, Ordering::Release);
        }
        static DEBOUNCER: LazyLock<Debounce<()>> = LazyLock::new(|| {
            debounce(
                |_| {
                    let recovering = FORCE_LAYOUT_REFRESH.swap(false, Ordering::AcqRel);
                    match MonitorManager::check_for_display_changes() {
                        Ok(changed) => {
                            if should_refresh_display_layout(changed, recovering) {
                                MonitorManager::send(MonitorManagerEvent::ViewsChanged);
                            } else {
                                log::trace!("Display state refresh requested but nothing changed");
                            }
                        }
                        Err(e) => {
                            if recovering {
                                FORCE_LAYOUT_REFRESH.store(true, Ordering::Release);
                            }
                            log::warn!("Failed to check for display changes: {e}");
                        }
                    }
                },
                Duration::from_millis(400),
            )
        });
        DEBOUNCER.call(());
    }

    /// Wake can precede the final display mode by seconds, with no subsequent
    /// topology event. Reapply geometry in bounded, coalesced recovery passes.
    pub fn refresh_after_resume() {
        let generation = RECOVERY_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
        Self::request_display_state_refresh(true);
        crate::get_tokio_handle().spawn(async move {
            for delay in [2, 3] {
                tokio::time::sleep(Duration::from_secs(delay)).await;
                if RECOVERY_GENERATION.load(Ordering::Acquire) != generation {
                    return;
                }
                Self::request_display_state_refresh(true);
            }
        });
    }

    /// Unlike the interactive-session boolean, this also detects a complete
    /// suspend/resume cycle that happened between two liveness checks.
    pub fn recovery_generation() -> u64 {
        RECOVERY_GENERATION.load(Ordering::Acquire)
    }

    /// Require consecutive matching identities, geometry and DPI, not just count.
    /// Never publish a timeout's possibly stale/partial sample to widget consumers.
    fn wait_for_win32_to_settle(
        expected: &HashMap<MonitorId, DisplayView>,
    ) -> Result<Vec<PhysicalMonitor>> {
        const MAX_ATTEMPTS: u32 = 30;
        const RETRY_DELAY: Duration = Duration::from_millis(150);
        let mut stable = StableDisplaySnapshot::new(4);

        for attempt in 0..MAX_ATTEMPTS {
            let sample = snapshot_physical_monitors().ok().filter(|monitors| {
                monitors.len() == expected.len()
                    && monitors.iter().all(|m| {
                        expected.contains_key(&m.id)
                            && m.rect.right > m.rect.left
                            && m.rect.bottom > m.rect.top
                            && m.scale_factor.is_finite()
                            && m.scale_factor > 0.0
                    })
            });
            if let Some(monitors) = stable.observe(sample) {
                return Ok(monitors);
            }
            if attempt + 1 < MAX_ATTEMPTS {
                std::thread::sleep(RETRY_DELAY);
            }
        }
        Err("Display topology is not stable yet; preserving the last known desktop geometry".into())
    }

    // Is recommended that subscribers re-enumerate all targets and state in this call,
    // since the system display stack could be left in any state before this event is raised.
    fn on_enabled(
        _sender: windows_core::Ref<DisplayManager>,
        args: windows_core::Ref<DisplayManagerEnabledEventArgs>,
    ) -> windows_core::Result<()> {
        log::trace!("DisplayManager enabled");

        // Critical!: app will crash if this is not set
        if let Some(args) = args.as_ref() {
            args.SetHandled(true)?;
        }
        DISPLAY_STACK_ENABLED.store(true, Ordering::Release);
        Self::refresh_after_resume();
        Ok(())
    }

    // Is recommended that subscribers attempt to clean up when Disabled is invoked.
    // Most display APIs will fail while the session display stack is disabled.
    fn on_disabled(
        _sender: windows_core::Ref<DisplayManager>,
        args: windows_core::Ref<DisplayManagerDisabledEventArgs>,
    ) -> windows_core::Result<()> {
        log::trace!("DisplayManager disabled");

        // Critical!: app will crash if this is not set
        if let Some(args) = args.as_ref() {
            args.SetHandled(true)?;
        }
        DISPLAY_STACK_ENABLED.store(false, Ordering::Release);
        Ok(())
    }

    // this only detects changes on the display adapters like connect/disconnect of displays
    fn on_changed(
        _sender: windows_core::Ref<DisplayManager>,
        args: windows_core::Ref<DisplayManagerChangedEventArgs>,
    ) -> windows_core::Result<()> {
        log::trace!("DisplayManager changed");
        Self::refresh_after_resume();

        // Critical!: app will crash if this is not set
        if let Some(args) = args.as_ref() {
            args.SetHandled(true)?;
        }
        Ok(())
    }

    fn on_paths_failed_or_invalidated(
        _sender: windows_core::Ref<DisplayManager>,
        args: windows_core::Ref<DisplayManagerPathsFailedOrInvalidatedEventArgs>,
    ) -> windows_core::Result<()> {
        log::trace!("DisplayManager paths failed or invalidated");
        // Treat this as a change event
        Self::refresh_after_resume();

        // Critical!: app will crash if this is not set
        if let Some(args) = args.as_ref() {
            args.SetHandled(true)?;
        }
        Ok(())
    }

    /// Diffs the current WinRT display state against the last known one, emitting
    /// `ViewAdded`/`ViewRemoved` for any real differences. Returns whether anything
    /// actually changed, so callers can avoid reacting to spurious notifications
    /// (WM_DISPLAYCHANGE and the WinRT display events can fire with no real topology change).
    fn check_for_display_changes() -> Result<bool> {
        if !DISPLAY_STACK_ENABLED.load(Ordering::Acquire) {
            return Err(
                "Display stack is suspended; preserving the last known desktop geometry".into(),
            );
        }
        let current_state = Self::instance()
            .display_manager
            .TryReadCurrentStateForAllTargets()?
            .State()?;

        let mut current_views = HashMap::new();
        for view in current_state.Views()? {
            let view = DisplayView::from(view);
            if !view.is_active().unwrap_or(false) {
                continue;
            }
            let id = match view.primary_target().and_then(|t| t.stable_id()) {
                Ok(id) => id,
                Err(_) => continue,
            };
            current_views.insert(id, view);
        }

        let physical_snapshot = Self::wait_for_win32_to_settle(&current_views)?;

        let mut old_views = Self::instance().state_views.to_hash_map();
        let current_ids: Vec<MonitorId> = current_views.keys().cloned().collect();
        // Publish both caches before any event; subscribers must never reconcile
        // new monitor IDs against the previous physical geometry.
        let state_data = &Self::instance().state_data;
        let mut changed = state_data.to_vec() != physical_snapshot;
        state_data.replace(physical_snapshot);
        Self::instance().state_views.replace(current_views);

        // new monitors were added
        for id in current_ids {
            if old_views.remove(&id).is_none() {
                changed = true;
                Self::send(MonitorManagerEvent::ViewAdded(id.clone()));
            }
        }

        // residuals were removed/disconnected
        for (id, _) in old_views {
            changed = true;
            Self::send(MonitorManagerEvent::ViewRemoved(id));
        }

        Ok(changed)
    }

    pub fn get_cached_ids(&self) -> Vec<MonitorId> {
        self.state_views.keys()
    }

    pub fn get_cached_data(&self) -> Vec<PhysicalMonitor> {
        self.state_data.to_vec()
    }
}

impl Drop for MonitorManager {
    fn drop(&mut self) {
        self.display_manager.Stop().log_error();

        if let Some(enabled_token) = self.enabled_token {
            self.display_manager
                .RemoveEnabled(enabled_token)
                .log_error();
        }

        if let Some(disabled_token) = self.disabled_token {
            self.display_manager
                .RemoveDisabled(disabled_token)
                .log_error();
        }

        if let Some(changed_token) = self.changed_token {
            self.display_manager
                .RemoveChanged(changed_token)
                .log_error();
        }

        if let Some(paths_failed_or_invalidated_token) = self.paths_failed_or_invalidated_token {
            self.display_manager
                .RemovePathsFailedOrInvalidated(paths_failed_or_invalidated_token)
                .log_error();
        }
    }
}
