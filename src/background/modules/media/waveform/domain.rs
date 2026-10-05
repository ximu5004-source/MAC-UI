use windows::Win32::Media::Audio::{IAudioCaptureClient, IAudioClient};

/// Owns the WASAPI loopback session COM pointers for the capture thread.
/// Exclusively held by a single capture thread; never shared.
pub struct CaptureSession {
    pub audio_client: IAudioClient,
    pub capture_client: IAudioCaptureClient,
    /// Number of channels in the device's native mix format.
    pub channels: u16,
    /// Sample rate reported by GetMixFormat, needed for Hz-accurate FFT bins.
    pub sample_rate: u32,
}

// IAudioClient and IAudioCaptureClient are COM interface pointers.
// The capture thread owns them exclusively after initialization.
unsafe impl Send for CaptureSession {}
unsafe impl Sync for CaptureSession {}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        unsafe {
            let _ = self.audio_client.Stop();
        }
    }
}
