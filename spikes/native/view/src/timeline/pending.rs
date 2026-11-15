//! Ordered pending permissions share the fold's existing index, without a second projection.
use super::{Entry, EntryKind, Permission, Timeline};
impl Timeline {
    /// All unresolved permissions in record order, for simultaneous request cards.
    pub fn pending_permissions(&self) -> impl Iterator<Item = (&Entry, &Permission)> {
        self.waiting.iter().filter_map(|index| {
            let entry = self.entries.get(*index)?;
            match &entry.kind {
                EntryKind::Permission(permission) => Some((entry, permission)),
                EntryKind::Ready { .. }
                | EntryKind::Prompt(_)
                | EntryKind::Steer { .. }
                | EntryKind::Question { .. }
                | EntryKind::Message { .. }
                | EntryKind::Thought { .. }
                | EntryKind::Tool(_)
                | EntryKind::Plan { .. }
                | EntryKind::TurnEnded { .. }
                | EntryKind::TurnFailed { .. }
                | EntryKind::Log { .. }
                | EntryKind::Status { .. }
                | EntryKind::Parking(_)
                | EntryKind::Forked { .. }
                | EntryKind::Profile { .. }
                | EntryKind::Model(_) => None,
            }
        })
    }
}
