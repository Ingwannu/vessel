//! One ordered microphone stream composes the existing speech policies.
//!
//! Registry/actor admission precedes this local owner. The detector parameter
//! must produce real model probabilities in production; tests use typed scores.
//! A new speech onset cancels previous-turn work before any new final STT job.

use std::{error::Error, fmt};
use tokio_util::sync::CancellationToken;
use vessel_proto::voice::{CaptureAck, CaptureVoice, VoiceLease, VoiceSeq};

use super::{
    boundary::{Boundary, BoundaryEvent},
    detector_input::{PushError, Window, Windows},
    finalize::Finalization,
    input_buffer::{BufferError, InputBuffer},
    probability::{Gate, Probability},
};

pub(super) trait Detector {
    type Error: Error + Send + Sync + 'static;
    fn probability(&mut self, window: &Window) -> Result<Probability, Self::Error>;
    /// Discard all recurrent state; this does not flush retained speech.
    fn reset(&mut self);
}

#[derive(Debug)]
pub(super) enum Transition {
    None,
    SpeechStarted,
    Finalizing(Finalization),
}
#[derive(Debug)]
pub(super) struct Accepted {
    pub(super) ack: CaptureAck,
    pub(super) transition: Transition,
}
#[derive(thiserror::Error)]
pub(super) enum Failure<E: Error + 'static> {
    #[error("voice turn is stopped")]
    Stopped,
    #[error("voice capture generation does not match")]
    Lease,
    #[error("voice capture sequence is not contiguous")]
    Sequence,
    #[error("voice capture sequence exhausted")]
    Exhausted,
    #[error("voice detector input failed")]
    Input(#[source] super::detector_input::InputError),
    #[error("voice utterance buffering failed")]
    Buffer(#[from] BufferError),
    #[error("voice probability clock exhausted")]
    Probability(#[from] super::probability::ClockExhausted),
    #[error("voice boundary clock exhausted")]
    Boundary(#[from] super::boundary::ClockExhausted),
    #[error("voice inference failed")]
    Detector(#[source] E),
    #[error("voice final transcription could not start")]
    Finalization(#[from] super::finalize::Failure),
    #[error("voice detector produced conflicting boundaries")]
    Order,
}
// A future runtime may include paths/tensors in its cause. Diagnostics name the
// rule only; neither Debug nor Display automatically expands a foreign cause.
impl<E: Error + 'static> fmt::Debug for Failure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub(super) struct Turn<D: Detector> {
    lease: VoiceLease,
    next: VoiceSeq,
    cancel: CancellationToken,
    work: CancellationToken,
    detector: D,
    windows: Windows,
    gate: Gate,
    boundary: Boundary,
    buffer: InputBuffer,
}
impl<D: Detector> fmt::Debug for Turn<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceTurn")
            .field("lease", &self.lease)
            .field("next", &self.next)
            .field("windows", &self.windows)
            .field("buffer", &self.buffer)
            .finish_non_exhaustive()
    }
}
impl<D: Detector> Turn<D> {
    pub(super) fn new(lease: VoiceLease, parent: &CancellationToken, detector: D) -> Self {
        let cancel = parent.child_token();
        Self {
            lease,
            next: VoiceSeq::ZERO,
            work: cancel.child_token(),
            cancel,
            detector,
            windows: Windows::default(),
            gate: Gate::default(),
            boundary: Boundary::default(),
            buffer: InputBuffer::default(),
        }
    }
    pub(super) fn append(&mut self, request: &CaptureVoice) -> Result<Accepted, Failure<D::Error>> {
        if self.cancel.is_cancelled() {
            self.stop();
            return Err(Failure::Stopped);
        }
        // A foreign/stale/gapped packet must not mutate this active stream or
        // call the detector. Transport policy decides whether to close its peer.
        if request.lease != self.lease {
            return Err(Failure::Lease);
        }
        if request.seq != self.next {
            return Err(Failure::Sequence);
        }
        let Some(next) = self.next.next() else {
            self.stop();
            return Err(Failure::Exhausted);
        };
        match self.process(request) {
            Ok(transition) => {
                self.next = next;
                Ok(Accepted {
                    ack: CaptureAck {
                        through: request.seq,
                    },
                    transition,
                })
            }
            Err(error) => {
                self.stop();
                Err(error)
            }
        }
    }
    fn process(&mut self, request: &CaptureVoice) -> Result<Transition, Failure<D::Error>> {
        self.buffer.append(&request.pcm)?;
        let Self {
            detector,
            windows,
            gate,
            boundary,
            buffer,
            cancel,
            work,
            ..
        } = self;
        let mut transition = Transition::None;
        let result = windows.try_push(&request.pcm, cancel, |window| {
            let probability = detector.probability(window).map_err(Failure::Detector)?;
            let event = boundary.observe(gate.observe(probability)?)?;
            match event {
                BoundaryEvent::NoChange => return Ok(()),
                BoundaryEvent::SpeechStarted => {
                    work.cancel();
                    *work = cancel.child_token();
                    buffer.confirmed()?;
                    if !matches!(transition, Transition::None) {
                        return Err(Failure::Order);
                    }
                    transition = Transition::SpeechStarted;
                }
                BoundaryEvent::SpeechStopped => {
                    let utterance = buffer.seal().ok_or(Failure::Order)?;
                    if !matches!(transition, Transition::None) {
                        return Err(Failure::Order);
                    }
                    transition = Transition::Finalizing(Finalization::new(utterance, work)?);
                }
            }
            Ok(())
        });
        match result {
            Ok(()) => Ok(transition),
            Err(PushError::Input(error)) => Err(Failure::Input(error)),
            Err(PushError::Observer(error)) => Err(error),
        }
    }
    /// Stop cancels STT children and clears every retained sample without flush.
    pub(super) fn stop(&mut self) {
        self.cancel.cancel();
        self.work.cancel();
        self.windows.clear();
        self.buffer.discard();
        self.gate.reset();
        self.boundary = Boundary::default();
        self.detector.reset();
    }
}
impl<D: Detector> Drop for Turn<D> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests;
