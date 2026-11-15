//! An agent's numeric protocol is a different domain from the node wire.
use super::InvalidDiagnostic;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Existing agent readiness reports this unsigned representation's range.
pub const MAX_DIAGNOSTIC_AGENT_PROTOCOL: u16 = u16::MAX;

/// An ACP protocol observation, never a Vessel `ProtocolVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct AcpProtocolVersion(u32);
impl AcpProtocolVersion {
    /// Its numeric value, when formatting a compatibility reason.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl From<u16> for AcpProtocolVersion {
    fn from(value: u16) -> Self {
        Self(u32::from(value))
    }
}
impl TryFrom<u32> for AcpProtocolVersion {
    type Error = InvalidDiagnostic;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > u32::from(MAX_DIAGNOSTIC_AGENT_PROTOCOL) {
            return Err(InvalidDiagnostic::AgentProtocol);
        }
        Ok(Self(value))
    }
}
impl<'de> Deserialize<'de> for AcpProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        // A serde error can quote hostile input; keep only the named rule.
        let raw = u32::deserialize(input)
            .map_err(|_untrusted| D::Error::custom(InvalidDiagnostic::AgentProtocol))?;
        Self::try_from(raw).map_err(D::Error::custom)
    }
}

impl fmt::Display for AcpProtocolVersion {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, output)
    }
}
