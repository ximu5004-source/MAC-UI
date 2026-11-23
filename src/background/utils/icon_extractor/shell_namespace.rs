//! Native icons for direct Shell namespace children of the Windows desktop.
//! Parsing a PIDL and reading its image-list entry never invokes its Shell verbs.

use std::{path::Path, sync::LazyLock};

use image::RgbaImage;
use windows::{
    Win32::{
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::Com::CoTaskMemFree,
        UI::{
            Controls::{IImageList, ILD_TRANSPARENT},
            Shell::{
                Common::ITEMIDLIST, SHFILEINFOW, SHGFI_PIDL, SHGFI_SYSICONINDEX, SHGetFileInfoW,
                SHGetImageList, SHIL_JUMBO, SHParseDisplayName,
            },
        },
    },
    core::{Owned, PCWSTR},
};

use crate::{
    error::Result,
    windows_api::{ComThread, string_utils::WindowsString},
};

use super::{convert_hicon_to_rgba_image, crop_transparent_borders};

/// Only a direct desktop namespace identity is accepted, not a filesystem path,
/// shell URL/command, or a child path into a namespace. The original spelling is
/// retained as the icon-cache key.
pub(crate) fn is_namespace_path(path: &Path) -> bool {
    let Some(value) = path.to_str() else {
        return false;
    };
    let Some(guid) = value.strip_prefix("::{").and_then(|s| s.strip_suffix('}')) else {
        return false;
    };
    guid.len() == 36
        && guid.bytes().enumerate().all(|(index, ch)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                ch == b'-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}

struct Pidl(*mut ITEMIDLIST);

impl Drop for Pidl {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0.cast())) };
    }
}

pub(super) fn get_icon(path: &Path) -> Result<RgbaImage> {
    if !is_namespace_path(path) {
        return Err("Expected a direct desktop Shell namespace path".into());
    }
    static ICON_STA: LazyLock<Result<ComThread<()>>> =
        LazyLock::new(|| ComThread::spawn("Desktop namespace icons", || Ok(())));
    let sta = ICON_STA
        .as_ref()
        .map_err(|err| format!("Desktop namespace icon apartment unavailable: {err}"))?;
    let path = path.to_owned();
    sta.call(move |_| unsafe {
        let name = WindowsString::from(path);
        let mut pidl = Pidl(std::ptr::null_mut());
        SHParseDisplayName(name.as_pcwstr(), None, &mut pidl.0, 0, None)?;
        if pidl.0.is_null() {
            return Err("Shell namespace parsing returned an empty PIDL".into());
        }
        let mut info = SHFILEINFOW::default();
        // With SHGFI_PIDL this argument is a PIDL, not a UTF-16 path string.
        let result = SHGetFileInfoW(
            PCWSTR(pidl.0.cast()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_PIDL | SHGFI_SYSICONINDEX,
        );
        if result == 0 {
            return Err("Failed to read the Shell namespace icon index".into());
        }
        // Index zero is valid for namespace items; unlike generic files, it
        // must not cause a visible system item to lose its native icon.
        let images: IImageList = SHGetImageList(SHIL_JUMBO as i32)?;
        let icon = Owned::new(images.GetIcon(info.iIcon, ILD_TRANSPARENT.0)?);
        Ok(crop_transparent_borders(&convert_hicon_to_rgba_image(
            &icon,
        )?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_namespace_identity_excludes_commands_and_filesystem_children() {
        assert!(is_namespace_path(Path::new(
            "::{645FF040-5081-101B-9F08-00AA002F954E}"
        )));
        assert!(is_namespace_path(Path::new(
            "::{20d04fe0-3aea-1069-a2d8-08002b30309d}"
        )));
        for value in [
            "shell:RecycleBinFolder",
            "::{645FF040-5081-101B-9F08-00AA002F954E}\\child",
            "::{645FF0405081101B9F0800AA002F954E}",
            "::{645FF040-5081-101B-9F08-00AA002F954X}",
            "::{645FF040-5081-101B-9F08-00AA002F954E}\0",
            "C:\\desktop\\file.txt",
            "",
        ] {
            assert!(!is_namespace_path(Path::new(value)), "{value:?}");
        }
    }

    #[test]
    #[ignore = "Explicit opt-in: reads one fixed Windows namespace icon, writes only a temporary owned image"]
    fn fixed_system_namespace_icon_decodes_and_roundtrips() {
        let image = get_icon(Path::new("::{645FF040-5081-101B-9F08-00AA002F954E}"))
            .expect("read the built-in Recycle Bin icon without invoking it");
        assert!(image.width() > 0 && image.height() > 0);
        assert!(image.pixels().any(|pixel| pixel.0[3] != 0));
        struct OwnedProbeDirectory(std::path::PathBuf);
        impl Drop for OwnedProbeDirectory {
            fn drop(&mut self) {
                std::fs::remove_file(self.0.join("native.webp")).ok();
                std::fs::remove_dir(&self.0).ok();
            }
        }
        let directory = OwnedProbeDirectory(std::env::temp_dir().join(format!(
            "macui-owned-namespace-icon-probe-{}",
            uuid::Uuid::new_v4()
        )));
        std::fs::create_dir(&directory.0).unwrap();
        let path = directory.0.join("native.webp");
        super::super::save_as_webp(&image, &path).unwrap();
        let decoded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), image.dimensions());
        let alpha_differences = image
            .pixels()
            .zip(decoded.pixels())
            .filter(|(original, cached)| original.0[3] != cached.0[3])
            .count();
        let visible_rgb_differences = image
            .pixels()
            .zip(decoded.pixels())
            .filter(|(original, cached)| original.0[3] != 0 && original.0[..3] != cached.0[..3])
            .count();
        let transparent_rgb_differences = image
            .pixels()
            .zip(decoded.pixels())
            .filter(|(original, cached)| original.0[3] == 0 && original.0[..3] != cached.0[..3])
            .count();
        eprintln!(
            "Native namespace icon {}x{}; cache alpha differences={alpha_differences}, visible RGB differences={visible_rgb_differences}, fully transparent RGB normalizations={transparent_rgb_differences}",
            image.width(),
            image.height()
        );
        // The existing WebP encoder may normalize RGB under fully transparent
        // pixels. All alpha and every visible (including partial-alpha) color
        // must survive the real cache codec exactly.
        assert_eq!(alpha_differences, 0);
        assert_eq!(visible_rgb_differences, 0);
    }
}
