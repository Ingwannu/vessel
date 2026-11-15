//! Voice-only transient output: two audio packets, sixteen controls, one waiter.
//!
//! The owning actor calls these synchronous methods before its next await.
//! A canceled oneshot restores the exact update; successful send is response
//! readiness, not network delivery or audible playback. PlayedVoice remains
//! the separate authority for releasing a synthesis receipt.

use std::collections::VecDeque;
use tokio::sync::oneshot;
use vessel_proto::voice::{VoiceFailure, VoiceUpdate};

const MAX_AUDIO: usize = 2;
const MAX_CONTROL: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Closed {
    Stopped,
    Failed(VoiceFailure),
}
impl Closed {
    fn update(self) -> VoiceUpdate {
        match self {
            Self::Stopped => VoiceUpdate::Stopped,
            Self::Failed(reason) => VoiceUpdate::Failed { reason },
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Open,
    Ending(Closed),
    Consumed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Audio,
    Control,
}
impl Kind {
    fn of(update: &VoiceUpdate) -> Option<Self> {
        match update {
            VoiceUpdate::Audio { .. } => Some(Self::Audio),
            VoiceUpdate::Input { .. }
            | VoiceUpdate::Transcription { .. }
            | VoiceUpdate::Turn { .. } => Some(Self::Control),
            VoiceUpdate::Failed { .. } | VoiceUpdate::Stopped => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum ReadRefusal {
    #[error("a voice update wait is already pending")]
    Waiting,
    #[error("the voice update stream is closed")]
    Gone,
}
#[derive(Debug, thiserror::Error)]
pub(super) enum PublishRefusal {
    #[error("voice output capacity is exhausted")]
    Capacity(VoiceUpdate),
    #[error("the voice output stream is closed")]
    Closed(VoiceUpdate),
    #[error("terminal output must close the voice generation")]
    Terminal(VoiceUpdate),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delivery {
    Waiting,
    UpdateReady,
    TerminalReady,
}

#[derive(Debug)]
pub(super) struct Updates {
    queue: VecDeque<VoiceUpdate>,
    audio: usize,
    controls: usize,
    waiter: Option<oneshot::Sender<VoiceUpdate>>,
    phase: Phase,
}
impl Default for Updates {
    fn default() -> Self {
        Self {
            queue: VecDeque::with_capacity(MAX_AUDIO.saturating_add(MAX_CONTROL)),
            audio: 0,
            controls: 0,
            waiter: None,
            phase: Phase::Open,
        }
    }
}
impl Updates {
    pub(super) fn publish(&mut self, update: VoiceUpdate) -> Result<(), PublishRefusal> {
        if self.phase != Phase::Open {
            return Err(PublishRefusal::Closed(update));
        }
        let Some(kind) = Kind::of(&update) else {
            return Err(PublishRefusal::Terminal(update));
        };
        let count = match kind {
            Kind::Audio => &mut self.audio,
            Kind::Control => &mut self.controls,
        };
        let maximum = match kind {
            Kind::Audio => MAX_AUDIO,
            Kind::Control => MAX_CONTROL,
        };
        if *count >= maximum {
            return Err(PublishRefusal::Capacity(update));
        }
        *count = count.saturating_add(1);
        self.queue.push_back(update);
        Ok(())
    }
    /// Stop/failure discards unsent audio independently of data-queue capacity.
    /// The actor separately interrupts its receipt book and cancels providers.
    pub(super) fn close(&mut self, terminal: Closed) -> Result<(), ReadRefusal> {
        match self.phase {
            Phase::Open => {
                self.phase = Phase::Ending(terminal);
                self.queue.clear();
                self.audio = 0;
                self.controls = 0;
                Ok(())
            }
            Phase::Ending(_) | Phase::Consumed => Err(ReadRefusal::Gone),
        }
    }
    /// A conflicting wait cannot steal the first waiter's pending update.
    pub(super) fn read(
        &mut self,
        waiter: oneshot::Sender<VoiceUpdate>,
    ) -> Result<Delivery, ReadRefusal> {
        if self.phase == Phase::Consumed {
            return Err(ReadRefusal::Gone);
        }
        if self
            .waiter
            .as_ref()
            .is_some_and(|current| !current.is_closed())
        {
            return Err(ReadRefusal::Waiting);
        }
        if waiter.is_closed() {
            return Ok(Delivery::Waiting);
        }
        self.waiter = Some(waiter);
        Ok(self.deliver())
    }
    pub(super) fn deliver(&mut self) -> Delivery {
        let Some(waiter) = self.waiter.take() else {
            return Delivery::Waiting;
        };
        if let Some(update) = self.queue.pop_front() {
            let kind = Kind::of(&update);
            match waiter.send(update) {
                Ok(()) => {
                    match kind {
                        Some(Kind::Audio) => self.audio = self.audio.saturating_sub(1),
                        Some(Kind::Control) => self.controls = self.controls.saturating_sub(1),
                        None => {}
                    }
                    return Delivery::UpdateReady;
                }
                Err(update) => {
                    self.queue.push_front(update);
                    return Delivery::Waiting;
                }
            }
        }
        match self.phase {
            Phase::Ending(terminal) => {
                if waiter.send(terminal.update()).is_ok() {
                    self.phase = Phase::Consumed;
                    Delivery::TerminalReady
                } else {
                    Delivery::Waiting
                }
            }
            Phase::Open => {
                self.waiter = Some(waiter);
                Delivery::Waiting
            }
            Phase::Consumed => Delivery::Waiting,
        }
    }
}

#[cfg(test)]
mod tests;
