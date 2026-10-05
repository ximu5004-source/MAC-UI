//! Samples the Windows wallpaper beneath toolbar groups and opted-in frosted
//! panels, never a screenshot. This estimates contrast when the backdrop contains
//! other applications; their pixels are deliberately not captured or inspected.
//! Decoding and COM stay off the UI/message-loop thread.
use crate::{
    error::Result,
    windows_api::{Com, ComThread},
};
use image::{ImageReader, RgbImage};
use slu_utils::{
    desktop_regions::physical_regions,
    wallpaper_contrast::{luminance, source_point},
};
use std::{path::PathBuf, sync::LazyLock, time::SystemTime};
use windows::{
    Win32::{
        Foundation::{HWND, RECT},
        UI::{
            Shell::{DesktopWallpaper, IDesktopWallpaper},
            WindowsAndMessaging::{GetClientRect, GetWindowRect},
        },
    },
    core::{PCWSTR, PWSTR},
};

struct OwnedString(PWSTR);
impl Drop for OwnedString {
    fn drop(&mut self) {
        Com::task_mem_free(self.0.0 as _);
    }
}

struct WallpaperImage {
    path: PathBuf,
    stamp: (Option<SystemTime>, u64),
    original: [u32; 2],
    pixels: RgbImage,
}
struct WallpaperState {
    shell: IDesktopWallpaper,
    images: Vec<WallpaperImage>,
}

static WALLPAPER: LazyLock<Result<ComThread<WallpaperState>>> = LazyLock::new(|| {
    ComThread::spawn("Toolbar wallpaper contrast", || {
        Ok(WallpaperState {
            shell: Com::create_instance(&DesktopWallpaper)?,
            images: Vec::new(),
        })
    })
});

impl WallpaperState {
    fn image(&mut self, path: &PathBuf) -> Result<&WallpaperImage> {
        let metadata = std::fs::metadata(path)?;
        let stamp = (metadata.modified().ok(), metadata.len());
        if let Some(index) = self
            .images
            .iter()
            .position(|image| image.path == *path && image.stamp == stamp)
        {
            return Ok(&self.images[index]);
        }
        self.images.retain(|image| image.path != *path);
        let mut reader = ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader.decode()?;
        let original = [decoded.width(), decoded.height()];
        let pixels = decoded.thumbnail(1024, 1024).to_rgb8();
        // Each cached thumbnail is <=3 MiB. Wallpaper slideshows cannot grow it forever.
        if self.images.len() >= 8 {
            self.images.remove(0);
        }
        self.images.push(WallpaperImage {
            path: path.clone(),
            stamp,
            original,
            pixels,
        });
        Ok(self.images.last().unwrap())
    }

    fn sample(&mut self, window: [i32; 4], regions: Vec<[i32; 4]>) -> Result<Vec<f64>> {
        let mode = unsafe { self.shell.GetPosition()? }.0;
        let color = unsafe { self.shell.GetBackgroundColor()? }.0;
        let background = luminance([color as u8, (color >> 8) as u8, (color >> 16) as u8]);
        let center = [(window[0] + window[2]) / 2, (window[1] + window[3]) / 2];
        let mut target = None;
        let mut span = RECT {
            left: i32::MAX,
            top: i32::MAX,
            right: i32::MIN,
            bottom: i32::MIN,
        };
        for index in 0..unsafe { self.shell.GetMonitorDevicePathCount()? } {
            let device = OwnedString(unsafe { self.shell.GetMonitorDevicePathAt(index)? });
            let rect = unsafe { self.shell.GetMonitorRECT(PCWSTR(device.0.0))? };
            span.left = span.left.min(rect.left);
            span.top = span.top.min(rect.top);
            span.right = span.right.max(rect.right);
            span.bottom = span.bottom.max(rect.bottom);
            if center[0] >= rect.left
                && center[0] < rect.right
                && center[1] >= rect.top
                && center[1] < rect.bottom
            {
                let path = OwnedString(unsafe { self.shell.GetWallpaper(PCWSTR(device.0.0))? });
                target = Some((rect, PathBuf::from(unsafe { path.0.to_string()? })));
            }
        }
        let Some((monitor, path)) = target else {
            return Ok(vec![background; regions.len()]);
        };
        let rect = if mode == 5 { span } else { monitor };
        let image = if path.as_os_str().is_empty() {
            None
        } else {
            Some(self.image(&path)?)
        };
        Ok(regions
            .into_iter()
            .map(|[left, top, right, bottom]| {
                let mut sum = 0.0;
                for y in 0..4 {
                    for x in 0..32 {
                        let point = [
                            f64::from(window[0] - rect.left + left)
                                + f64::from(right - left) * (f64::from(x) + 0.5) / 32.0,
                            f64::from(window[1] - rect.top + top)
                                + f64::from(bottom - top) * (f64::from(y) + 0.5) / 4.0,
                        ];
                        let source = image.and_then(|image| {
                            source_point(
                                image.original,
                                [
                                    f64::from(rect.right - rect.left),
                                    f64::from(rect.bottom - rect.top),
                                ],
                                point,
                                mode,
                            )
                            .map(|point| (image, point))
                        });
                        sum += source.map_or(background, |(image, [sx, sy])| {
                            let tx = (u64::from(sx) * u64::from(image.pixels.width())
                                / u64::from(image.original[0]))
                                as u32;
                            let ty = (u64::from(sy) * u64::from(image.pixels.height())
                                / u64::from(image.original[1]))
                                as u32;
                            luminance(image.pixels.get_pixel(tx, ty).0)
                        });
                    }
                }
                sum / 128.0
            })
            .collect())
    }
}

#[tauri::command(async)]
pub async fn get_toolbar_wallpaper_luminance(
    webview: tauri::WebviewWindow,
    viewport: [f64; 2],
    rects: Vec<[f64; 4]>,
) -> Result<Vec<f64>> {
    let label = super::super::webview::WidgetWebviewLabel::try_from_raw(webview.label())?;
    if label.widget_id.as_str() != "@seelen/fancy-toolbar" || rects.len() > 3 {
        return Err("Wallpaper contrast is limited to the toolbar's three groups".into());
    }
    sample_window_wallpaper(webview, viewport, rects, "toolbar").await
}

fn frosted_sampling_allowed(widget_id: &str, count: usize) -> bool {
    count <= 3
        && matches!(
            widget_id,
            "@seelen/quick-settings"
                | "@seelen/apps-menu"
                | "@seelen/media-popup"
                | "@seelen/flyouts"
                | "@seelen/keyboard-selector"
                | "@seelen/network-popup"
                | "@seelen/bluetooth-popup"
                | "@seelen/notifications"
                | "@seelen/system-tray"
        )
}

#[tauri::command(async)]
pub async fn get_frosted_wallpaper_luminance(
    webview: tauri::WebviewWindow,
    viewport: [f64; 2],
    rects: Vec<[f64; 4]>,
) -> Result<Vec<f64>> {
    let label = super::super::webview::WidgetWebviewLabel::try_from_raw(webview.label())?;
    if !frosted_sampling_allowed(label.widget_id.as_str(), rects.len()) {
        return Err("Wallpaper contrast is limited to three opted-in panel surfaces".into());
    }
    sample_window_wallpaper(webview, viewport, rects, "frosted panel").await
}

/// Share the exact wallpaper placement, thumbnail cache and sampling pipeline.
/// The native client/window rectangles locate these CSS surfaces on the actual
/// display; callers cannot request arbitrary paths or read other window pixels.
async fn sample_window_wallpaper(
    webview: tauri::WebviewWindow,
    viewport: [f64; 2],
    rects: Vec<[f64; 4]>,
    description: &'static str,
) -> Result<Vec<f64>> {
    let hwnd = HWND(webview.hwnd()?.0);
    let mut client = RECT::default();
    let mut window = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client)?;
        GetWindowRect(hwnd, &mut window)?;
    }
    let regions = physical_regions(viewport, [client.right, client.bottom], &rects)
        .ok_or_else(|| format!("Invalid {description} sampling geometry"))?;
    if regions.len() != rects.len() {
        return Err(format!("Empty {description} sampling region").into());
    }
    let window = [window.left, window.top, window.right, window.bottom];
    tokio::task::spawn_blocking(move || {
        let thread = WALLPAPER.as_ref().map_err(|error| error.to_string())?;
        thread.call(move |state| state.sample(window, regions))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::frosted_sampling_allowed;

    #[test]
    fn frosted_sampling_is_scoped_to_three_opted_in_panels() {
        for id in [
            "@seelen/quick-settings",
            "@seelen/apps-menu",
            "@seelen/media-popup",
            "@seelen/flyouts",
            "@seelen/keyboard-selector",
            "@seelen/network-popup",
            "@seelen/bluetooth-popup",
            "@seelen/notifications",
            "@seelen/system-tray",
        ] {
            assert!(frosted_sampling_allowed(id, 0));
            assert!(frosted_sampling_allowed(id, 1));
            assert!(frosted_sampling_allowed(id, 3));
            assert!(!frosted_sampling_allowed(id, 4));
        }
    }

    #[test]
    fn frosted_sampling_does_not_expand_toolbar_or_third_party_authority() {
        for id in [
            "@seelen/fancy-toolbar",
            "@seelen/desktop-shell",
            "@seelen/weg",
            "@someone/widget",
            "@seelen/flyouts?monitorId=test",
        ] {
            assert!(!frosted_sampling_allowed(id, 1));
        }
    }
}
