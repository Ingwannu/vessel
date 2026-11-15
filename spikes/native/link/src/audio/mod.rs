//! Shared native microphone ownership; conversation output remains staged.
//!
//! The device thread drives `view::audio` ownership and owns the capture DSP.
//! The public API exposes dictation capture; conversation acquisition is
//! refused until duplex playback is wired. Tests open no device.

mod budget;
mod upload_access;
pub use budget::CAPTURE_QUEUE_BYTES;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use upload_access::Interval as UploadInterval;
mod capture;
pub use capture::{AcquireError, Capture, CaptureControl, CaptureState, DeviceFailure, Frame};
pub use format::{Channels, Format, FormatError, FrameCount, RateHz};

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod callback;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod device;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod dsp;
#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod duplex;
mod format;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod owner;
#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod playback;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod thread;

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, thiserror::Error)]
pub(super) enum AudioError {
    #[error("audio ownership cannot be verified")]
    Ownership,
    #[error("audio capture is stopped")]
    Stopped,
    #[error(transparent)]
    Reservation(vessel_app_view::audio::CaptureRefusal),
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("audio frame size does not match its validated format")]
    Frame,
    #[error("audio contains a non-finite sample")]
    NonFinite,
    #[error("audio sample is outside the normalized device range")]
    Range,
    #[error("audio processing failed")]
    Processor(#[source] sonora::Error),
}

#[cfg(test)]
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) mod testing;
