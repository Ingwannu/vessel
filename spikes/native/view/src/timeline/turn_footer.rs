//! A footer belongs to one response, including history with no visible prompt.
//! It must not copy an earlier response or invent a duration across an idle gap.

use chrono::{DateTime, Utc};

use super::{Entry, EntryKind};

const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i64 = 60;

/// Completion time and response content share the same turn boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFooter {
    /// Always available for a real completion, even in partial history.
    pub completed_at: DateTime<Utc>,
    /// Absent when the loaded response has no visible start.
    pub started_at: Option<DateTime<Utc>>,
    /// All assistant messages in this response, separated as in Paseo's copy button.
    pub response: String,
}

impl TurnFooter {
    /// Only a turn-end entry can own a completed footer.
    #[must_use]
    pub fn of(entries: &[Entry], index: usize) -> Option<Self> {
        let end = entries.get(index)?;
        if !matches!(end.kind, EntryKind::TurnEnded { .. }) {
            return None;
        }
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
        let response = before
            .get(start..)?
            .iter()
            .filter_map(|entry| {
                if let EntryKind::Message { text } = &entry.kind {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        Some(Self {
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
