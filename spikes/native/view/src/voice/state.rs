//! Generation-fenced voice transitions; the link owns every side effect.

use vessel_proto::{NodeId, SessionId};

use super::VoiceEvent;

/// The exact session and node whose voice lease may receive microphone audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// Identity is a node id, never a mutable book label.
    pub node: NodeId,
    /// Voice uses this existing conversation rather than creating another agent.
    pub session: SessionId,
}

/// Locally unique operation identity; late device/node callbacks cannot retarget it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

/// Why a start must be refused before any node or device work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The connected node advertises ready voice and the agent is idle.
    Ready,
    /// The host's opt-in voice switch is off.
    Disabled,
    /// The negotiated peer has no voice methods.
    Unsupported,
    /// A required model or provider is unavailable.
    NotReady,
    /// A disconnected host must not leave a microphone running.
    Offline,
    /// Paseo requires the running turn to be interrupted first.
    AgentRunning,
    /// Dictation or another audio consumer owns capture.
    CaptureBusy,
}

/// Mute suppresses upload while retaining the capture meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mute {
    /// Audio may be uploaded while the transport is ready.
    Open,
    /// Captured audio is never uploaded.
    Muted,
}

/// Paseo's seven visible runtime phases, without invalid boolean combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No device or node voice lease is active.
    Disabled,
    /// Node enable and device start have not both completed.
    Starting,
    /// Capture is ready for the next spoken utterance.
    Listening,
    /// The node is finalizing speech before an agent turn.
    Submitting,
    /// The agent is working without audible speech playing.
    Waiting,
    /// Assistant speech is actually playing, not merely queued.
    Playing,
    /// Cleanup must settle before another target may start.
    Stopping,
}

/// Overlay controls are derived from ownership; switching cannot attach actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// No overlay is drawn.
    Absent,
    /// Both controls disabled; Stop shows the shared spinner, mute keeps its icon.
    Switching(Mute),
    /// Both controls may act for this same active generation.
    Active(Mute),
}

/// Node enable precedes capture; a device cannot start on speculative readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartStep {
    /// Wait for the exact node lease to be accepted.
    Node,
    /// The accepted node lease can now be paired with device capture.
    Capture,
}

/// A stale completion must be acknowledged by the caller without mutating current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// The completion belongs to the current generation.
    Current,
    /// The link must retire any resource this older operation acquired.
    Stale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Disabled,
    Starting {
        target: Target,
        generation: Generation,
        step: StartStep,
    },
    Live {
        target: Target,
        generation: Generation,
        phase: LivePhase,
        turn: Turn,
        mute: Mute,
    },
    Stopping {
        target: Target,
        generation: Generation,
        mute: Mute,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LivePhase {
    Listening,
    Submitting,
    Waiting,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Idle,
    Running,
}

/// The single app conversation; no device handle, audio bytes or credentials live here.
#[derive(Debug, Default)]
pub struct State {
    next: u64,
    mode: Mode,
}

/// Invalid transitions are explicit; callers cannot mistake refusal for a started microphone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    /// Starting is validated before any capture or node effect.
    #[error("voice is unavailable: {0:?}")]
    Unavailable(Availability),
    /// Switching targets must first complete cleanup.
    #[error("voice already holds a lease")]
    Busy,
    /// Exhaustion refuses rather than reusing an earlier operation identity.
    #[error("voice operation identities exhausted")]
    Exhausted,
    /// A matching generation arrived at an impossible step.
    #[error("voice completion arrived out of order")]
    Order,
    /// Controls cannot invent a live session while disabled or switching.
    #[error("voice is not active")]
    Inactive,
}

impl State {
    /// The presentation phase is derived from ownership rather than separately mutable flags.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        match self.mode {
            Mode::Disabled => Phase::Disabled,
            Mode::Starting { .. } => Phase::Starting,
            Mode::Stopping { .. } => Phase::Stopping,
            Mode::Live { phase, .. } => match phase {
                LivePhase::Listening => Phase::Listening,
                LivePhase::Submitting => Phase::Submitting,
                LivePhase::Waiting => Phase::Waiting,
                LivePhase::Playing => Phase::Playing,
            },
        }
    }

    /// Preserve the previous mute icon through uncertain cleanup; never infer
    /// actionable controls from a route or an independently mutable boolean.
    #[must_use]
    pub const fn overlay(&self) -> Overlay {
        match self.mode {
            Mode::Disabled => Overlay::Absent,
            Mode::Starting { .. } => Overlay::Switching(Mute::Open),
            Mode::Stopping { mute, .. } => Overlay::Switching(mute),
            Mode::Live { mute, .. } => Overlay::Active(mute),
        }
    }

    /// Route changes retain the identity whose cleanup is still owed.
    #[must_use]
    pub const fn target(&self) -> Option<Target> {
        match self.mode {
            Mode::Disabled => None,
            Mode::Starting { target, .. }
            | Mode::Live { target, .. }
            | Mode::Stopping { target, .. } => Some(target),
        }
    }

    /// Refuse an unavailable target before allocating a generation or changing ownership.
    ///
    /// # Errors
    /// Unavailable, an existing lease, or exhausted generation space.
    pub fn start(
        &mut self,
        target: Target,
        readiness: Availability,
    ) -> Result<Generation, TransitionError> {
        if readiness != Availability::Ready {
            return Err(TransitionError::Unavailable(readiness));
        }
        if self.mode != Mode::Disabled {
            return Err(TransitionError::Busy);
        }
        let generation = self.advance()?;
        self.mode = Mode::Starting {
            target,
            generation,
            step: StartStep::Node,
        };
        Ok(generation)
    }

    fn advance(&mut self) -> Result<Generation, TransitionError> {
        // Refuse wraparound: an old callback must never acquire a new identity.
        let next = self.next.checked_add(1).ok_or(TransitionError::Exhausted)?;
        self.next = next;
        Ok(Generation(next))
    }

    /// A late node success returns Stale so its newly acquired remote lease is retired.
    ///
    /// # Errors
    /// A matching operation cannot complete node enable twice.
    pub fn node_ready(&mut self, received: Generation) -> Result<Applied, TransitionError> {
        if self.generation() != Some(received) {
            return Ok(Applied::Stale);
        }
        match self.mode {
            Mode::Starting {
                target,
                generation,
                step: StartStep::Node,
            } if generation == received => {
                self.mode = Mode::Starting {
                    target,
                    generation,
                    step: StartStep::Capture,
                };
                Ok(Applied::Current)
            }
            Mode::Stopping { .. } => Ok(Applied::Stale),
            Mode::Starting { .. } | Mode::Live { .. } | Mode::Disabled => {
                Err(TransitionError::Order)
            }
        }
    }

    /// Capture becomes live only after the node acknowledged this same generation.
    ///
    /// # Errors
    /// Starting capture before node enable or completing it twice is invalid.
    pub fn capture_ready(&mut self, received: Generation) -> Result<Applied, TransitionError> {
        if self.generation() != Some(received) {
            return Ok(Applied::Stale);
        }
        match self.mode {
            Mode::Starting {
                target,
                generation,
                step: StartStep::Capture,
            } if generation == received => {
                self.mode = Mode::Live {
                    target,
                    generation,
                    phase: LivePhase::Listening,
                    turn: Turn::Idle,
                    mute: Mute::Open,
                };
                Ok(Applied::Current)
            }
            Mode::Stopping { .. } => Ok(Applied::Stale),
            Mode::Starting { .. } | Mode::Live { .. } | Mode::Disabled => {
                Err(TransitionError::Order)
            }
        }
    }

    pub(super) const fn generation(&self) -> Option<Generation> {
        match self.mode {
            Mode::Disabled => None,
            Mode::Starting { generation, .. }
            | Mode::Live { generation, .. }
            | Mode::Stopping { generation, .. } => Some(generation),
        }
    }

    /// Upload permission is stricter than capture existence: starting/stopping never uploads.
    #[must_use]
    pub fn can_upload(&self, received: Generation) -> bool {
        matches!(self.mode, Mode::Live { generation, mute: Mute::Open, .. } if generation == received)
    }

    /// Muting never changes which node owns capture or makes a disabled session live.
    ///
    /// # Errors
    /// Disabled and switching controls refuse input.
    pub fn toggle_mute(&mut self) -> Result<Mute, TransitionError> {
        let Mode::Live { mute, .. } = &mut self.mode else {
            return Err(TransitionError::Inactive);
        };
        *mute = match *mute {
            Mute::Open => Mute::Muted,
            Mute::Muted => Mute::Open,
        };
        Ok(*mute)
    }

    /// Node events settle the live presentation without crossing target generations.
    ///
    /// # Errors
    /// A current-generation event while starting/stopping cannot invent a live phase.
    pub fn observe(
        &mut self,
        received: Generation,
        event: VoiceEvent,
    ) -> Result<Applied, TransitionError> {
        if self.generation() != Some(received) {
            return Ok(Applied::Stale);
        }
        let Mode::Live { phase, turn, .. } = &mut self.mode else {
            return Err(TransitionError::Inactive);
        };
        if event == VoiceEvent::PlaybackFinished && *phase != LivePhase::Playing {
            return Ok(Applied::Stale);
        }
        *phase = match event {
            VoiceEvent::SpeechStarted | VoiceEvent::TranscriptEmpty => {
                *turn = Turn::Idle;
                LivePhase::Listening
            }
            VoiceEvent::SpeechStopped => LivePhase::Submitting,
            VoiceEvent::TranscriptReady | VoiceEvent::TurnStarted => {
                *turn = Turn::Running;
                match phase {
                    LivePhase::Playing => LivePhase::Playing,
                    LivePhase::Listening | LivePhase::Submitting | LivePhase::Waiting => {
                        LivePhase::Waiting
                    }
                }
            }
            VoiceEvent::TurnEnded => {
                *turn = Turn::Idle;
                match phase {
                    LivePhase::Playing => LivePhase::Playing,
                    LivePhase::Submitting => LivePhase::Submitting,
                    LivePhase::Listening | LivePhase::Waiting => LivePhase::Listening,
                }
            }
            VoiceEvent::PlaybackStarted => LivePhase::Playing,
            VoiceEvent::PlaybackFinished => match turn {
                Turn::Running => LivePhase::Waiting,
                Turn::Idle => LivePhase::Listening,
            },
        };
        Ok(Applied::Current)
    }

    /// Cleanup retains ownership until the node and device have both settled.
    ///
    /// # Errors
    /// A disabled conversation has nothing to stop; repeated stop is Busy.
    pub fn stop(&mut self) -> Result<Generation, TransitionError> {
        let (target, generation, mute) = match self.mode {
            Mode::Starting {
                target, generation, ..
            } => (target, generation, Mute::Open),
            Mode::Live {
                target,
                generation,
                mute,
                ..
            } => (target, generation, mute),
            Mode::Stopping { .. } => return Err(TransitionError::Busy),
            Mode::Disabled => return Err(TransitionError::Inactive),
        };
        self.mode = Mode::Stopping {
            target,
            generation,
            mute,
        };
        Ok(generation)
    }

    /// Only confirmed cleanup releases ownership; an uncertain stop stays Stopping.
    ///
    /// # Errors
    /// Matching cleanup before stop was requested is an invalid completion.
    pub fn stopped(&mut self, received: Generation) -> Result<Applied, TransitionError> {
        if self.generation() != Some(received) {
            return Ok(Applied::Stale);
        }
        match self.mode {
            Mode::Stopping { generation, .. } if generation == received => {
                self.mode = Mode::Disabled;
                Ok(Applied::Current)
            }
            Mode::Starting { .. } | Mode::Live { .. } | Mode::Stopping { .. } | Mode::Disabled => {
                Err(TransitionError::Order)
            }
        }
    }
}
