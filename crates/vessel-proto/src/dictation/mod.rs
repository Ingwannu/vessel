//! Composer dictation (ADR 0067).
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

pub use error::DictationDecodeError;

/// First protocol version with these calls. The merge changes this number.
pub const SINCE: u32 = 20;

/// Longest PCM16 chunk. One second at 48 kHz is 96 000 bytes, so this also
/// holds a second at every accepted rate.
const MAX_CHUNK_BYTES: usize = 256 * 1024;

/// Last sequence a stream accepts. 1800 one-second chunks is 30 minutes.
pub const MAX_SEQ: ChunkSeq = ChunkSeq::new(1_799);

/// Longest transcript the node will return.
const MAX_TRANSCRIPT_BYTES: usize = 64 * 1024;

/// A dictation stream. One to 64 characters of `[A-Za-z0-9_-]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct DictationId(String);

impl DictationId {
    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DictationId {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let ok = (1..=64).contains(&text.len())
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

/// Samples per second. 8 000 to 48 000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Validate before any stream or provider is opened.
    ///
    /// # Errors
    /// Rates outside 8–48 kHz cannot be used for dictation.
    pub fn new(rate: u32) -> Result<Self, DictationDecodeError> {
        if (8_000..=48_000).contains(&rate) {
            Ok(Self(rate))
        } else {
            Err(DictationDecodeError::RateRange)
        }
    }

    /// The rate as a number of samples per second.
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
    /// The sample rate.
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

/// One PCM16 chunk. Even, at least one sample, at most 256 KiB.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct DictationPcm(Bytes);

impl DictationPcm {
    /// PCM16 that is even, at least one sample, and at most 256 KiB.
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

    /// The samples.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// The samples, for the node to keep in the stream.
    #[must_use]
    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

fn accepted(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes.len() <= MAX_CHUNK_BYTES && bytes.len().is_multiple_of(2)
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

    /// The index.
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
        Self(if ms > 300_000 { 300_000 } else { ms })
    }

    /// The milliseconds.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A transcript. Empty is allowed. Longer than 64 KiB of UTF-8 is refused.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct Transcript(String);

impl Transcript {
    /// The text.
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
        if ms <= 300_000 {
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
