//! ADR 0074's immutable, allowlisted report consumer. No RPC is registered here.
//! Technical data uses English section names and canonical JSON for nested
//! typed rows, keeping saturation/omission and unknown reasons explicit.
use super::collection::{MAX_REPORT_BYTES, Problem, ReportText};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{fmt::Write as _, sync::Arc};
use vessel_proto::{
    NodeId,
    diagnostics::{
        DiagnosticHome, DiagnosticNode, DiagnosticProcess, DiagnosticReport,
        DiagnosticReportFields, DiagnosticSection,
    },
};

/// Missing producers remain separate from a successfully collected typed report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonSnapshot {
    /// No report, with an explicit closed failure/availability reason.
    Unavailable(Problem),
    /// Validated at the protocol's immutable report root.
    Report(Arc<DiagnosticReport>),
}
impl DaemonSnapshot {
    /// Identity must agree with the authenticated connection before attribution.
    #[must_use]
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Self::Unavailable(_) => None,
            Self::Report(report) => Some(report.fields().node.id),
        }
    }
    /// Collection success does not imply all independent sources succeeded.
    #[must_use]
    pub fn complete(&self) -> bool {
        match self {
            Self::Unavailable(_) => false,
            Self::Report(report) => {
                let DiagnosticReportFields {
                    collected_at: _,
                    node: _,
                    process,
                    store,
                    agents,
                    sessions,
                    listeners,
                    relays,
                    recent_failures,
                } = report.fields();
                process_complete(process)
                    && available(store)
                    && available(agents)
                    && available(sessions)
                    && available(listeners)
                    && available(relays)
                    && available(recent_failures)
            }
        }
    }
    pub(super) fn render(&self) -> Result<ReportText, Problem> {
        let report = match self {
            Self::Unavailable(problem) => {
                return ReportText::new(&format!("Daemon diagnostic\n  Status: {problem}"));
            }
            Self::Report(report) => report.fields(),
        };
        // Exhaustive field destructuring makes a new safe source an integration obligation.
        let DiagnosticReportFields {
            collected_at,
            node,
            process,
            store,
            agents,
            sessions,
            listeners,
            relays,
            recent_failures,
        } = report;
        let mut text = node_section(*collected_at, node);
        source(
            &mut text,
            "Daemon self process (numeric metadata only)",
            process,
        )?;
        source(&mut text, "Store (readability, not integrity)", store)?;
        source(&mut text, "Agents (cached observations)", agents)?;
        source(&mut text, "Runtime sessions", sessions)?;
        source(&mut text, "Listeners (no reachability claim)", listeners)?;
        source(&mut text, "Relays", relays)?;
        source(
            &mut text,
            "Failed local node calls (not a daemon log)",
            recent_failures,
        )?;
        ReportText::new(&text)
    }
}
fn node_section(collected_at: DateTime<Utc>, node: &DiagnosticNode) -> String {
    let home = match node.home {
        DiagnosticHome::Redacted => "~",
    };
    format!(
        "Daemon process\n  Collected at: {collected_at}\n  Node ID: {}\n  Version: {}\n  OS: {:?}\n  Architecture: {:?}\n  Lifecycle: {:?}\n  Monotonic uptime ms: {}\n  Home: {home}",
        node.id,
        node.version
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string),
        node.os,
        node.arch,
        node.state,
        node.uptime.get()
    )
}

fn process_complete(source: &DiagnosticSection<DiagnosticProcess>) -> bool {
    match source {
        DiagnosticSection::Available { value, .. } => available(&value.memory),
        DiagnosticSection::Unavailable { .. } => false,
    }
}
fn available<T>(source: &DiagnosticSection<T>) -> bool {
    match source {
        DiagnosticSection::Available { .. } => true,
        DiagnosticSection::Unavailable { .. } => false,
    }
}
fn source<T: Serialize>(
    text: &mut String,
    title: &str,
    source: &DiagnosticSection<T>,
) -> Result<(), Problem> {
    let _written = write!(text, "\n\n{title}");
    match source {
        DiagnosticSection::Available { at, value } => value_line(text, *at, value)?,
        DiagnosticSection::Unavailable { reason } => {
            let _written = write!(text, "\n  Unavailable: {reason:?}");
        }
    }
    if text.len() > MAX_REPORT_BYTES {
        return Err(Problem::TooLarge);
    }
    Ok(())
}
fn value_line<T: Serialize>(
    text: &mut String,
    at: DateTime<Utc>,
    value: &T,
) -> Result<(), Problem> {
    // Only the private calls above can serialize values, all from the validated safe root.
    let value = serde_json::to_string(value).map_err(|_untrusted| Problem::InvalidReply)?;
    let _written = write!(text, "\n  Observed at: {at}\n  Data: {value}");
    Ok(())
}

#[cfg(test)]
mod tests;
