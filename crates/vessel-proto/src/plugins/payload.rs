//! Opaque helper payloads carry plugin data without ambient diagnostic leaks.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::RawJson;

/// Leaves space for envelope fields inside the bounded helper frame.
pub const MAX_HELPER_PAYLOAD_BYTES: usize = 3 * 1024 * 1024;

/// This private pipe's request/result data is never a node RPC argument by
/// implication. The owning adapter must decode its concrete business type and
/// recheck core authorization before acting on any returned data.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawJson", into = "RawJson")]
pub struct HelperPayload(RawJson);

impl fmt::Debug for HelperPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HelperPayload(<private plugin data>)")
    }
}

/// A helper payload must fit without silently truncating a result or input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("plugin helper payload exceeds {MAX_HELPER_PAYLOAD_BYTES} bytes")]
pub struct PayloadTooLarge;

impl TryFrom<RawJson> for HelperPayload {
    type Error = PayloadTooLarge;
    fn try_from(raw: RawJson) -> Result<Self, Self::Error> {
        if raw.as_text().len() > MAX_HELPER_PAYLOAD_BYTES {
            return Err(PayloadTooLarge);
        }
        Ok(Self(raw))
    }
}

impl From<HelperPayload> for RawJson {
    fn from(payload: HelperPayload) -> Self {
        payload.0
    }
}

impl HelperPayload {
    /// Only the owning typed adapter decodes this; generic supervision never
    /// branches on its fields. Do not copy this text into an error or trace.
    #[must_use]
    pub const fn raw(&self) -> &RawJson {
        &self.0
    }
}
