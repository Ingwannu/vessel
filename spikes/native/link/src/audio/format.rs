//! The initial 10 ms processing contract is validated before processor allocation.

use super::AudioError;

const MIN_RATE_HZ: u32 = 8_000;
const MAX_RATE_HZ: u32 = 192_000;
const FRAMES_PER_SECOND: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RateHz(u32);

impl RateHz {
    pub(super) fn new(raw: u32) -> Result<Self, AudioError> {
        if !(MIN_RATE_HZ..=MAX_RATE_HZ).contains(&raw) || !raw.is_multiple_of(FRAMES_PER_SECOND) {
            return Err(AudioError::Rate);
        }
        Ok(Self(raw))
    }

    pub(super) const fn get(self) -> u32 {
        self.0
    }
}

pub(super) const NORMALIZED_RATE: RateHz = RateHz(16_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Channels {
    Mono,
    Stereo,
}

impl TryFrom<u16> for Channels {
    type Error = AudioError;
    fn try_from(raw: u16) -> Result<Self, Self::Error> {
        match raw {
            1 => Ok(Self::Mono),
            2 => Ok(Self::Stereo),
            _ => Err(AudioError::Channels),
        }
    }
}

impl Channels {
    pub(super) const fn get(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrameCount(usize);

impl FrameCount {
    pub(super) const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Format {
    rate: RateHz,
    channels: Channels,
    frames: FrameCount,
}

impl Format {
    pub(super) fn new(rate: RateHz, channels: Channels) -> Result<Self, AudioError> {
        let frames =
            usize::try_from(rate.get().div_euclid(FRAMES_PER_SECOND)).map_err(AudioError::Count)?;
        Ok(Self {
            rate,
            channels,
            frames: FrameCount(frames),
        })
    }
    pub(super) const fn frames(self) -> FrameCount {
        self.frames
    }
    pub(super) const fn channels(self) -> Channels {
        self.channels
    }
    pub(super) const fn rate(self) -> RateHz {
        self.rate
    }
    pub(super) fn config(self) -> sonora::StreamConfig {
        sonora::StreamConfig::new(self.rate.get(), self.channels.get())
    }
}
