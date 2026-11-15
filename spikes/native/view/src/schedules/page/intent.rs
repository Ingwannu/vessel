//! A UI activation carries the row it actually offered, not a later alias lookup.
//!
//! [Decision Log]
//! - Purpose: delayed menu/confirmation callbacks must not target a replacement.
//! - Existing seam: the canonical `ConnectionContext` and node-authored revision.
//! - Choice: capture both with the composite row key, validate before Page::act.
//! - Alternative: look up only the key at activation and silently use new facts.
//! - Why/cost: reconnect and edits retire old intent without adding a registry;
//!   a person must reopen the action after that change.
use super::{Page, Request, Row, RowKey};
use crate::connection::ConnectionContext;
use crate::schedules::Action;

/// Immutable identity of the row and physical connection offered to a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    key: RowKey,
    connection: ConnectionContext,
    revision: u32,
}
impl Intent {
    /// Composite row identity, without exposing mutable identity fields.
    #[must_use]
    pub const fn key(&self) -> &RowKey {
        &self.key
    }
    /// True only for the original connection, id and node-authored revision.
    #[must_use]
    pub fn is_current(&self, page: &Page) -> bool {
        page.row(&self.key).is_some_and(|row| {
            row.connection == Some(&self.connection) && row.schedule.revision == self.revision
        })
    }
    /// Recheck current authority and operation state after checking identity.
    #[must_use]
    pub fn allows(&self, page: &Page, action: Action) -> bool {
        self.is_current(page)
            && page
                .row(&self.key)
                .is_some_and(|row| row.actions().contains(&action))
    }
}
impl Row<'_> {
    /// Capture only an admitted row. Offline cached facts offer no activation.
    #[must_use]
    pub fn intent(&self) -> Option<Intent> {
        Some(Intent {
            key: self.key.clone(),
            connection: self.connection?.clone(),
            revision: self.schedule.revision,
        })
    }
}
impl Page {
    /// Preserve the activation's original physical target through queueing.
    pub fn act_intent(&mut self, intent: &Intent, action: Action) -> Option<Request> {
        if !intent.allows(self, action) {
            return None;
        }
        self.act(intent.key(), action)
    }
}
#[cfg(test)]
mod tests;
