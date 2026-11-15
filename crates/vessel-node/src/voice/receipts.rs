//! Playback is complete only after the matching generation's explicit device receipt.
//!
//! Bounded tombstones make retries idempotent without reopening a completed
//! speak. A late Played cannot turn an interrupted/expired packet into success.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use vessel_proto::voice::{Playback, PlaybackId, PlaybackOutcome, PlayedVoice, VoiceLease};

const MAX_PENDING: usize = 2;
const MAX_CLOSED: usize = 256;
const CLOSED_TTL: Duration = Duration::from_secs(10);
const PLAYBACK_GRACE: Duration = Duration::from_secs(30);
const MAX_RECEIPT_WAIT: Duration = Duration::from_secs(210);
const SAMPLES_PER_PCM16: usize = size_of::<i16>();
const NANOS_PER_SECOND: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum ReceiptError {
    #[error("playback belongs to a different voice generation")]
    Foreign,
    #[error("playback receipt identity is unknown")]
    Unknown,
    #[error("playback receipt conflicts with its settled outcome")]
    Conflict,
    #[error("playback identity cannot be reused")]
    Reused,
    #[error("playback capacity or duration exceeds its limit")]
    Capacity,
    #[error("playback completion arrived after its deadline")]
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Completion {
    Settled(PlaybackOutcome),
    Duplicate,
}

#[derive(Debug)]
struct Closed {
    id: PlaybackId,
    outcome: PlaybackOutcome,
    at: Instant,
}

#[derive(Debug)]
pub(super) struct Receipts {
    lease: VoiceLease,
    pending: HashMap<PlaybackId, Instant>,
    closed: VecDeque<Closed>,
}

impl Receipts {
    pub(super) fn new(lease: VoiceLease) -> Self {
        Self {
            lease,
            pending: HashMap::new(),
            closed: VecDeque::new(),
        }
    }

    pub(super) fn open(&mut self, playback: &Playback, now: Instant) -> Result<(), ReceiptError> {
        self.prune(now);
        if self.pending.contains_key(&playback.id)
            || self.closed.iter().any(|entry| entry.id == playback.id)
        {
            return Err(ReceiptError::Reused);
        }
        if self.pending.len() >= MAX_PENDING {
            return Err(ReceiptError::Capacity);
        }
        let wait = wait_duration(playback)?;
        let deadline = now.checked_add(wait).ok_or(ReceiptError::Capacity)?;
        let _previous = self.pending.insert(playback.id, deadline);
        Ok(())
    }

    pub(super) fn complete(
        &mut self,
        receipt: PlayedVoice,
        now: Instant,
    ) -> Result<Completion, ReceiptError> {
        if receipt.lease != self.lease {
            return Err(ReceiptError::Foreign);
        }
        self.prune(now);
        if let Some(closed) = self
            .closed
            .iter()
            .find(|entry| entry.id == receipt.playback)
        {
            return if receipt.outcome == closed.outcome {
                Ok(Completion::Duplicate)
            } else {
                Err(ReceiptError::Conflict)
            };
        }
        let deadline = self
            .pending
            .remove(&receipt.playback)
            .ok_or(ReceiptError::Unknown)?;
        if now >= deadline {
            self.close(receipt.playback, PlaybackOutcome::Failed, now);
            return Err(ReceiptError::TimedOut);
        }
        self.close(receipt.playback, receipt.outcome, now);
        Ok(Completion::Settled(receipt.outcome))
    }

    pub(super) fn expire(&mut self, now: Instant) -> Vec<PlaybackId> {
        self.prune(now);
        let expired: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(&id, &deadline)| (now >= deadline).then_some(id))
            .collect();
        for &id in &expired {
            let _previous = self.pending.remove(&id);
            self.close(id, PlaybackOutcome::Failed, now);
        }
        expired
    }

    pub(super) fn interrupt(&mut self, now: Instant) -> Vec<PlaybackId> {
        self.prune(now);
        let pending: Vec<_> = self.pending.drain().map(|(id, _deadline)| id).collect();
        for &id in &pending {
            self.close(id, PlaybackOutcome::Interrupted, now);
        }
        pending
    }

    fn close(&mut self, id: PlaybackId, outcome: PlaybackOutcome, at: Instant) {
        if self.closed.len() >= MAX_CLOSED {
            let _oldest = self.closed.pop_front();
        }
        self.closed.push_back(Closed { id, outcome, at });
    }

    fn prune(&mut self, now: Instant) {
        while self
            .closed
            .front()
            .is_some_and(|entry| now.saturating_duration_since(entry.at) >= CLOSED_TTL)
        {
            let _expired = self.closed.pop_front();
        }
    }
}

/// The pipeline and the receipt book use the same bounded PCM-duration rule.
pub(super) fn wait_duration(playback: &Playback) -> Result<Duration, ReceiptError> {
    let samples = u64::try_from(playback.pcm.as_slice().len().div_euclid(SAMPLES_PER_PCM16))
        .map_err(|_private| ReceiptError::Capacity)?;
    let nanos = samples
        .checked_mul(NANOS_PER_SECOND)
        .ok_or(ReceiptError::Capacity)?
        .div_ceil(u64::from(playback.format.rate().get()));
    let wait = Duration::from_nanos(nanos)
        .checked_add(PLAYBACK_GRACE)
        .ok_or(ReceiptError::Capacity)?;
    if wait > MAX_RECEIPT_WAIT {
        return Err(ReceiptError::Capacity);
    }
    Ok(wait)
}

#[cfg(test)]
mod tests;
