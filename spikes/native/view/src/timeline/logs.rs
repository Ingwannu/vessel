//! An agent's own log in its conversation: the lines its adapter writes
//! to its error output (`AgentEvent::HarnessLog`), folded out of the way.
//!
//! [Decision Log]
//! - Purpose: a real Claude Code session (its ACP adapter) opens with a
//!   dozen lines such as `[session/create] sessionId=… phase=settings
//!   durationMs=40 totalMs=41` before the first prompt; drawn one entry a
//!   line, they filled the conversation's first screen (the owner, build
//!   0.0.0.0.7).
//! - Constraint: the node keeps every such line because login and
//!   configuration problems surface only there (`HarnessLog`'s own
//!   comment), so they cannot simply be dropped.
//! - The references: Paseo and GOAT never show an adapter's log in the
//!   conversation. The page draws every line raw in both of its display
//!   settings (`timeline-item.tsx`, `case "log"`), the same defect.
//! - Choice: a run of consecutive log lines is one row, quiet like a run
//!   of tool calls, saying how many lines it holds and opening to them in
//!   one text box; shown only at the fullest display setting, and left
//!   out entirely while tool calls are collapsed, as Paseo and GOAT leave
//!   it out.
//! - Cost: the runs are found again over the whole log on every batch,
//!   as the tool runs are (linear, and cheap beside the fold).

use super::entry::{Entry, EntryKind};

/// One run of consecutive log lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogRun {
    /// The first line's entry index: where the run's row stands.
    pub lead: usize,
    /// One past the last line's entry index.
    pub end: usize,
}

impl LogRun {
    /// How many lines it holds.
    #[must_use]
    pub const fn lines(&self) -> usize {
        self.end.saturating_sub(self.lead)
    }
}

/// The log runs of a timeline, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Logs {
    runs: Vec<LogRun>,
}

impl Logs {
    /// The runs of log lines in `entries`.
    #[must_use]
    pub fn of(entries: &[Entry]) -> Self {
        let mut runs = Vec::new();
        let mut start: Option<usize> = None;
        for (index, entry) in entries.iter().enumerate() {
            let log = matches!(entry.kind, EntryKind::Log { .. });
            match (log, start) {
                (true, None) => start = Some(index),
                (false, Some(lead)) => {
                    runs.push(LogRun { lead, end: index });
                    start = None;
                }
                (true, Some(_)) | (false, None) => {}
            }
        }
        if let Some(lead) = start {
            runs.push(LogRun {
                lead,
                end: entries.len(),
            });
        }
        Self { runs }
    }

    /// The runs, in order.
    #[must_use]
    pub fn runs(&self) -> &[LogRun] {
        &self.runs
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// The run entry `index` is in, if it is a log line.
    #[must_use]
    pub fn run_of(&self, index: usize) -> Option<&LogRun> {
        let after = self.runs.partition_point(|run| run.lead <= index);
        let run = self.runs.get(after.checked_sub(1)?)?;
        (index < run.end).then_some(run)
    }

    /// Every log line of the session, joined, as the Session actions'
    /// "Agent log" shows and copies them (the lines are not drawn in the
    /// conversation: Paseo's stream has no adapter log).
    #[must_use]
    pub fn all(entries: &[Entry]) -> String {
        entries
            .iter()
            .filter_map(|entry| {
                if let EntryKind::Log { line } = &entry.kind {
                    Some(line.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The lines of `run`, joined, as its box shows and copies them.
    #[must_use]
    pub fn text(run: &LogRun, entries: &[Entry]) -> String {
        entries
            .get(run.lead..run.end)
            .unwrap_or_default()
            .iter()
            .filter_map(|entry| {
                if let EntryKind::Log { line } = &entry.kind {
                    Some(line.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
