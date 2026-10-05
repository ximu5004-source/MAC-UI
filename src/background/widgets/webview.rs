use std::path::PathBuf;

use base64::Engine;
use seelen_core::{
    resource::WidgetId,
    state::{Widget, WidgetLoader, WidgetPreset},
    system_state::MonitorId,
};

use tauri::{Emitter, Manager};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::{
        Dwm::{DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DwmEnableBlurBehindWindow},
        Gdi::{CreateRectRgn, DeleteObject},
    },
    UI::WindowsAndMessaging::{GetClientRect, WS_CHILD, WS_EX_NOREDIRECTIONBITMAP},
};

use crate::{
    app::get_app_handle,
    error::{Result, ResultLogExt},
    state::application::FULL_STATE,
    utils::constants::SEELEN_COMMON,
    windows_api::WindowsApi,
};

pub struct WidgetWebview(pub tauri::WebviewWindow);

fn restore_native_transparency(hwnd: HWND) -> Result<bool> {
    if !slu_utils::display_recovery::should_restore_native_transparency(
        WindowsApi::get_styles(hwnd).contains(WS_CHILD),
        WindowsApi::get_ex_styles(hwnd).contains(WS_EX_NOREDIRECTIONBITMAP),
    ) {
        return Ok(false);
    }
    unsafe {
        // Reapply precisely Tao's transparent-window initialization. Display/GPU
        // recovery can invalidate the native alpha surface while bounds stay valid.
        let region = CreateRectRgn(0, 0, -1, -1);
        if region.is_invalid() {
            return Err("Could not create transparent window region".into());
        }
        let result = DwmEnableBlurBehindWindow(
            hwnd,
            &DWM_BLURBEHIND {
                dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                fEnable: true.into(),
                hRgnBlur: region,
                fTransitionOnMaximized: false.into(),
            },
        );
        // The caller owns HRGN; release it even when DWM rejects the operation.
        DeleteObject(region.into()).ok()?;
        result?;
    }
    Ok(true)
}

/// Repair transparency and the controller viewport after display recovery. A
/// correct HWND rectangle alone does not prevent a stale opaque composition
/// surface from covering everything underneath a transparent overlay.
pub fn restore_webview_bounds(window: &tauri::WebviewWindow) -> Result<()> {
    let label = WidgetWebviewLabel::try_from_raw(window.label())
        .map(|label| label.decoded)
        .unwrap_or_else(|_| window.label().to_owned());
    // Wry holds the WebView dispatcher's window_id mutex while invoking a
    // with_webview callback inline on the UI thread. Calling another Tauri
    // WebView method inside that callback locks it again and deadlocks.
    // Resolve the HWND and set only the WebView's background before entering it.
    // The WebviewWindow background setter also paints Win32 RGB (ignoring alpha).
    let native_handle = window.hwnd()?.0 as isize;
    window
        .as_ref()
        .set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)))?;
    let recovery_window = window.clone();
    // with_webview dispatches to the owning UI thread. No widget/deployment locks
    // or synchronous COM completion waits are held around this operation.
    window.with_webview(move |webview| {
        let result = (|| -> Result<()> {
            let native_reapplied = match restore_native_transparency(HWND(native_handle as _)) {
                Ok(reapplied) => reapplied,
                Err(error) => {
                    log::warn!("Could not restore native transparency for {label}: {error}");
                    false
                }
            };
            log::debug!("Reapplied WebView transparency for {label}; native DWM restored: {native_reapplied}");
            let controller = webview.controller();
            unsafe {
                // Infer WebView2's Windows 0.61 types; our native APIs use 0.62.
                let mut parent = Default::default();
                controller.ParentWindow(&mut parent).map_err(|error| error.to_string())?;
                let mut client = RECT::default();
                GetClientRect(HWND(parent.0), &mut client)?;
                let Some((width, height)) = slu_utils::display_recovery::physical_client_viewport(
                    client.left, client.top, client.right, client.bottom,
                ) else {
                    return Ok(());
                };
                let mut bounds = Default::default();
                controller.Bounds(&mut bounds).map_err(|error| error.to_string())?;
                let previous = (bounds.right - bounds.left, bounds.bottom - bounds.top);
                bounds.left = 0;
                bounds.top = 0;
                bounds.right = width;
                bounds.bottom = height;
                controller.SetBounds(bounds).map_err(|error| error.to_string())?;
                controller.NotifyParentWindowPositionChanged().map_err(|error| error.to_string())?;
                if previous != (width, height) {
                    log::debug!("Restored WebView2 viewport for {label}: {previous:?} -> ({width}, {height})");
                }
            }
            // The CSS viewport has now received its corrected physical bounds.
            // Rebuild cached native surfaces only after SetBounds, then request
            // fresh DOM geometry even when the final screen size is unchanged.
            super::frosted::restore(HWND(native_handle as _));
            // Emit can evaluate JavaScript and take Wry's window-id mutex. Never
            // do that inside this with_webview callback: queue it from a detached
            // worker, without joining or retaining widget/deployment locks.
            std::thread::spawn(move || {
                recovery_window.emit("mac-native-frost-recover", ()).log_error();
            });
            Ok(())
        })();
        if let Err(error) = result {
            log::warn!("Could not restore WebView2 surface for {label}: {error}");
        }
    })?;
    Ok(())
}

pub fn restore_webviews_after_display_change() {
    // Tauri returns cloned handles, so its registry is also unlocked before
    // dispatch. This covers the desktop, toolbar, Dock and surviving popup pages.
    for window in get_app_handle().webview_windows().into_values() {
        restore_webview_bounds(&window).log_error();
    }
}

impl WidgetWebview {
    pub fn create(
        widget: &Widget,
        label: &WidgetWebviewLabel,
        owner_hwnd: Option<isize>,
    ) -> Result<Self> {
        let state = FULL_STATE.load();
        let title = widget.metadata.display_name.get(state.locale());

        let args = WebviewArgs::create(
            state.settings.hardware_acceleration || widget.force_hardware_acceleration,
            state.settings.unstable_optimizations,
        );

        let url = match widget.loader {
            WidgetLoader::Legacy => {
                return Err("Legacy widgets are not supported by the new widget loader".into());
            }
            WidgetLoader::InternalReact => {
                let resource_name = widget
                    .id
                    .resource_name()
                    .ok_or("Can't get internal resource path")?;
                tauri::WebviewUrl::App(format!("react/{resource_name}/index.html").into())
            }
            WidgetLoader::Internal => {
                let resource_name = widget
                    .id
                    .resource_name()
                    .ok_or("Can't get internal resource path")?;
                tauri::WebviewUrl::App(format!("svelte/{resource_name}/index.html").into())
            }
            WidgetLoader::ThirdParty => {
                tauri::WebviewUrl::App("vanilla/third_party/index.html".into())
            }
        };

        let mut builder = tauri::WebviewWindowBuilder::new(get_app_handle(), &label.raw, url)
            .title(title)
            .transparent(true)
            .visible(false);

        if matches!(
            widget.preset,
            WidgetPreset::Desktop | WidgetPreset::Overlay | WidgetPreset::Popup
        ) {
            builder = builder
                .decorations(false)
                .shadow(false)
                .skip_taskbar(true)
                .minimizable(false)
                .maximizable(false)
                .closable(false);
        }

        // Desktop widgets must remain focusable so replacement-shell controls can receive
        // pointer and keyboard input. Overlay widgets stay non-activating.
        if matches!(widget.preset, WidgetPreset::Overlay) {
            builder = builder.focusable(false).focused(false);
        }

        match widget.preset {
            WidgetPreset::Desktop => {
                builder = builder.focused(false);
                // Wallpaper is a WorkerW child; the independent desktop controls its own z-order.
                if !matches!(
                    widget.id.as_str(),
                    "@seelen/wallpaper-manager" | "@seelen/desktop-shell"
                ) {
                    builder = builder.always_on_bottom(true);
                }
            }
            WidgetPreset::Overlay | WidgetPreset::Popup => {
                builder = builder.always_on_top(true).resizable(false);
            }
            _ => {}
        }

        if let Some(owner) = owner_hwnd {
            // SAFETY: HWND in windows 0.61 (tauri) and 0.62 (ours) share the same memory layout
            #[allow(clippy::missing_transmute_annotations)]
            {
                builder = builder.owner_raw(unsafe { std::mem::transmute(owner) });
            }
        }

        let window = builder
            .data_directory(args.data_directory())
            .additional_browser_args(&args.to_string())
            .build()?;

        let frosted_owner = window.hwnd()?.0 as isize;
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                super::frosted::destroyed(frosted_owner);
            }
        });

        if widget.id.as_str() == "@seelen/desktop-shell" {
            let owner = window.hwnd()?.0 as isize;
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    super::desktop_shell::window_destroyed(owner);
                }
            });
        }

        // Widgets handle their own show/hide animations, avoid the ones from the system.
        // SAFETY: HWND in windows 0.61 (tauri) and 0.62 (ours) share the same memory layout
        WindowsApi::set_system_transitions_disabled(HWND(window.hwnd()?.0), true).log_error();
        Ok(Self(window))
    }

    pub fn reload(&self) {
        let window = self.0.clone();
        std::thread::spawn(move || {
            window.reload().log_error();
        });
    }
}

impl Drop for WidgetWebview {
    fn drop(&mut self) {
        // Pods are removed while the widget maps are locked. Destruction may wait
        // for the UI thread and must not occur inside those locks or Destroyed.
        let window = self.0.clone();
        std::thread::spawn(move || {
            if let Some(active) = get_app_handle().get_webview_window(window.label()) {
                if let (Ok(active_hwnd), Ok(retired_hwnd)) = (active.hwnd(), window.hwnd()) {
                    // A later incarnation may already have reused this label.
                    if active_hwnd == retired_hwnd {
                        let _ = window.destroy();
                    }
                }
            }
        });
    }
}

// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetWebviewLabel {
    /// this should be used as the real webview label
    pub raw: String,
    /// this is the decoded label, useful for debugging and logging
    pub decoded: String,
    /// widget id from this label was created
    pub widget_id: WidgetId,
    pub monitor_id: Option<MonitorId>,
    pub instance_id: Option<uuid::Uuid>,
}

impl WidgetWebviewLabel {
    pub fn new(
        widget_id: &WidgetId,
        monitor_id: Option<&str>,
        instance_id: Option<&uuid::Uuid>,
    ) -> Self {
        let mut label = widget_id.to_string();
        let with_monitor_id = monitor_id.is_some();
        let with_instance_id = instance_id.is_some();
        if with_monitor_id || with_instance_id {
            label.push('?');
        }

        if let Some(monitor_id) = monitor_id {
            label.push_str(&format!("monitorId={}", urlencoding::encode(monitor_id)));
        }

        if let Some(instance_id) = instance_id {
            if with_monitor_id {
                label.push('&');
            }
            label.push_str(&format!(
                "instanceId={}",
                urlencoding::encode(&instance_id.to_string())
            ));
        }

        Self {
            raw: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&label),
            decoded: label,
            widget_id: widget_id.clone(),
            monitor_id: monitor_id.map(MonitorId::from),
            instance_id: instance_id.cloned(),
        }
    }

    pub fn try_from_raw(raw: &str) -> Result<Self> {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(raw)?;
        let decoded = String::from_utf8(decoded)?;

        let mut parts = decoded.splitn(2, '?');
        let widget_id = WidgetId::from(parts.next().expect("Invalid label"));

        let mut monitor_id = None;
        let mut instance_id = None;
        if let Some(query) = parts.next() {
            for param in query.split('&') {
                if let Some(value) = param.strip_prefix("monitorId=") {
                    let decoded_value = urlencoding::decode(value).unwrap_or_default();
                    monitor_id = Some(MonitorId::from(decoded_value.as_ref()));
                } else if let Some(value) = param.strip_prefix("instanceId=") {
                    let decoded_value = urlencoding::decode(value).unwrap_or_default();
                    instance_id = decoded_value.parse::<uuid::Uuid>().ok();
                }
            }
        }

        Ok(Self {
            raw: raw.to_string(),
            decoded,
            widget_id,
            monitor_id,
            instance_id,
        })
    }
}

impl std::fmt::Display for WidgetWebviewLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.decoded)
    }
}

// =============================================================================

pub struct WebviewArgs {
    args: Vec<String>,
    with_gpu: bool,
    unstable: bool,
}

impl WebviewArgs {
    const BASE_ARGS: &[&str] = &[
        "--disable-features=translate,msWebOOUI,msPdfOOUI,msSmartScreenProtection,RendererAppContainer,BackForwardCache,InterestCohort,SharedArrayBuffer,CalculateNativeWinOcclusion,OptimizationHints,AutofillServerCommunication,PaintHolding",
        "--no-first-run",
        "--disable-site-isolation-trials",
        "--disk-cache-size=0",
        "--disable-application-cache",
        "--media-cache-size=0",
        "--disable-extensions",
        "--disable-component-extensions-with-background-pages",
        "--disable-ipc-flooding-protection",
        "--disable-breakpad",
        "--disable-crash-reporter",
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        // prevents the browser from lowering the CPU priority of invisible windows, oposite of what we want
        // "--disable-renderer-backgrounding"
        "--disable-sync",
        "--no-pings",
        // maybe causes more resources than it reduces
        // "--aggressive-cache-discard",
    ];

    const GPU_ARGS: &[&str] = &[
        "--enable-gpu",
        "--enable-accelerated-video-decode",
        "--enable-gpu-rasterization",
        "--enable-zero-copy",
        "--enable-native-gpu-memory-buffers",
        "--enable-oop-rasterization",
        "--use-angle=d3d11", // Media Foundation + DXVA + D3D11 in windows is the most optimized
    ];

    const PERFORMANCE_ARGS: &[&str] = &[
        // unstable flag that causes more issues than it solves
        // "--enable-low-end-device-mode",
        // this completely removes the gpu process
        "--in-process-gpu",
        "--disable-gpu",
        "--disable-gpu-compositing",
        "--disable-gpu-shader-disk-cache",
        "--disable-accelerated-video-encode",
        "--disable-gpu-rasterization",
        "--disable-software-rasterizer",
    ];

    const UNSTABLE_OPTIMIZATIONS: &[&str] = &[
        // this reduces ram usage but if a widget crashes it will crash
        // all widgets with the same loader, so it's not worth it
        "--process-per-site",
    ];

    pub fn create(with_gpu: bool, unstable_optimizations: bool) -> Self {
        let mut args: Vec<String> = Self::BASE_ARGS.iter().map(|s| s.to_string()).collect();

        if with_gpu {
            args.extend(Self::GPU_ARGS.iter().map(|s| s.to_string()));
        } else {
            args.extend(Self::PERFORMANCE_ARGS.iter().map(|s| s.to_string()));
        };

        if unstable_optimizations {
            args.extend(Self::UNSTABLE_OPTIMIZATIONS.iter().map(|s| s.to_string()));
        }

        Self {
            args,
            with_gpu,
            unstable: unstable_optimizations,
        }
    }

    pub fn data_directory(&self) -> PathBuf {
        let foldername = match (self.with_gpu, self.unstable) {
            (true, true) => "gpu-unstable",
            (true, false) => "gpu",
            (false, true) => "no-gpu-unstable",
            (false, false) => "no-gpu",
        };

        SEELEN_COMMON.app_cache_dir().join(foldername)
    }
}

impl std::fmt::Display for WebviewArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.args.join(" "))
    }
}
