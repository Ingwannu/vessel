//! Staged desktop audio/DSP mechanisms, not an advertised device capability.
//!
//! The device thread drives `view::audio` ownership and owns the capture DSP.
//! Conversation playback wiring still removes this outer staging boundary.
//! These tests open no device.

mod callback;
mod device;
mod dsp;
mod format;
mod owner;
mod playback;
mod thread;

use std::num::TryFromIntError;

#[derive(Debug, thiserror::Error)]
pub(super) enum AudioError {
    #[error("audio ownership cannot be verified")]
    Ownership,
    #[error("audio capture is stopped")]
    Stopped,
    #[error(transparent)]
    Reservation(vessel_app_view::audio::CaptureRefusal),
    #[error("the device sample rate is unsupported")]
    Rate,
    #[error("the device must offer mono or stereo audio")]
    Channels,
    #[error("audio frame size does not match its validated format")]
    Frame,
    #[error("audio contains a non-finite sample")]
    NonFinite,
    #[error("audio count exceeds this platform's range")]
    Count(#[source] TryFromIntError),
    #[error("audio processing failed")]
    Processor(#[source] sonora::Error),
}
