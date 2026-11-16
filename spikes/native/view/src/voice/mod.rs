//! Spoken-conversation presentation and ownership, without devices or I/O.
//!
//! ADR 0080 keeps a single app capture owner and fences late completions:
//! changing a route must never send old audio to a different session.

mod cue;
mod meter;
pub mod settings;
mod state;

pub use cue::{Cue, CueAction, CueError, CuePlayback, THINKING_CUE_DURATION};

pub use meter::{DisplayMeter, InvalidLevel, Level, SpeechActivity, Telemetry};

pub use state::{
    Applied, Availability, Generation, Mute, Overlay, Phase, StartStep, State, Target,
    TransitionError,
};

#[cfg(test)]
mod tests;

/// Inputs carry meaning rather than assigning an arbitrary visible phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEvent {
    /// Confirmed speech cancels current playback and the agent in the link.
    SpeechStarted,
    /// Server-side VAD sealed an utterance for transcription.
    SpeechStopped,
    /// Silence or low confidence must not create an agent turn.
    TranscriptEmpty,
    /// Accepted spoken text is entering the ordinary agent send path.
    TranscriptReady,
    /// The selected session started work.
    TurnStarted,
    /// Completion, cancellation and failure all end the wait for that turn.
    TurnEnded,
    /// Physical playback began, rather than just being queued.
    PlaybackStarted,
    /// Physical playback finished; link accounting decides whether more is pending.
    PlaybackFinished,
}
