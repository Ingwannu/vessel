//! Validation errors name rules and limits, never rejected input.
use super::{
    MAX_DIAGNOSTIC_AGENT_PROTOCOL, MAX_DIAGNOSTIC_AGENTS, MAX_DIAGNOSTIC_COUNT,
    MAX_DIAGNOSTIC_FAILURES, MAX_DIAGNOSTIC_REPORT_BYTES, MAX_DIAGNOSTIC_SCHEMA,
    MAX_DIAGNOSTIC_VERSION_BYTES, MAX_DIAGNOSTIC_VERSION_COMPONENT, MIN_CUSTOM_AGENT_SLOT,
    MIN_DIAGNOSTIC_VERSION_DIGITS, VERSION_COMPONENTS,
};
use std::fmt;

/// A diagnostic value failed an allowlist or bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidDiagnostic {
    /// Process identity cannot be zero.
    ProcessId,
    /// A saturated count is not at its ceiling.
    Count,
    /// Outside the agent protocol representation.
    AgentProtocol,
    /// Not a supported nonnegative `SQLite` schema number.
    Schema,
    /// Outside the report-local custom slot range.
    CustomSlot,
    /// Not a bounded normalized version.
    Version,
    /// Too many agent records.
    Agents,
    /// Repeated agent identity in one report.
    DuplicateAgents,
    /// Too many retained failures.
    Failures,
    /// Cross-field counts disagree.
    CountsDisagree,
    /// Unknown/malformed report field, with rejected input redacted.
    Report,
    /// Canonical JSON exceeded its bound.
    TooLarge,
}
impl fmt::Display for InvalidDiagnostic {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessId => write!(
                output,
                "diagnostic process id must be at least {}",
                super::MIN_DIAGNOSTIC_PROCESS_ID
            ),
            Self::Count => write!(
                output,
                "diagnostic counts are unsigned and a saturated count must equal {MAX_DIAGNOSTIC_COUNT}"
            ),
            Self::AgentProtocol => write!(
                output,
                "diagnostic agent protocol must be unsigned and at most {MAX_DIAGNOSTIC_AGENT_PROTOCOL}"
            ),
            Self::Schema => write!(
                output,
                "diagnostic schema must be nonnegative and at most {MAX_DIAGNOSTIC_SCHEMA}"
            ),
            Self::CustomSlot => write!(
                output,
                "custom diagnostic slot must be {MIN_CUSTOM_AGENT_SLOT} through {MAX_DIAGNOSTIC_AGENTS}"
            ),
            Self::Version => write!(
                output,
                "diagnostic version must fit {MAX_DIAGNOSTIC_VERSION_BYTES} bytes with {VERSION_COMPONENTS} numeric components, each at least {MIN_DIAGNOSTIC_VERSION_DIGITS} digit and at most {MAX_DIAGNOSTIC_VERSION_COMPONENT}; suffix words are not retained"
            ),
            Self::Agents | Self::DuplicateAgents => write!(
                output,
                "diagnostic agents must have distinct identities and at most {MAX_DIAGNOSTIC_AGENTS} rows"
            ),
            Self::Failures => write!(
                output,
                "diagnostic failures must have at most {MAX_DIAGNOSTIC_FAILURES} rows"
            ),
            Self::CountsDisagree => output.write_str("diagnostic session counts disagree"),
            Self::Report => write!(
                output,
                "invalid diagnostic report; maximum {MAX_DIAGNOSTIC_REPORT_BYTES} bytes, {MAX_DIAGNOSTIC_AGENTS} agents and {MAX_DIAGNOSTIC_FAILURES} failures; rejected content is redacted"
            ),
            Self::TooLarge => write!(
                output,
                "diagnostic report exceeds {MAX_DIAGNOSTIC_REPORT_BYTES} bytes"
            ),
        }
    }
}
impl std::error::Error for InvalidDiagnostic {}
