//! Validated dictation wire values (ADR 0067), without stream state, providers or I/O.
//!
//! The merge assigns the protocol number by changing [`SINCE`]. This branch
//! keeps it at the base version so a method's `since` is never newer than
//! [`PROTOCOL_VERSION`](crate::PROTOCOL_VERSION).

use std::fmt;
use std::str::FromStr;

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::key::serde_via_text;

mod error;
mod local_model;
mod patch;
mod secret_ref;
mod settings;
mod settings_values;

pub use local_model::{LocalModelId, UnknownLocalModelId};
pub use patch::DictationChange;
/// Speech settings share omission/null decoding without duplicating its visitor.
pub(crate) use patch::present as deserialize_change;
pub use secret_ref::{RedactedNameError, SecretReference};
pub use settings::{
    DictationProvider, DictationSettings, DictationSwitch, KeyPresence, SetDictation,
};
pub use settings_values::{EndpointBase, EndpointModel, Language};

pub use error::DictationDecodeError;

/// First protocol version with these calls. The merge changes this number.
pub const SINCE: crate::ProtocolVersion = crate::PROTOCOL_VERSION;

// Bounds are shared with diagnostic formatting so a refusal cannot name a
// different limit from the one that rejected the input.
const MIN_ID_CHARS: usize = 1;
const MAX_ID_CHARS: usize = 64;
const MIN_RATE: u32 = 8_000;
const MAX_RATE: u32 = 48_000;
const MAX_FINISH_MS: u32 = 300_000;
const PCM_SAMPLE_BYTES: usize = size_of::<i16>();
const KIB_BYTES: usize = 1024;

/// Longest PCM16 chunk; holds a second at every accepted sample rate.
const MAX_CHUNK_BYTES: usize = 256 * KIB_BYTES;

/// Last sequence a stream accepts; the stream enforces this duration bound.
pub const MAX_SEQ: ChunkSeq = ChunkSeq::new(1_799);

/// Longest transcript the node will return.
const MAX_TRANSCRIPT_BYTES: usize = 64 * KIB_BYTES;

/// A dictation stream token, bounded by `MIN_ID_CHARS` and `MAX_ID_CHARS`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct DictationId(String);

impl DictationId {
    /// Encode the validated token at an output or correlation boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DictationId {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let ok = (MIN_ID_CHARS..=MAX_ID_CHARS).contains(&text.len())
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
        if ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(DictationDecodeError::Id)
        }
    }
}

impl fmt::Display for DictationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(DictationId);

/// Samples per second, bounded by `MIN_RATE` and `MAX_RATE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(u32);

impl SampleRate {
    /// The mono PCM rate required by the shared OpenAI-compatible STT encoder.
    pub const TRANSCRIPTION: Self = Self(24_000);

    /// Let codec fixtures derive the same lower bound as the validator.
    pub const MIN: Self = Self(MIN_RATE);

    /// Let codec fixtures derive the same upper bound as the validator.
    pub const MAX: Self = Self(MAX_RATE);

    /// Validate before any stream or provider is opened.
    ///
    /// # Errors
    /// Rates outside `MIN_RATE` through `MAX_RATE` cannot be used for dictation.
    pub fn new(rate: u32) -> Result<Self, DictationDecodeError> {
        if (MIN_RATE..=MAX_RATE).contains(&rate) {
            Ok(Self(rate))
        } else {
            Err(DictationDecodeError::RateRange)
        }
    }

    /// Encode validated Hertz in provider audio metadata.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Signed 16-bit mono PCM. The wire form is `audio/pcm;rate=N;bits=16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct Pcm16Mono {
    rate: SampleRate,
}

impl Pcm16Mono {
    /// Give resampling code a validated input rate.
    #[must_use]
    pub const fn rate(self) -> SampleRate {
        self.rate
    }
}

impl FromStr for Pcm16Mono {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split(';');
        if parts.next() != Some("audio/pcm") {
            return Err(DictationDecodeError::Format);
        }
        let mut rate = None;
        let mut bits = None;
        let mut channels = None;
        for part in parts {
            let Some((key, value)) = part.split_once('=') else {
                return Err(DictationDecodeError::Format);
            };
            match key {
                "rate" if rate.is_none() => {
                    let parsed = value.parse().map_err(DictationDecodeError::RateSyntax)?;
                    rate = Some(SampleRate::new(parsed)?);
                }
                "bits" if bits.is_none() && value == "16" => bits = Some(()),
                "channels" if channels.is_none() && value == "1" => channels = Some(()),
                _ => return Err(DictationDecodeError::Format),
            }
        }
        let rate = rate.ok_or(DictationDecodeError::Format)?;
        if bits.is_none() {
            return Err(DictationDecodeError::Format);
        }
        Ok(Self { rate })
    }
}

impl fmt::Display for Pcm16Mono {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "audio/pcm;rate={};bits=16", self.rate.get())
    }
}

serde_via_text!(Pcm16Mono);

/// One complete PCM16 chunk, bounded by `PCM_SAMPLE_BYTES` and `MAX_CHUNK_BYTES`.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct DictationPcm(Bytes);

impl DictationPcm {
    /// Complete PCM16 samples up to `MAX_CHUNK_BYTES`.
    ///
    /// # Errors
    /// The bytes are odd, empty, or longer than the cap.
    pub fn new(bytes: impl Into<Bytes>) -> Result<Self, DictationDecodeError> {
        let bytes = bytes.into();
        if accepted(&bytes) {
            Ok(Self(bytes))
        } else {
            Err(DictationDecodeError::Pcm)
        }
    }

    /// Let codecs borrow the samples without duplicating their buffer.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Transfer the validated buffer to audio processing without copying it.
    #[must_use]
    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

fn accepted(bytes: &[u8]) -> bool {
    bytes.len() >= PCM_SAMPLE_BYTES
        && bytes.len() <= MAX_CHUNK_BYTES
        && bytes.len().is_multiple_of(PCM_SAMPLE_BYTES)
}

impl Serialize for DictationPcm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::base64_bytes::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for DictationPcm {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Reject an oversized encoded chunk before allocating its decoded bytes.
        deserializer.deserialize_str(PcmVisitor)
    }
}

/// A chunk's place in the stream. The type accepts any `u32` so a sequence
/// past [`MAX_SEQ`] is still a request the node can refuse as `TooLong`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct ChunkSeq(u32);

impl ChunkSeq {
    /// Any index, including one past [`MAX_SEQ`]. The node refuses those.
    #[must_use]
    pub const fn new(seq: u32) -> Self {
        Self(seq)
    }

    /// Compare or encode a position at the stream boundary.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// How long `finish` will wait, in milliseconds. At most five minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u32", into = "u32")]
pub struct WithinMs(u32);

impl WithinMs {
    /// A budget the node computed, capped at five minutes.
    #[must_use]
    pub const fn new(ms: u32) -> Self {
        Self(if ms > MAX_FINISH_MS {
            MAX_FINISH_MS
        } else {
            ms
        })
    }

    /// Construct a deadline timer from the bounded wire budget.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A transcript, allowing empty text up to `MAX_TRANSCRIPT_BYTES` in UTF-8.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct Transcript(String);

impl Transcript {
    /// Expose text to an explicit output or rendering boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Transcript {
    type Error = DictationDecodeError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.len() > MAX_TRANSCRIPT_BYTES {
            Err(DictationDecodeError::Transcript)
        } else {
            Ok(Self(text))
        }
    }
}

impl fmt::Display for Transcript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Transcript {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Transcript {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::try_from(text).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for DictationPcm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DictationPcm([redacted])")
    }
}

impl fmt::Debug for Transcript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Transcript([redacted])")
    }
}

impl TryFrom<u32> for WithinMs {
    type Error = DictationDecodeError;

    fn try_from(ms: u32) -> Result<Self, Self::Error> {
        if ms <= MAX_FINISH_MS {
            Ok(Self(ms))
        } else {
            Err(DictationDecodeError::Budget)
        }
    }
}

impl From<WithinMs> for u32 {
    fn from(ms: WithinMs) -> Self {
        ms.get()
    }
}

struct PcmVisitor;

impl serde::de::Visitor<'_> for PcmVisitor {
    type Value = DictationPcm;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded base64 PCM16 audio")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
        use base64::Engine as _;
        // Ceiling division includes padding; the cap is on decoded bytes.
        const ENCODED_CAP: usize = MAX_CHUNK_BYTES.div_ceil(3).saturating_mul(4);
        if text.len() > ENCODED_CAP {
            return Err(E::custom(DictationDecodeError::Pcm));
        }
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(|source| E::custom(DictationDecodeError::Encoding(source)))?;
        DictationPcm::new(decoded).map_err(E::custom)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod metadata_tests;
