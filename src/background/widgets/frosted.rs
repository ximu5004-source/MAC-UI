//! Live, neutral backdrop frost, painted *behind* the WebView. CSS backdrop-filter
//! cannot sample other applications. This uses the documented Win32 host backdrop
//! permission and Windows Composition effect pipeline instead; no screen capture,
//! process injection, whole-window acrylic, or modification of native input regions.

use std::{cell::RefCell, collections::HashMap};
use tauri::Manager;

use windows::{
    Foundation::{IPropertyValue, PropertyValue},
    Graphics::Effects::{
        IGraphicsEffect, IGraphicsEffect_Impl, IGraphicsEffectSource, IGraphicsEffectSource_Impl,
    },
    System::{DispatcherQueue, DispatcherQueueController},
    UI::Composition::{
        CompositionEffectBrush, CompositionEffectSourceParameter, Compositor, ContainerVisual,
        Desktop::DesktopWindowTarget,
    },
    Win32::{
        Foundation::{E_INVALIDARG, E_NOTIMPL, HWND, RECT},
        Graphics::Dwm::{DWMWA_USE_HOSTBACKDROPBRUSH, DwmSetWindowAttribute},
        System::WinRT::{
            Composition::ICompositorDesktopInterop,
            CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT,
            DispatcherQueueOptions,
            Graphics::Direct2D::{
                GRAPHICS_EFFECT_PROPERTY_MAPPING, IGraphicsEffectD2D1Interop,
                IGraphicsEffectD2D1Interop_Impl,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
};
use windows_core::{GUID, HSTRING, Interface, PCWSTR, implement};

use crate::error::{Result, ResultLogExt};

type Snapshot = ([f64; 2], Vec<[f64; 5]>);

#[cfg(test)]
#[path = "frosted/qa_backdrop.rs"]
mod qa_backdrop;

fn supports_frosted_widget(widget_id: &str) -> bool {
    matches!(
        widget_id,
        "@seelen/desktop-shell"
            | "@seelen/apps-menu"
            | "@seelen/weg"
            | "@seelen/quick-settings"
            | "@seelen/media-popup"
            | "@seelen/flyouts"
            | "@seelen/keyboard-selector"
            | "@seelen/network-popup"
            | "@seelen/bluetooth-popup"
            | "@seelen/calendar-popup"
            | "@seelen/user-menu"
            | "@seelen/power-menu"
            | "@seelen/notifications"
            | "@seelen/system-tray"
    )
}

thread_local! {
    // All WinRT composition objects are confined to the Tauri window thread.
    static SURFACES: RefCell<HashMap<isize, NativeFrost>> = RefCell::new(HashMap::new());
    static QUEUE: RefCell<Option<DispatcherQueueController>> = const { RefCell::new(None) };
}

// The SDK GaussianBlur effect contract is implemented directly so this small
// native brush does not require Win2D/XAML or third-party injected module code.
#[implement(IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectD2D1Interop)]
struct GaussianBlur {
    source: IGraphicsEffectSource,
    amount: f32,
}

impl IGraphicsEffect_Impl for GaussianBlur_Impl {
    fn Name(&self) -> windows_core::Result<HSTRING> {
        Ok(HSTRING::from("Frost"))
    }
    fn SetName(&self, _name: &HSTRING) -> windows_core::Result<()> {
        Err(E_NOTIMPL.into())
    }
}
impl IGraphicsEffectSource_Impl for GaussianBlur_Impl {}
impl IGraphicsEffectD2D1Interop_Impl for GaussianBlur_Impl {
    fn GetEffectId(&self) -> windows_core::Result<GUID> {
        Ok(GUID::from_u128(0x1feb6d69_2fe6_4ac9_8c58_1d7f93e7a6a5))
    }
    fn GetNamedPropertyMapping(
        &self,
        _name: &PCWSTR,
        _index: *mut u32,
        _mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING,
    ) -> windows_core::Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn GetPropertyCount(&self) -> windows_core::Result<u32> {
        Ok(3)
    }
    fn GetProperty(&self, index: u32) -> windows_core::Result<IPropertyValue> {
        match index {
            0 => PropertyValue::CreateSingle(self.amount)?.cast(),
            1 => PropertyValue::CreateUInt32(1)?.cast(), // balanced optimization
            2 => PropertyValue::CreateUInt32(1)?.cast(), // hard border: no dark transparent fringe
            _ => Err(E_INVALIDARG.into()),
        }
    }
    fn GetSource(&self, index: u32) -> windows_core::Result<IGraphicsEffectSource> {
        if index == 0 {
            Ok(self.source.clone())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
    fn GetSourceCount(&self) -> windows_core::Result<u32> {
        Ok(1)
    }
}

struct NativeFrost {
    compositor: Compositor,
    target: DesktopWindowTarget,
    root: ContainerVisual,
    brush: CompositionEffectBrush,
    snapshot: Snapshot,
    scale: f32,
}

impl Drop for NativeFrost {
    fn drop(&mut self) {
        // Close the target first to disconnect from the HWND before releasing its
        // source. A destroyed/reused HWND must never retain an old effect tree.
        let _ = self.target.Close();
        let _ = self.brush.Close();
        let _ = self.root.Close();
        let _ = self.compositor.Close();
    }
}

fn ensure_dispatcher_queue() -> Result<()> {
    if DispatcherQueue::GetForCurrentThread().is_ok() {
        return Ok(());
    }
    QUEUE.with(|slot| -> Result<()> {
        if slot.borrow().is_none() {
            let queue = unsafe {
                CreateDispatcherQueueController(DispatcherQueueOptions {
                    dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
                    threadType: DQTYPE_THREAD_CURRENT,
                    apartmentType: DQTAT_COM_NONE,
                })?
            };
            *slot.borrow_mut() = Some(queue);
        }
        Ok(())
    })
}

fn enable_host_backdrop(hwnd: HWND) -> Result<()> {
    // Windows 11 build 22000+. Rejected APIs propagate as an unavailable effect,
    // never as an opaque full-window substitute or a fake static wallpaper.
    let enabled = 1i32;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_HOSTBACKDROPBRUSH,
            &enabled as *const _ as _,
            std::mem::size_of_val(&enabled) as u32,
        )?;
    }
    Ok(())
}

impl NativeFrost {
    fn create(hwnd: HWND, snapshot: Snapshot, scale: f32) -> Result<Self> {
        ensure_dispatcher_queue()?;
        enable_host_backdrop(hwnd)?;
        let compositor = Compositor::new()?;
        let source = CompositionEffectSourceParameter::Create(&HSTRING::from("Desktop"))?;
        let effect: IGraphicsEffect = GaussianBlur {
            source: source.cast()?,
            amount: 18.0 * scale,
        }
        .into();
        let factory = compositor.CreateEffectFactory(&effect)?;
        let brush = factory.CreateBrush()?;
        brush.SetSourceParameter(
            &HSTRING::from("Desktop"),
            &compositor.CreateHostBackdropBrush()?,
        )?;
        let target = unsafe {
            compositor
                .cast::<ICompositorDesktopInterop>()?
                .CreateDesktopWindowTarget(hwnd, false)?
        };
        let root = compositor.CreateContainerVisual()?;
        let state = Self {
            compositor,
            target,
            root,
            brush,
            snapshot,
            scale,
        };
        // From this point any API error invokes our explicit Close cleanup, not
        // merely release of temporary COM references to an attached target.
        state.target.SetRoot(&state.root)?;
        Ok(state)
    }

    fn paint(&mut self, client: [i32; 2]) -> Result<()> {
        let regions =
            slu_utils::frosted_regions::physical_regions(self.snapshot.0, client, &self.snapshot.1)
                .ok_or("Invalid native frosted surface geometry")?;
        let children = self.root.Children()?;
        // Build a complete replacement subtree before publishing it, so failed
        // allocation does not leave a half-updated panel at its old bounds.
        let next = self.compositor.CreateContainerVisual()?;
        for region in regions {
            let visual = self.compositor.CreateSpriteVisual()?;
            let mut size = visual.Size()?;
            size.X = region.width;
            size.Y = region.height;
            visual.SetSize(size)?;
            let mut offset = visual.Offset()?;
            offset.X = region.left;
            offset.Y = region.top;
            visual.SetOffset(offset)?;
            let rounded = self.compositor.CreateRoundedRectangleGeometry()?;
            rounded.SetSize(size)?;
            let mut radius = rounded.CornerRadius()?;
            radius.X = region.radius_x;
            radius.Y = region.radius_y;
            rounded.SetCornerRadius(radius)?;
            visual.SetClip(&self.compositor.CreateGeometricClipWithGeometry(&rounded)?)?;
            visual.SetBrush(&self.brush)?;
            next.Children()?.InsertAtTop(&visual)?;
        }
        children.RemoveAll()?;
        children.InsertAtTop(&next)?;
        Ok(())
    }
}

fn update(hwnd: HWND, snapshot: Snapshot) -> Result<()> {
    let mut bounds = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut bounds)?;
    }
    let client = [bounds.right - bounds.left, bounds.bottom - bounds.top];
    slu_utils::frosted_regions::physical_regions(snapshot.0, client, &snapshot.1)
        .ok_or("Invalid native frosted surface geometry")?;
    let scale = (client[0] as f64 / snapshot.0[0]).min(client[1] as f64 / snapshot.0[1]) as f32;
    SURFACES.with(|surfaces| -> Result<()> {
        let mut surfaces = surfaces.borrow_mut();
        let key = hwnd.0 as isize;
        if snapshot.1.is_empty() {
            // OSDs retain their transparent HWND between notifications. Removing
            // the composition target here is required to remove its native blur.
            if surfaces.remove(&key).is_some() {
                log::debug!("Removed native frosted surfaces for HWND {key}");
            }
            return Ok(());
        }
        if surfaces
            .get(&key)
            .is_some_and(|surface| (surface.scale - scale).abs() > 0.01)
        {
            surfaces.remove(&key);
        }
        if !surfaces.contains_key(&key) {
            surfaces.insert(key, NativeFrost::create(hwnd, snapshot.clone(), scale)?);
            log::debug!(
                "Created live host-backdrop frost for HWND {key}, scale={scale}, blur=18 CSS px"
            );
        }
        let surface = surfaces.get_mut(&key).unwrap();
        surface.snapshot = snapshot;
        if let Err(error) = surface.paint(client) {
            surfaces.remove(&key);
            return Err(error);
        }
        Ok(())
    })
}

#[tauri::command(async)]
pub async fn set_frosted_regions(
    webview: tauri::WebviewWindow,
    viewport: [f64; 2],
    rects: Vec<[f64; 5]>,
) -> Result<()> {
    let label = super::webview::WidgetWebviewLabel::try_from_raw(webview.label())?;
    if !supports_frosted_widget(label.widget_id.as_str()) {
        return Err("This widget does not own native frosted surfaces".into());
    }
    slu_utils::frosted_regions::physical_regions(viewport, [1, 1], &rects)
        .ok_or("Invalid native frosted surface geometry")?;
    let handle = webview.hwnd()?.0 as isize;
    let raw_label = webview.label().to_owned();
    let (tx, rx) = tokio::sync::oneshot::channel();
    webview.run_on_main_thread(move || {
        let result = (|| -> Result<()> {
            // Dispatch may outlive a hidden/destroyed popup. Never apply an old
            // request to a reused HWND, including another process's window.
            let current = crate::app::get_app_handle()
                .get_webview_window(&raw_label)
                .ok_or("Native frosted window is no longer active")?;
            if current.hwnd()?.0 as isize != handle {
                return Err("Native frosted window owner changed".into());
            }
            update(HWND(handle as _), (viewport, rects))
        })();
        let _ = tx.send(result);
    })?;
    rx.await.map_err(|error| error.to_string())?
}

/// Called on the WebView's owning thread after its native alpha surface recovers.
pub fn restore(hwnd: HWND) {
    let snapshot = SURFACES.with(|surfaces| {
        surfaces
            .borrow_mut()
            .remove(&(hwnd.0 as isize))
            .map(|surface| surface.snapshot.clone())
    });
    if let Some(snapshot) = snapshot {
        update(hwnd, snapshot).log_error();
    }
}

/// Called on the window thread, not from Drop while widget registry locks are held.
pub fn destroyed(handle: isize) {
    SURFACES.with(|surfaces| {
        surfaces.borrow_mut().remove(&handle);
    });
}

#[cfg(test)]
mod tests {
    use super::supports_frosted_widget;

    #[test]
    fn only_opted_in_widgets_can_install_native_frost() {
        for id in [
            "@seelen/desktop-shell",
            "@seelen/apps-menu",
            "@seelen/weg",
            "@seelen/quick-settings",
            "@seelen/media-popup",
            "@seelen/flyouts",
            "@seelen/keyboard-selector",
            "@seelen/network-popup",
            "@seelen/bluetooth-popup",
            "@seelen/calendar-popup",
            "@seelen/user-menu",
            "@seelen/power-menu",
            "@seelen/notifications",
            "@seelen/system-tray",
        ] {
            assert!(supports_frosted_widget(id));
        }
        for id in [
            "@seelen/context-menu",
            "@third-party/widget",
            "@seelen/notifications?other=true",
            "@seelen/calendar-popup?other=true",
            "@seelen/power-menu?other=true",
        ] {
            assert!(!supports_frosted_widget(id));
        }
    }
}
