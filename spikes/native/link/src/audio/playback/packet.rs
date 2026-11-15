//! One collected PCM16 mono segment, decoded in bounded 10 ms pieces.

use std::{fmt, sync::Arc, time::Duration};

use bytes::Bytes;
use cpal::Sample as _;

use super::{
    Refusal,
    control::{Access, Reservation},
    receipts::{Lease, Receipt},
};
use crate::audio::format::RateHz;

const MAX_PACKET_BYTES: usize = 8 * 1024 * 1024;
const BLOCKS_PER_SECOND: u32 = 100;
const MAX_PACKET_TIME: Duration = Duration::from_secs(180);
const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub(in crate::audio) struct Packet {
    pcm: Bytes,
    rate: RateHz,
    duration: Duration,
    access: Option<Access>,
    reservation: Option<Reservation>,
}

impl Packet {
    pub(in crate::audio) fn new(pcm: Bytes, rate: RateHz) -> Result<Self, Refusal> {
        if pcm.is_empty()
            || !pcm.len().is_multiple_of(size_of::<i16>())
            || pcm.len() > MAX_PACKET_BYTES
        {
            return Err(Refusal::Format);
        }
        let frames = u64::try_from(pcm.len().div_euclid(size_of::<i16>()))
            .map_err(|_private| Refusal::Format)?;
        let hz = u64::from(rate.get());
        let nanos = frames
            .checked_mul(NANOS_PER_SECOND)
            .and_then(|count| count.checked_add(hz.saturating_sub(1)))
            .and_then(|count| count.checked_div(hz))
            .ok_or(Refusal::Format)?;
        let duration = Duration::from_nanos(nanos);
        if duration > MAX_PACKET_TIME {
            return Err(Refusal::Format);
        }
        Ok(Self {
            pcm,
            rate,
            duration,
            access: None,
            reservation: None,
        })
    }

    pub(super) fn admit(
        mut self,
        access: Access,
        permit: tokio::sync::OwnedSemaphorePermit,
    ) -> Self {
        self.access = Some(access);
        self.reservation = Some(Arc::new(permit));
        self
    }
    pub(super) fn access(&self) -> Option<Access> {
        self.access.clone()
    }
    pub(super) fn stopped(&self) -> bool {
        self.access.as_ref().is_some_and(Access::stopped)
    }
    pub(super) fn duration(&self) -> Duration {
        self.duration
    }
    pub(super) fn reader(&self, lease: Lease, receipt: Arc<Receipt>) -> Reader {
        Reader {
            lease,
            pcm: self.pcm.clone(),
            rate: self.rate,
            receipt,
            _reservation: self.reservation.clone(),
        }
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Packet")
            .field("bytes", &self.pcm.len())
            .field("rate", &self.rate)
            .field("duration", &self.duration)
            .finish()
    }
}

/// The final block is zero-padded only after its valid PCM, never dropped.
pub(in crate::audio) struct Reader {
    lease: Lease,
    pcm: Bytes,
    rate: RateHz,
    receipt: Arc<Receipt>,
    _reservation: Option<Reservation>,
}

impl Reader {
    pub(super) fn lease(&self) -> Lease {
        self.lease
    }
    pub(super) fn receipt(&self) -> Arc<Receipt> {
        Arc::clone(&self.receipt)
    }
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
            .finish_non_exhaustive()
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
