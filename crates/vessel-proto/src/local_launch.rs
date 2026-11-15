//! Dedicated child readiness is separate from human logs and authenticated RPC.
//!
//! The launcher still authenticates the local token and checks `NodeId`/Serving
//! after reading this record. A readiness line never grants process ownership.

use crate::NodeId;

/// Both ends share this single marker and local-record format version.
pub const READY_PREFIX: &str = "vessel-local-ready/1 ";
/// Canonical UUID text is bounded before decoding.
const UUID_TEXT_BYTES: usize = 36;
/// Includes the final newline so the reader bounds allocation before parsing.
pub const MAX_READY_BYTES: usize = READY_PREFIX
    .len()
    .saturating_add(UUID_TEXT_BYTES)
    .saturating_add(1);

/// A serving child reports only its public node identity, never a credential/PID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ready {
    node: NodeId,
}

/// Framing errors do not retain potentially sensitive child output.
#[derive(Debug, thiserror::Error)]
pub enum ReadyError {
    /// The record exceeded its named byte bound.
    #[error("local readiness record exceeds {MAX_READY_BYTES} bytes")]
    TooLong,
    /// Unknown format, missing newline or extra data.
    #[error("invalid local readiness framing or version")]
    Framing,
    /// The public node identity could not be decoded.
    #[error("invalid local readiness node identity")]
    Identifier(#[source] uuid::Error),
    /// UUID aliases or whitespace are not canonical records.
    #[error("noncanonical local readiness node identity")]
    Noncanonical,
}

impl Ready {
    /// Emit only after the node has started successfully.
    #[must_use]
    pub const fn new(node: NodeId) -> Self {
        Self { node }
    }

    /// Compare with the authenticated local greeting.
    #[must_use]
    pub const fn node(self) -> NodeId {
        self.node
    }

    /// Canonical pipe framing, including the final newline.
    #[must_use]
    pub fn encode(self) -> String {
        format!("{READY_PREFIX}{}\n", self.node)
    }

    /// Decode one complete record; banner/log lines are never skipped.
    ///
    /// # Errors
    /// Unknown versions, extra data, noncanonical IDs and oversized input fail.
    pub fn decode(line: &str) -> Result<Self, ReadyError> {
        if line.len() > MAX_READY_BYTES {
            return Err(ReadyError::TooLong);
        }
        let raw = line
            .strip_prefix(READY_PREFIX)
            .and_then(|line| line.strip_suffix('\n'))
            .ok_or(ReadyError::Framing)?;
        if raw.len() != UUID_TEXT_BYTES {
            return Err(ReadyError::Noncanonical);
        }
        let node: NodeId = raw.parse().map_err(ReadyError::Identifier)?;
        if node.to_string() != raw {
            return Err(ReadyError::Noncanonical);
        }
        Ok(Self { node })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_is_bounded_and_round_trips() {
        let ready = Ready::new(NodeId::random());
        let encoded = ready.encode();
        assert_eq!(encoded.len(), MAX_READY_BYTES);
        assert_eq!(Ready::decode(&encoded).unwrap(), ready);
    }

    #[test]
    fn logs_unknown_versions_extra_bytes_and_aliases_are_refused() {
        let encoded = Ready::new(NodeId::random()).encode();
        for bad in [
            encoded.trim_end().to_owned(),
            encoded.replace("/1 ", "/2 "),
            format!("log\n{encoded}"),
            encoded.to_ascii_uppercase(),
            format!("{encoded}extra"),
            format!("{READY_PREFIX}{}\n", "x".repeat(MAX_READY_BYTES)),
        ] {
            assert!(Ready::decode(&bad).is_err());
        }
    }
}
