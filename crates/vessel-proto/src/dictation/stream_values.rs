//! Dictation request and update values, without registration or node behaviour.
//!
//! Transcript and audio fields keep their validated, redacted types so nesting
//! them in a request or update cannot expose their contents through Debug.

use super::{ChunkSeq, DictationId, DictationPcm, LocalModelId, Pcm16Mono, Transcript, WithinMs};
use serde::{Deserialize, Serialize};

/// `dictation.start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StartDictation {
    /// The stream id.
    pub id: DictationId,
    /// PCM16 mono and its rate.
    pub format: Pcm16Mono,
}

/// `dictation.chunk`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChunkDictation {
    /// The stream.
    pub id: DictationId,
    /// The chunk's index, starting at 0.
    pub seq: ChunkSeq,
    /// The samples.
    pub pcm: DictationPcm,
}

/// How far the node has accepted a contiguous run from sequence 0.
/// Omitted until sequence 0 itself has been accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DictationAck {
    /// The highest contiguous sequence, when sequence 0 is in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub through: Option<ChunkSeq>,
}

/// `dictation.finish`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FinishDictation {
    /// The stream.
    pub id: DictationId,
    /// The last sequence the client sent, including a shorter tail.
    pub final_seq: ChunkSeq,
}

/// The first admitted finish freezes this budget and its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FinishAccepted {
    /// The wait, in milliseconds.
    pub within_ms: WithinMs,
}

/// `dictation.cancel`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CancelDictation {
    /// The stream.
    pub id: DictationId,
}

/// `dictation.next`. One update per call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NextDictation {
    /// The stream.
    pub id: DictationId,
}

/// One observation from `dictation.next`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DictationUpdate {
    /// Text so far. A later partial replaces it.
    Partial {
        /// The joined transcript.
        text: Transcript,
    },
    /// The stream is done.
    Final {
        /// The joined transcript. Empty when the audio was silence.
        text: Transcript,
    },
    /// The stream ended without a Final.
    Failed {
        /// Why.
        reason: DictationReason,
    },
    /// A newer node introduced an update kind this client does not understand.
    #[serde(other)]
    Unknown,
}

/// Why a dictation call or a `Failed` update stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DictationReason {
    /// The bytes are not the one-second shape, or the tail is not the one finish names.
    BadAudio,
    /// The sequence or total audio passed the stream duration bound.
    TooLong,
    /// Held audio passed the reorder count or byte bound.
    ReorderOverflow,
    /// The transcript would pass its byte bound.
    TranscriptTooLong,
    /// `finish` waited out its budget.
    TimedOut,
    /// This connection opened the same id again.
    Replaced,
    /// The endpoint failed. A new id can retry.
    Upstream,
    /// The stream was cancelled by the caller, connection close or node stop.
    Cancelled,
    /// `finish` before any accepted audio.
    NoAudio,
    /// The switch is off.
    Disabled,
    /// The endpoint has no usable key.
    KeyAbsent,
    /// This build has no local engine and must not silently select an endpoint.
    LocalNotBuilt {
        /// The selected catalogue id is actionable without exposing a credential.
        model: LocalModelId,
    },
    /// This connection or the node already has its streams.
    TooMany,
    /// A newer node wrote a reason this one does not know.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
#[path = "stream_values_tests.rs"]
mod tests;
