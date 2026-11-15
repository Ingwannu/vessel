//! Pending call values; the single method table owns Request impls and admission stays unassigned.

use serde::{Deserialize, Serialize};

use super::{
    CapturePcm, PlaybackId, PlaybackPcm, SpeechGroupId, SpeechText, VoiceEpoch, VoiceId, VoiceSeq,
};
use crate::SessionId;
use crate::dictation::Pcm16Mono;

/// Every subsequent operation names both client id and node-minted epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VoiceLease {
    /// Client identity allows cancellation before start completes.
    pub id: VoiceId,
    /// New starts cannot inherit the previous lease's callbacks or receipts.
    pub epoch: VoiceEpoch,
}

/// A person enables an idle existing session, never an agent-selected target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct StartVoice {
    /// Unique per attempt, never a cross-connection bearer capability.
    pub id: VoiceId,
    /// The service checks operate authority and session state before work.
    pub session: SessionId,
}

/// Node readiness precedes app microphone capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadyVoice {
    /// Bound to the caller's live connection and this exact session.
    pub lease: VoiceLease,
    /// Identity is repeated so a stale response cannot retarget a UI.
    pub session: SessionId,
}

/// Fixed-format PCM avoids negotiating a new interpretation after a lease opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct CaptureVoice {
    /// The receiving actor authorizes the connection before retaining PCM.
    pub lease: VoiceLease,
    /// Contiguous order; a repeated sequence never transcribes twice.
    pub seq: VoiceSeq,
    /// 16 kHz mono PCM16, never a caller-selected compressed format.
    pub pcm: CapturePcm,
}

/// An acknowledgement exists only after one or more chunks were accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CaptureAck {
    /// Highest contiguous accepted sequence; no -1 sentinel.
    pub through: VoiceSeq,
}

macro_rules! lease_request {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// Only the owner connection may address this generation.
            pub lease: VoiceLease,
        }
    };
}

lease_request! { /// Wait for a transient update without consuming it on cancellation.
NextVoice }
lease_request! { /// Release remote state after immediate local Stop has already gated upload.
StopVoice }
lease_request! { /// Barge-in cancels speech and the associated running agent turn.
AbortVoice }

/// Device receipts distinguish successful playback from discarded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlaybackOutcome {
    /// The hardware queue completed this packet.
    Played,
    /// The device refused or failed during playback.
    Failed,
    /// Stop or barge-in retired this packet before completion.
    Interrupted,
}

/// A client cannot acknowledge merely queued data or another lease's packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PlayedVoice {
    /// Connection and generation are checked before receipt lookup.
    pub lease: VoiceLease,
    /// The node minted this identity with the emitted audio.
    pub playback: PlaybackId,
    /// Failure unblocks synthesis but is never reported as audible success.
    pub outcome: PlaybackOutcome,
}

/// The authenticated agent's own active voice session is inferred, not an argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SpeakVoice {
    /// Only intentional user-facing speech reaches the selected TTS provider.
    pub text: SpeechText,
}

/// Output carries actual sample rate; local models may differ from endpoint defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Playback {
    /// The receipt id is separate from ordering and presentation identity.
    pub id: PlaybackId,
    /// One spoken segment; the initial node emits one PCM packet per segment.
    pub group: SpeechGroupId,
    /// This packet's mono PCM16 rate, validated at decode.
    pub format: Pcm16Mono,
    /// Audio never enters the session event log or diagnostics.
    pub pcm: PlaybackPcm,
}

/// Server-confirmed VAD, rather than the dormant client threshold configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InputSpeech {
    /// Confirmed sound begins a new spoken turn and cancels old speech.
    Speaking,
    /// The ending-silence boundary seals capture for final STT.
    Quiet,
}

/// Confidence is policy, not a naked floating point on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// This model did not supply log-probabilities.
    Unspecified,
    /// Available finite mean meets the configured threshold.
    Accepted,
}

/// Low confidence cannot coexist with retained text or become an agent prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VoiceTranscript {
    /// Silence or empty provider output creates no agent turn.
    Empty,
    /// Low-confidence content is discarded before the wire update is constructed.
    Rejected,
    /// Only nonempty bounded text can reach the ordinary session prompt path.
    Accepted {
        /// This intentional text is redacted in diagnostics.
        text: SpeechText,
        /// The model supplied acceptable confidence or no scores.
        confidence: Confidence,
    },
}

/// Turn completion must not stop PCM that is still physically playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AgentTurn {
    /// Work is pending after the spoken prompt was accepted.
    Started,
    /// Completion, failure or cancellation ends that work.
    Ended,
}

/// Closed failure reasons avoid forwarding provider exceptions or response bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum VoiceFailure {
    /// The host switch is off.
    Disabled,
    /// The chosen inference implementation is not linked.
    LocalNotBuilt,
    /// A selected model is absent.
    ModelsMissing,
    /// Credentials have not been selected or were removed.
    KeyAbsent,
    /// The current session or connection already holds voice ownership.
    Busy,
    /// Gaps are refused rather than silently changing the spoken content.
    Sequence,
    /// A named queue, byte or duration bound was exceeded.
    Capacity,
    /// A provider failed without exposing its diagnostic or requested address.
    Provider,
    /// A finite operation deadline ended.
    TimedOut,
    /// A device interruption released capture rather than silently restarting.
    Device,
    /// A newer node can introduce a reason without breaking a reader.
    #[serde(other)]
    Unknown,
}

/// Remote uncertainty remains visible while the local microphone is already stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StopVoiceResult {
    /// Node generation and its speech hooks were retired.
    Stopped,
    /// No automatic restart is permitted until remote ownership is reconciled.
    Uncertain {
        /// Typed reason never carries audio, keys or foreign error text.
        reason: VoiceFailure,
    },
}

/// Long-poll updates are ephemeral; ordinary session history remains the text authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VoiceUpdate {
    /// Server VAD begins or seals the current utterance.
    Input {
        /// One closed state, not independent speaking/quiet flags.
        state: InputSpeech,
    },
    /// Blank or rejected text does not create an agent turn.
    Transcription {
        /// Rejected and empty branches cannot carry speech text.
        result: VoiceTranscript,
    },
    /// One collected synthesis segment; current reference also buffers each segment.
    Audio {
        /// Physical playback completion must acknowledge this exact packet.
        playback: Playback,
    },
    /// Mirrors the existing selected-session turn stream without persisting audio.
    Turn {
        /// Whether the selected session is working.
        state: AgentTurn,
    },
    /// Terminal failures end this generation and release its provider capacity.
    Failed {
        /// Safe public classification of the failure.
        reason: VoiceFailure,
    },
    /// A closed generation has no further long-poll updates.
    Stopped,
}
