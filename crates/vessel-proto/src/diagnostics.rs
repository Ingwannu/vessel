//! A bounded, content-free node report and pending RPC (ADR 0074).
//!
//! [Decision Log]
//! Existing node/harness objects contain paths, keys and arbitrary output.
//! This allowlist has only identifiers, closed reasons, normalized numeric
//! versions and bounded collections. Clients render it; the node sends no
//! raw log/text block. No request or method may be added before the shared
//! unassigned-protocol admission accessor reaches the node line.

mod admission;
mod bounded;
pub use admission::{DiagnosticRefusal, MAX_DIAGNOSTIC_COLLECTIONS};
mod error;
mod process;
mod protocol;
pub use process::{
    DiagnosticMemory, DiagnosticProcess, DiagnosticProcessId, MIN_DIAGNOSTIC_PROCESS_ID,
};
mod report;
mod scalar;
mod sections;
mod version;

pub use bounded::{
    DiagnosticAgents, MAX_DIAGNOSTIC_AGENTS, MAX_DIAGNOSTIC_FAILURES, RecentFailures,
};
pub use error::InvalidDiagnostic;
pub use protocol::{AcpProtocolVersion, MAX_DIAGNOSTIC_AGENT_PROTOCOL};
pub use report::{
    DiagnosticFailureWindow, DiagnosticReport, DiagnosticReportFields, MAX_DIAGNOSTIC_REPORT_BYTES,
};
pub use scalar::{
    CustomAgentSlot, DiagnosticCount, DiagnosticSchemaVersion, MAX_DIAGNOSTIC_COUNT,
    MAX_DIAGNOSTIC_SCHEMA, MIN_CUSTOM_AGENT_SLOT,
};
pub use sections::{
    DiagnosticAgent, DiagnosticAgentId, DiagnosticAgentRefusal, DiagnosticArch,
    DiagnosticAvailability, DiagnosticBinding, DiagnosticErrorCoverage, DiagnosticFailure,
    DiagnosticHome, DiagnosticListeners, DiagnosticLocalKind, DiagnosticNode, DiagnosticOs,
    DiagnosticPresence, DiagnosticRelayCounts, DiagnosticRelays, DiagnosticRuntimeSessions,
    DiagnosticSection, DiagnosticSignIn, DiagnosticStore, DiagnosticStoredSessions,
    DiagnosticUnavailable,
};
pub use version::{
    DiagnosticVersion, MAX_DIAGNOSTIC_VERSION_BYTES, MAX_DIAGNOSTIC_VERSION_COMPONENT,
    MIN_DIAGNOSTIC_VERSION_DIGITS, VERSION_COMPONENTS,
};

/// Request a bounded metadata-only report. Never refreshes providers or agents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetDiagnostics {}

/// Protocol 24 (2026-11-15): `node.diagnostics` is served from this version
/// on; the node, typed client and CLI refuse it below 24 through the shared
/// `required_protocol` accessor. The integrator alone assigns this number.
pub const DIAGNOSTICS_PROTOCOL: Option<crate::ProtocolVersion> =
    Some(crate::ProtocolVersion::new(24));

#[cfg(test)]
mod tests;
