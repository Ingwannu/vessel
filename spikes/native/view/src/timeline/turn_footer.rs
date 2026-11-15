//! A footer belongs to one response, including history with no visible prompt.
//! It must not copy an earlier response or invent a duration across an idle gap.

use chrono::{DateTime, Utc};

use super::{Entry, EntryKind, repeats_failure};
use vessel_proto::StopReason;

const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i64 = 60;

/// Completion stays distinct from its provider failure message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnCompletion {
    /// A normal protocol stop retains its exact reason.
    Ended(StopReason),
    /// The turn failed; the existing failure row says why.
    Failed,
}

impl TurnCompletion {
    fn of(kind: &EntryKind) -> Option<Self> {
        match kind {
            EntryKind::TurnEnded { reason, .. } => Some(Self::Ended(*reason)),
            EntryKind::TurnFailed { .. } => Some(Self::Failed),
            EntryKind::Ready { .. }
            | EntryKind::Prompt(_)
            | EntryKind::Steer { .. }
            | EntryKind::Question { .. }
            | EntryKind::Message { .. }
            | EntryKind::Thought { .. }
            | EntryKind::Tool(_)
            | EntryKind::Plan { .. }
            | EntryKind::Permission(_)
            | EntryKind::Log { .. }
            | EntryKind::Status { .. }
            | EntryKind::Parking(_)
            | EntryKind::Forked { .. }
            | EntryKind::Profile { .. }
            | EntryKind::Notification(_)
            | EntryKind::Compaction(_)
            | EntryKind::Plugin(_)
            | EntryKind::Model(_) => None,
        }
    }
}

/// Completion time and response content share the same turn boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFooter {
    /// Failure cannot be mislabeled as a successful end.
    pub completion: TurnCompletion,
    /// Always available for a real completion, even in partial history.
    pub completed_at: DateTime<Utc>,
    /// Absent when the loaded response has no visible start.
    pub started_at: Option<DateTime<Utc>>,
    /// All assistant messages in this response, separated as in Paseo's copy button.
    pub response: String,
}

impl TurnFooter {
    /// A real completion owns its footer; failed turns need a response/tool host.
    #[must_use]
    pub fn of(entries: &[Entry], index: usize) -> Option<Self> {
        let end = entries.get(index)?;
        let completion = TurnCompletion::of(&end.kind)?;
        let before = entries.get(..index)?;
        let boundary = before.iter().rposition(|entry| {
            matches!(
                entry.kind,
                EntryKind::Prompt(_) | EntryKind::TurnEnded { .. } | EntryKind::TurnFailed { .. }
            )
        });
        let started_at = boundary
            .and_then(|at| before.get(at))
            .filter(|entry| matches!(entry.kind, EntryKind::Prompt(_)))
            .map(|entry| entry.at);
        let start = boundary.map_or(0, |at| at.saturating_add(1));
        let response_entries = before.get(start..)?;
        if completion == TurnCompletion::Failed
            && !response_entries.iter().enumerate().any(|(offset, entry)| {
                matches!(entry.kind, EntryKind::Tool(_))
                    || (matches!(entry.kind, EntryKind::Message { .. })
                        && !repeats_failure(entries, start.saturating_add(offset)))
            })
        {
            return None;
        }
        let response = response_entries
            .iter()
            .enumerate()
            .filter(|(offset, _)| !repeats_failure(entries, start.saturating_add(*offset)))
            .filter_map(|(_, entry)| {
                if let EntryKind::Message { text } = &entry.kind {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        Some(Self {
            completion,
            completed_at: end.at,
            started_at,
            response,
        })
    }
}

/// Seconds, minutes and hours use Paseo's duration thresholds, clamped before zero.
#[must_use]
pub fn duration_label(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let seconds = end.signed_duration_since(start).num_seconds().max(0);
    if seconds < SECONDS_PER_MINUTE {
        return format!("{seconds}s");
    }
    let minutes = seconds.div_euclid(SECONDS_PER_MINUTE);
    if minutes < MINUTES_PER_HOUR {
        let rest = seconds.rem_euclid(SECONDS_PER_MINUTE);
        return if rest == 0 {
            format!("{minutes}m")
        } else {
            format!("{minutes}m {rest}s")
        };
    }
    let hours = minutes.div_euclid(MINUTES_PER_HOUR);
    let rest = minutes.rem_euclid(MINUTES_PER_HOUR);
    if rest == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {rest}m")
    }
}

#[cfg(test)]
mod tests;
