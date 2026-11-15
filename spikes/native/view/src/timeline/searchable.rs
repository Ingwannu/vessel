//! Closed item kinds select searchable prose independently of finder scheduling.
//! This keeps vocabulary growth out of the bounded incremental-search controller.

use super::{Entry, EntryKind};

/// The text a find looks in, for the kinds it looks in.
#[must_use]
pub fn searchable(entry: &Entry) -> Option<&str> {
    match &entry.kind {
        EntryKind::Prompt(prompt) => Some(super::presented_prompt(&prompt.text)),
        EntryKind::Steer { text, .. } => Some(super::presented_prompt(text)),
        EntryKind::Message { text } => Some(text),
        EntryKind::Question { question, .. } => Some(&question.message),
        EntryKind::Ready { .. }
        | EntryKind::Thought { .. }
        | EntryKind::Tool(_)
        | EntryKind::Plan { .. }
        | EntryKind::Permission(_)
        | EntryKind::TurnEnded { .. }
        | EntryKind::TurnFailed { .. }
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
