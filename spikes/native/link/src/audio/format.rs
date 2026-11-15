//! The initial 10 ms processing contract is validated before processor allocation.

/// Invalid device metadata is refused before processor allocation.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// Rate must support exact10ms frames within8–192kHz.
    #[error("the device sample rate is unsupported")]
    Rate,
    /// Only mono/stereo can be converted by this backend.
    #[error("the device must offer mono or stereo audio")]
    Channels,
    /// A frame count cannot be represented on this platform.
    #[error("audio count exceeds this platform's range")]
    Count(#[source] std::num::TryFromIntError),
}

const MIN_RATE_HZ: u32 = 8_000;
const MAX_RATE_HZ: u32 = 192_000;
const FRAMES_PER_SECOND: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Validated samples per second.
pub struct RateHz(u32);

impl RateHz {
    /// Validate a device rate before allocating10ms processor buffers.
    ///
    /// # Errors
    /// Outside8–192kHz, or a rate without an integral10ms frame count.
    pub fn new(raw: u32) -> Result<Self, FormatError> {
        if !(MIN_RATE_HZ..=MAX_RATE_HZ).contains(&raw) || !raw.is_multiple_of(FRAMES_PER_SECOND) {
            return Err(FormatError::Rate);
        }
        Ok(Self(raw))
    }

    /// Samples per second.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

pub(super) const NORMALIZED_RATE: RateHz = RateHz(16_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Validated channel layout.
pub enum Channels {
    /// One audio channel.
    Mono,
    /// Left and right channels.
    Stereo,
}

impl TryFrom<u16> for Channels {
    type Error = FormatError;
    fn try_from(raw: u16) -> Result<Self, Self::Error> {
        match raw {
            1 => Ok(Self::Mono),
            2 => Ok(Self::Stereo),
            _ => Err(FormatError::Channels),
        }
    }
}

impl Channels {
    /// Number of interleaved channels.
    #[must_use]
    pub const fn get(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Samples per channel in one10ms processing block.
pub struct FrameCount(usize);

impl FrameCount {
    /// Samples per channel in one processing block.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Validated local PCM processing metadata, never a wire negotiation.
pub struct Format {
    rate: RateHz,
    channels: Channels,
    frames: FrameCount,
}

impl Format {
    pub(super) const fn capture() -> Self {
        Self {
            rate: NORMALIZED_RATE,
            channels: Channels::Mono,
            frames: FrameCount(160),
        }
    }
    /// Build checked local metadata from rate and channel units.
    ///
    /// # Errors
    /// The platform cannot represent the derived frame count.
    pub fn new(rate: RateHz, channels: Channels) -> Result<Self, FormatError> {
        let frames = usize::try_from(rate.get().div_euclid(FRAMES_PER_SECOND))
            .map_err(FormatError::Count)?;
        Ok(Self {
            rate,
            channels,
            frames: FrameCount(frames),
        })
    }
    /// Validated sampling frequency.
    #[must_use]
    pub const fn rate(self) -> RateHz {
        self.rate
    }
    /// Samples per channel per10ms.
    #[must_use]
    pub const fn frames(self) -> FrameCount {
        self.frames
    }
    /// Interleaved channel layout.
    #[must_use]
    pub const fn channels(self) -> Channels {
        self.channels
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub(super) fn config(self) -> sonora::StreamConfig {
        sonora::StreamConfig::new(self.rate.get(), self.channels.get())
    }
}
