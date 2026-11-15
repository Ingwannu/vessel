//! One collected PCM16 mono segment, decoded in bounded 10 ms pieces.

use std::{fmt, sync::Arc};

use bytes::Bytes;
use cpal::Sample as _;

use super::{Refusal, receipts::Receipt};
use crate::audio::format::RateHz;

const MAX_PACKET_BYTES: usize = 8 * 1024 * 1024;
const BLOCKS_PER_SECOND: u32 = 100;

pub(in crate::audio) struct Packet {
    pcm: Bytes,
    rate: RateHz,
}

impl Packet {
    pub(in crate::audio) fn new(pcm: Bytes, rate: RateHz) -> Result<Self, Refusal> {
        if pcm.is_empty()
            || !pcm.len().is_multiple_of(size_of::<i16>())
            || pcm.len() > MAX_PACKET_BYTES
        {
            return Err(Refusal::Format);
        }
        Ok(Self { pcm, rate })
    }

    pub(super) fn reader(&self, receipt: Arc<Receipt>) -> Reader {
        Reader {
            pcm: self.pcm.clone(),
            rate: self.rate,
            receipt,
        }
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Packet")
            .field("bytes", &self.pcm.len())
            .field("rate", &self.rate)
            .finish()
    }
}

/// The final block is zero-padded only after its valid PCM, never dropped.
pub(in crate::audio) struct Reader {
    pcm: Bytes,
    rate: RateHz,
    receipt: Arc<Receipt>,
}

impl Reader {
    pub(in crate::audio) fn rate(&self) -> RateHz {
        self.rate
    }

    pub(in crate::audio) fn next(&mut self) -> Result<Option<Block>, Refusal> {
        if self.receipt.stopped() {
            return Err(Refusal::Stopped);
        }
        if self.pcm.is_empty() {
            return Ok(None);
        }
        let frames = usize::try_from(self.rate.get().div_euclid(BLOCKS_PER_SECOND))
            .map_err(|_private| Refusal::Format)?;
        let bytes = frames.saturating_mul(size_of::<i16>()).min(self.pcm.len());
        let part = self.pcm.split_to(bytes);
        let mut mono = Vec::new();
        mono.try_reserve_exact(frames)
            .map_err(|_private| Refusal::Capacity)?;
        for sample in part.as_chunks::<2>().0 {
            mono.push(i16::from_le_bytes(*sample).to_sample::<f32>());
        }
        let valid_frames = mono.len();
        mono.resize(frames, 0.0);
        if self.receipt.stopped() {
            return Err(Refusal::Stopped);
        }
        Ok(Some(Block {
            mono,
            valid_frames,
            last: self.pcm.is_empty(),
        }))
    }

    pub(in crate::audio) fn stopped(&self) -> bool {
        self.receipt.stopped()
    }
}

pub(in crate::audio) struct Block {
    mono: Vec<f32>,
    valid_frames: usize,
    last: bool,
}

impl Block {
    pub(in crate::audio) fn mono(&self) -> &[f32] {
        &self.mono
    }
    pub(in crate::audio) fn valid_frames(&self) -> usize {
        self.valid_frames
    }
    pub(in crate::audio) fn last(&self) -> bool {
        self.last
    }
}
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlaybackBlock")
            .field("frames", &self.mono.len())
            .field("last", &self.last)
            .finish()
    }
}

impl fmt::Debug for Reader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlaybackReader")
            .field("remaining_bytes", &self.pcm.len())
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}
