//! One bounded private bootstrap transcript, with no I/O or ownership authority.
//!
//! The Windows launcher creates entropy only after opening its dedicated pipe.
//! The exact child echoes it through inherited stdout, and authentication then
//! uses the same retained pipe. These values alone prove neither fact.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::NodeId;

/// The launcher fills these bytes from the OS RNG after opening the pipe.
pub const CHALLENGE_BYTES: usize = 32;
const ENCODED_BYTES: usize = 43;
const CHALLENGE_PREFIX: &str = "vessel-local-challenge/1 ";
const PIPE_PREFIX: &str = "vessel-local-pipe/1 ";
const READY_PREFIX: &str = "vessel-local-ready/2 ";
const NODE_BYTES: usize = 36;
/// Fixed size lets the child read exactly one challenge without buffering auth.
pub const CHALLENGE_RECORD_BYTES: usize = CHALLENGE_PREFIX.len() + ENCODED_BYTES + 1;
/// Child stdin carries one launch-private name seed, followed by EOF.
pub const PIPE_RECORD_BYTES: usize = PIPE_PREFIX.len() + ENCODED_BYTES + 1;
/// Includes the exact final newline; readers cap allocation before decoding.
pub const MAX_BOOTSTRAP_READY_BYTES: usize =
    READY_PREFIX.len() + NODE_BYTES + 1 + ENCODED_BYTES + 1;

/// A public, single-use correlation value. Never compare it with ordinary `==`.
#[derive(Clone, Copy)]
pub struct BootstrapChallenge([u8; CHALLENGE_BYTES]);

/// A fresh pipe-name seed, unrelated to the later connection's challenge.
///
/// Naming is not peer proof. This type is neither persisted nor serialized into
/// an RPC or argv; only the managed child's dedicated stdin receives its record.
#[derive(Clone, Copy)]
pub struct BootstrapPipe([u8; CHALLENGE_BYTES]);

impl std::fmt::Debug for BootstrapPipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BootstrapPipe(<redacted>)")
    }
}

impl BootstrapPipe {
    /// The caller fills a new seed from the OS RNG for every child launch.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; CHALLENGE_BYTES]) -> Self {
        Self(bytes)
    }

    /// Used only to bind/open the one private Windows server instance.
    #[must_use]
    pub fn path(self) -> std::path::PathBuf {
        format!(
            r"\\.\pipe\vessel-desktop-{}",
            URL_SAFE_NO_PAD.encode(self.0)
        )
        .into()
    }

    /// The inherited child stdin record; never log it or put it in argv/env.
    #[must_use]
    pub fn encode(self) -> String {
        format!("{PIPE_PREFIX}{}\n", URL_SAFE_NO_PAD.encode(self.0))
    }

    /// Decode the complete bounded stdin record, including its final newline.
    ///
    /// # Errors
    /// Unknown markers, aliases, extra bytes and malformed name seeds fail.
    pub fn decode(record: &str) -> Result<Self, TranscriptError> {
        let raw = body(record, PIPE_PREFIX, PIPE_RECORD_BYTES)?;
        Ok(Self(BootstrapChallenge::encoded(raw)?.0))
    }
}

impl std::fmt::Debug for BootstrapChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BootstrapChallenge(<redacted>)")
    }
}

/// The inherited stdout echo from the retained child, not pipe-supplied proof.
#[derive(Debug, Clone, Copy)]
pub struct BootstrapReady {
    node: NodeId,
    challenge: BootstrapChallenge,
}

/// No error retains a pipe name, echoed payload or credential.
#[derive(Debug, thiserror::Error)]
pub enum TranscriptError {
    /// Records must have the exact canonical width, marker and single newline.
    #[error("invalid private local bootstrap framing")]
    Framing,
    /// No aliases, padding, whitespace or alternative base64 alphabet.
    #[error("noncanonical private local bootstrap challenge")]
    Challenge,
    /// The bounded base64 decoder rejected the challenge's representation.
    #[error("invalid private local bootstrap challenge encoding")]
    Decode(#[source] base64::DecodeSliceError),
    /// The public identity could not be decoded.
    #[error("invalid private local bootstrap node identity")]
    Identifier(#[source] uuid::Error),
    /// UUID aliases or whitespace cannot identify the expected record.
    #[error("noncanonical private local bootstrap node identity")]
    Noncanonical,
}

impl BootstrapChallenge {
    /// The caller must obtain fresh OS entropy after opening this launch's pipe.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; CHALLENGE_BYTES]) -> Self {
        Self(bytes)
    }

    /// For the launcher's constant-time comparison of its own echoed challenge.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; CHALLENGE_BYTES] {
        &self.0
    }

    /// One exact pre-auth record, never accepted on the canonical endpoint.
    #[must_use]
    pub fn encode(self) -> String {
        format!("{CHALLENGE_PREFIX}{}\n", URL_SAFE_NO_PAD.encode(self.0))
    }

    /// Decode exactly one bounded record without skipping banner/log lines.
    ///
    /// # Errors
    /// Unknown markers, wrong lengths and noncanonical challenge text fail.
    pub fn decode(record: &str) -> Result<Self, TranscriptError> {
        let raw = body(record, CHALLENGE_PREFIX, CHALLENGE_RECORD_BYTES)?;
        Self::encoded(raw)
    }

    fn encoded(raw: &str) -> Result<Self, TranscriptError> {
        if raw.len() != ENCODED_BYTES {
            return Err(TranscriptError::Challenge);
        }
        let mut bytes = [0; CHALLENGE_BYTES];
        let size = URL_SAFE_NO_PAD
            .decode_slice(raw, &mut bytes)
            .map_err(TranscriptError::Decode)?;
        if size != CHALLENGE_BYTES || URL_SAFE_NO_PAD.encode(bytes) != raw {
            return Err(TranscriptError::Challenge);
        }
        Ok(Self(bytes))
    }
}

impl BootstrapReady {
    /// Create only after reading the challenge on the exact retained pipe.
    #[must_use]
    pub const fn new(node: NodeId, challenge: BootstrapChallenge) -> Self {
        Self { node, challenge }
    }

    /// Compared with the private persisted identity and authenticated greeting.
    #[must_use]
    pub const fn node(self) -> NodeId {
        self.node
    }

    /// Merely an echo; the launcher still verifies original child and transport.
    #[must_use]
    pub const fn challenge(&self) -> &BootstrapChallenge {
        &self.challenge
    }

    /// One inherited stdout record. Never write this to a log or canonical pipe.
    #[must_use]
    pub fn encode(self) -> String {
        format!(
            "{READY_PREFIX}{} {}\n",
            self.node,
            URL_SAFE_NO_PAD.encode(self.challenge.0)
        )
    }

    /// Decode only the exact version and width; extra lines never get ignored.
    ///
    /// # Errors
    /// Wrong framing, noncanonical IDs or invalid challenges fail closed.
    pub fn decode(record: &str) -> Result<Self, TranscriptError> {
        let raw = body(record, READY_PREFIX, MAX_BOOTSTRAP_READY_BYTES)?;
        let (node, challenge) = raw.split_once(' ').ok_or(TranscriptError::Framing)?;
        let parsed: NodeId = node.parse().map_err(TranscriptError::Identifier)?;
        if node.len() != NODE_BYTES || parsed.to_string() != node {
            return Err(TranscriptError::Noncanonical);
        }
        Ok(Self::new(parsed, BootstrapChallenge::encoded(challenge)?))
    }
}

fn body<'a>(record: &'a str, prefix: &str, size: usize) -> Result<&'a str, TranscriptError> {
    if record.len() != size {
        return Err(TranscriptError::Framing);
    }
    record
        .strip_prefix(prefix)
        .and_then(|raw| raw.strip_suffix('\n'))
        .ok_or(TranscriptError::Framing)
}

#[cfg(test)]
mod tests;
