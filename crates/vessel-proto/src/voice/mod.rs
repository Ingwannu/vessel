//! Transient spoken-conversation contract (ADR 0080), without I/O or providers.
//!
//! Methods have pending rows. The integrator assigns [`SINCE`] only with
//! node wiring; no peer may infer support from these definitions.

mod audio;
mod messages;
mod settings;
mod text;

pub use crate::id::{PlaybackId, SpeechGroupId, VoiceEpoch, VoiceId, VoiceSeq};
pub use audio::{CapturePcm, PlaybackPcm};
pub use messages::{
    AbortVoice, AgentTurn, CaptureAck, CaptureVoice, Confidence, InputSpeech, NextVoice, Playback,
    PlaybackOutcome, PlayedVoice, ReadyVoice, SpeakVoice, StartVoice, StopVoice, StopVoiceResult,
    VoiceFailure, VoiceLease, VoiceStopTarget, VoiceTranscript, VoiceUpdate,
};
pub use settings::{
    LocalTtsModelId, SetVoice, SetVoiceStt, SetVoiceTts, TtsModel, TtsVoice, UnknownTtsModelId,
    VoiceSettings, VoiceSettingsError, VoiceSttSettings, VoiceTtsSettings,
};
pub use text::SpeechText;

/// One version gate for the whole package; unassigned calls are never sent or dispatched.
pub const SINCE: Option<crate::ProtocolVersion> = None;

/// Capture is normalized in the app before crossing the node boundary.
pub const CAPTURE_RATE_HZ: u32 = 16_000;
/// Normal microphone upload cadence; batching never extends beyond the chunk cap.
pub const CAPTURE_CHUNK_MS: u32 = 20;
/// At most 100 ms of mono PCM16; large chunks would delay interruption.
pub const MAX_CAPTURE_BYTES: usize = 3_200;
/// A provider segment cannot grow the playback queue without a bound.
pub const MAX_PLAYBACK_BYTES: usize = 8 * 1024 * 1024;
/// Speech is bounded before segmentation or provider work.
pub const MAX_SPEECH_BYTES: usize = 64 * 1024;

/// Decoder errors reveal only the violated rule, never speech, audio or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VoiceDecodeError {
    /// PCM16 consists of complete, nonempty samples inside its concrete cap.
    #[error("voice PCM must be nonempty, even-length and within its byte limit")]
    Pcm,
    /// Reject before a synthesis job or retained speech allocation is made.
    #[error("speech must be nonblank and within its byte limit")]
    Speech,
}

#[cfg(test)]
mod tests;
