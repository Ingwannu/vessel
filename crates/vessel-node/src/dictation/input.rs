//! Accepted PCM ordering and shape, without resampling, providers or I/O.
//!
//! One current chunk is separate from bounded gap holds. A short contiguous tail
//! advances acknowledgement but cannot reach the provider until finish names it.

use std::collections::BTreeMap;
use vessel_proto::dictation::{ChunkSeq, DictationPcm, MAX_SEQ, SampleRate};

const MAX_HELD_CHUNKS: usize = 32;
const MAX_HELD_BYTES: usize = 1_048_576;
const SAMPLE_BYTES: usize = size_of::<i16>();

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
/// Distinguish call-only shape refusals from terminal resource failures.
pub(super) enum InputError {
    #[error("the dictation chunk or finish has an invalid audio shape")]
    BadAudio,
    #[error("the dictation sequence or duration exceeds its stream bound")]
    TooLong,
    #[error(
        "the dictation reorder hold exceeds {MAX_HELD_CHUNKS} chunks or {MAX_HELD_BYTES} bytes"
    )]
    ReorderOverflow,
    #[error("the dictation stream has no accepted audio")]
    NoAudio,
    #[error("dictation input counts exceeded this platform's range")]
    Count(#[source] std::num::TryFromIntError),
}

/// Keep accepted and forwarded cursors separate while provider work awaits.
pub(super) struct Input {
    second: usize,
    accepted: u64,
    limit: u64,
    forward: ChunkSeq,
    acknowledge: ChunkSeq,
    current: Option<DictationPcm>,
    held: BTreeMap<ChunkSeq, DictationPcm>,
    held_bytes: usize,
    highest: Option<ChunkSeq>,
    tail: Option<ChunkSeq>,
    finish: Option<ChunkSeq>,
}

impl Input {
    /// Derive input bounds from the parsed rate before spawning a stream.
    pub(super) fn new(rate: SampleRate) -> Result<Self, InputError> {
        let second = usize::try_from(rate.get())
            .map_err(InputError::Count)?
            .saturating_mul(SAMPLE_BYTES)
            .saturating_mul(super::MAX_CHUNK_SECONDS);
        // The sequence is zero based and every full chunk has this duration.
        // Validated rates and caps keep these saturating products far below u64.
        let seconds = u64::from(MAX_SEQ.get().saturating_add(1))
            .saturating_mul(u64::try_from(super::MAX_CHUNK_SECONDS).map_err(InputError::Count)?);
        let limit = u64::from(rate.get())
            .saturating_mul(u64::try_from(SAMPLE_BYTES).map_err(InputError::Count)?)
            .saturating_mul(seconds);
        Ok(Self {
            second,
            limit,
            accepted: 0,
            forward: ChunkSeq::new(0),
            acknowledge: ChunkSeq::new(0),
            current: None,
            held: BTreeMap::new(),
            held_bytes: 0,
            highest: None,
            tail: None,
            finish: None,
        })
    }

    /// Refuse before storing so a rejected call cannot corrupt ordering.
    pub(super) fn accept(
        &mut self,
        seq: ChunkSeq,
        pcm: DictationPcm,
    ) -> Result<Option<ChunkSeq>, InputError> {
        // The terminal sequence bound precedes duplicate and shape checks.
        if seq > MAX_SEQ {
            return Err(InputError::TooLong);
        }
        if self.contains(seq) {
            return Ok(self.ack());
        }
        self.shape(seq, &pcm)?;
        let bytes = u64::try_from(pcm.as_slice().len()).map_err(InputError::Count)?;
        let accepted = self.accepted.saturating_add(bytes);
        if accepted > self.limit {
            return Err(InputError::TooLong);
        }
        if seq == self.forward {
            self.current = Some(pcm);
        } else {
            let held_bytes = self.held_bytes.saturating_add(pcm.as_slice().len());
            if self.held.len() >= MAX_HELD_CHUNKS || held_bytes > MAX_HELD_BYTES {
                return Err(InputError::ReorderOverflow);
            }
            self.held_bytes = held_bytes;
            // contains() already excluded this key while we hold exclusive state.
            let _previous = self.held.insert(seq, pcm);
        }
        self.accepted = accepted;
        self.highest = Some(self.highest.map_or(seq, |old| old.max(seq)));
        // Shape was checked before storing, so the accepted short chunk is
        // unique and above every earlier accepted sequence.
        let short = self
            .chunk(seq)
            .is_some_and(|pcm| pcm.as_slice().len() < self.second);
        if short {
            self.tail = Some(seq);
        }
        while self.contains(self.acknowledge) {
            self.acknowledge = ChunkSeq::new(self.acknowledge.get().saturating_add(1));
        }
        Ok(self.ack())
    }

    fn shape(&self, seq: ChunkSeq, pcm: &DictationPcm) -> Result<(), InputError> {
        let length = pcm.as_slice().len();
        let short = length < self.second;
        if length > self.second
            || self
                .finish
                .is_some_and(|last| seq > last || (short && seq != last))
            || self.tail.is_some_and(|tail| short || seq > tail)
            || (short && self.highest.is_some_and(|high| seq <= high))
        {
            return Err(InputError::BadAudio);
        }
        Ok(())
    }

    /// Pin final metadata before any held tail can be forwarded.
    pub(super) fn finish(&mut self, last: ChunkSeq) -> Result<(), InputError> {
        if last > MAX_SEQ {
            return Err(InputError::TooLong);
        }
        if self.highest.is_none() {
            return Err(InputError::NoAudio);
        }
        if self.finish.is_some_and(|prior| prior != last)
            || self.tail.is_some_and(|tail| tail != last)
            || self.highest.is_some_and(|high| high > last)
        {
            return Err(InputError::BadAudio);
        }
        self.finish = Some(last);
        Ok(())
    }

    /// Release one contiguous chunk so commit bounds apply after each append.
    pub(super) fn pop(&mut self) -> Option<DictationPcm> {
        if self.tail == Some(self.forward) && self.finish != Some(self.forward) {
            return None;
        }
        let pcm = self.current.take()?;
        self.forward = ChunkSeq::new(self.forward.get().saturating_add(1));
        self.current = self.held.remove(&self.forward);
        if let Some(current) = &self.current {
            self.held_bytes = self.held_bytes.saturating_sub(current.as_slice().len());
        }
        Some(pcm)
    }

    /// A short tail stays acknowledged until finish releases it.
    pub(super) fn forwardable(&self) -> bool {
        self.current.is_some()
            && (self.tail != Some(self.forward) || self.finish == Some(self.forward))
    }
    /// Finalization waits for the named tail, including gaps accepted earlier.
    pub(super) fn all_forwarded(&self) -> bool {
        self.finish.is_some_and(|last| self.forward > last)
    }
    /// The actor freezes its first finish estimate against this metadata.
    pub(super) fn final_seq(&self) -> Option<ChunkSeq> {
        self.finish
    }

    fn contains(&self, seq: ChunkSeq) -> bool {
        seq < self.forward || self.chunk(seq).is_some()
    }
    fn chunk(&self, seq: ChunkSeq) -> Option<&DictationPcm> {
        if seq == self.forward {
            self.current.as_ref()
        } else {
            self.held.get(&seq)
        }
    }
    /// Expose accepted contiguous progress independently of provider completion.
    pub(super) fn ack(&self) -> Option<ChunkSeq> {
        self.acknowledge.get().checked_sub(1).map(ChunkSeq::new)
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
