//! Safe numeric process metadata; no arbitrary pid, command or environment.
use super::{DiagnosticCount, DiagnosticSection, InvalidDiagnostic};
use serde::{Deserialize, Serialize};

/// Smallest operating-system process identity.
pub const MIN_DIAGNOSTIC_PROCESS_ID: u32 = 1;

/// Nonzero process identity, distinct from node and session identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "number"))]
pub struct DiagnosticProcessId(u32);
impl TryFrom<u32> for DiagnosticProcessId {
    type Error = InvalidDiagnostic;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value < MIN_DIAGNOSTIC_PROCESS_ID {
            return Err(InvalidDiagnostic::ProcessId);
        }
        Ok(Self(value))
    }
}
impl From<DiagnosticProcessId> for u32 {
    fn from(value: DiagnosticProcessId) -> Self {
        value.0
    }
}
impl DiagnosticProcessId {
    /// Checked process identity; zero is never an absence sentinel.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Memory from one observation. Counts expose JSON-safe saturation explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiagnosticMemory {
    /// Resident memory in bytes.
    pub resident_bytes: DiagnosticCount,
    /// Virtual address-space size in bytes.
    pub virtual_bytes: DiagnosticCount,
}

/// Only this daemon's process; no remote process-list or signalling handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiagnosticProcess {
    /// Process identity at observation time.
    pub pid: DiagnosticProcessId,
    /// Parent, absent when the operating system supplies no parent.
    pub parent: Option<DiagnosticProcessId>,
    /// Missing memory is explicit; no zero substitution.
    pub memory: DiagnosticSection<DiagnosticMemory>,
}
