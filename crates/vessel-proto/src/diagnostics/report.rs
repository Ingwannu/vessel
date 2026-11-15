//! Validated root boundary: no unbounded text and a canonical byte cap.
use super::{
    DiagnosticAgents, DiagnosticCount, DiagnosticErrorCoverage, DiagnosticListeners,
    DiagnosticNode, DiagnosticProcess, DiagnosticRelays, DiagnosticRuntimeSessions,
    DiagnosticSection, DiagnosticStore, InvalidDiagnostic, RecentFailures,
};
use chrono::{DateTime, Utc};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::io::{self, Write};

/// Maximum canonical JSON bytes of a report, excluding the RPC envelope.
pub const MAX_DIAGNOSTIC_REPORT_BYTES: usize = 65_536;

/// Inputs to the validated report; the public root owns an immutable copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReportFields {
    /// When this collection completed.
    pub collected_at: DateTime<Utc>,
    /// Safe node identity/platform/lifecycle.
    pub node: DiagnosticNode,
    /// Numeric metadata about this daemon process only.
    pub process: DiagnosticSection<DiagnosticProcess>,
    /// Readability/schema and stored session counts, not integrity proof.
    pub store: DiagnosticSection<DiagnosticStore>,
    /// Complete bounded inventory or an explicit reason it is unavailable.
    pub agents: DiagnosticSection<DiagnosticAgents>,
    /// Current runtime counts, separate from durable state.
    pub sessions: DiagnosticSection<DiagnosticRuntimeSessions>,
    /// Bound state without routes.
    pub listeners: DiagnosticSection<DiagnosticListeners>,
    /// Relay watch counts without endpoint data.
    pub relays: DiagnosticSection<DiagnosticRelays>,
    /// Explicitly scoped failed-call observations, not a log tail.
    pub recent_failures: DiagnosticSection<DiagnosticFailureWindow>,
}

/// A bounded failure window with explicit observation/loss coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticFailureWindow {
    /// What is and is not observed.
    pub coverage: DiagnosticErrorCoverage,
    /// A new node instance starts a new window; history is not persisted.
    pub observed_since: DateTime<Utc>,
    /// Retained failures from the collector's rolling time window.
    pub entries: RecentFailures,
    /// Observations lost/evicted since this node instance started.
    pub omitted: DiagnosticCount,
}

/// A complete canonical report that fits its byte cap; no mutable access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct DiagnosticReport(DiagnosticReportFields);
impl DiagnosticReport {
    /// Read its validated fields.
    #[must_use]
    pub const fn fields(&self) -> &DiagnosticReportFields {
        &self.0
    }
}
impl TryFrom<DiagnosticReportFields> for DiagnosticReport {
    type Error = InvalidDiagnostic;
    fn try_from(fields: DiagnosticReportFields) -> Result<Self, Self::Error> {
        validate_counts(&fields)?;
        let mut output = ByteBudget::default();
        if serde_json::to_writer(&mut output, &fields).is_err() {
            return Err(if output.exceeded {
                InvalidDiagnostic::TooLarge
            } else {
                InvalidDiagnostic::Report
            });
        }
        Ok(Self(fields))
    }
}
impl<'de> Deserialize<'de> for DiagnosticReport {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        // Serde's unknown-field/variant errors can include a secret supplied
        // as that name. Retaining the original error as a source would leak
        // through Debug too. This public edge deliberately replaces it.
        let fields = DiagnosticReportFields::deserialize(input)
            .map_err(|_untrusted| D::Error::custom(InvalidDiagnostic::Report))?;
        Self::try_from(fields).map_err(D::Error::custom)
    }
}
fn validate_counts(fields: &DiagnosticReportFields) -> Result<(), InvalidDiagnostic> {
    if let DiagnosticSection::Available { value, .. } = &fields.store
        && value.sessions.live.saturating_add(value.sessions.ended) != value.sessions.total
    {
        return Err(InvalidDiagnostic::CountsDisagree);
    }
    if let DiagnosticSection::Available { value, .. } = &fields.sessions
        && (value.parked.get() > value.live.get()
            || (value.parked.is_saturated() && !value.live.is_saturated()))
    {
        return Err(InvalidDiagnostic::CountsDisagree);
    }
    Ok(())
}

#[derive(Default)]
struct ByteBudget {
    used: usize,
    exceeded: bool,
}
impl Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self.used.checked_add(bytes.len());
        match next {
            Some(next) if next <= MAX_DIAGNOSTIC_REPORT_BYTES => {
                self.used = next;
                Ok(bytes.len())
            }
            Some(_) | None => {
                self.exceeded = true;
                Err(io::Error::other(InvalidDiagnostic::TooLarge))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn exact_cap_and_first_excess_byte() {
        let mut sink = ByteBudget::default();
        sink.write_all(&vec![b'x'; MAX_DIAGNOSTIC_REPORT_BYTES])
            .unwrap();
        let error = sink.write_all(b"x").unwrap_err();
        assert!(sink.exceeded);
        assert!(
            error
                .to_string()
                .contains(&MAX_DIAGNOSTIC_REPORT_BYTES.to_string())
        );
    }
}
