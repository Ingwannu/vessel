//! Staged desktop audio/DSP mechanisms, not an advertised device capability.
//!
//! A real process-wide device actor will drive view::audio ownership and
//! use this processor on its dedicated thread. These tests open no device.

mod dsp;
mod format;

use std::num::TryFromIntError;

#[derive(Debug, thiserror::Error)]
pub(super) enum AudioError {
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
