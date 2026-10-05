use windows::Win32::{
    Foundation::{LPARAM, POINT, WPARAM},
    UI::WindowsAndMessaging::{
        GetCursorPos, HWND_BROADCAST, RegisterWindowMessageW, SendNotifyMessageW,
    },
};

use windows_core::w;

pub struct Util;
impl Util {
    /// Packs two 16-bit values into a 32-bit value. This is commonly used
    /// for `WPARAM` and `LPARAM` values.
    ///
    /// Equivalent to the Win32 `MAKELPARAM` and `MAKEWPARAM` macros.
    pub fn pack_i32(low: i16, high: i16) -> i32 {
        i32::from(low as u16) | (i32::from(high as u16) << 16)
    }

    /// Gets the mouse position in screen coordinates.
    pub fn cursor_position() -> crate::Result<(i32, i32)> {
        let mut point = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut point) }?;
        Ok((point.x, point.y))
    }

    /// Refreshes the icons of the tray.
    ///
    /// Simulates the Windows taskbar being re-created. Some windows fail to
    /// re-add their icons, in which case it's an implementation error on
    /// their side. These windows that fail also do not re-add their icons
    /// to the Windows taskbar when `explorer.exe` is restarted ordinarily.
    pub fn refresh_icons() -> crate::Result<()> {
        log::info!("Refreshing icons by sending `TaskbarCreated` message.");
        let msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        if msg == 0 {
            return Err("Failed to register message".into());
        }
        unsafe { SendNotifyMessageW(HWND_BROADCAST, msg, WPARAM::default(), LPARAM::default()) }?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Util;

    #[test]
    fn negative_monitor_coordinates_do_not_overwrite_the_other_word() {
        for (x, y) in [(-1920, 450), (320, -1080), (-1920, -1080), (100, 200)] {
            let packed = Util::pack_i32(x, y);
            assert_eq!((packed & 0xffff) as i16, x);
            assert_eq!((packed >> 16) as i16, y);
        }
    }

    #[test]
    fn high_bit_icon_ids_and_message_values_remain_unsigned_words() {
        let packed = Util::pack_i32(0x0202, 0xffff_u16 as i16) as u32;
        assert_eq!(packed & 0xffff, 0x0202);
        assert_eq!(packed >> 16, 0xffff);
    }
}
