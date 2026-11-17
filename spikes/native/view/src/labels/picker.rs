//! Per-name pending assignment state; a successful toggle never closes the picker.

use std::collections::BTreeMap;

use super::values::Key;
use super::{Assignment, Catalog, Failure, HostState, Label, Name, Ticket};

/// One host catalog row joined with this workspace's assigned names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    /// Host-owned definition.
    pub label: Label,
    /// Current committed assignment.
    pub assignment: Assignment,
    /// A write is in flight for this normalized name.
    pub pending: Option<Ticket>,
}

/// Pending names are independent: one double tap cannot send opposite writes.
#[derive(Debug, Default)]
pub struct Picker {
    pending: BTreeMap<Key, Ticket>,
    error: Option<Failure>,
}

impl Picker {
    /// Draw server state until a fresh workspace descriptor arrives.
    #[must_use]
    pub fn rows(&self, catalog: &Catalog, assigned: &[Name], query: &str) -> Vec<PickerRow> {
        let query = super::search::Query::new(query);
        catalog
            .labels()
            .filter(|label| query.matches(&label.name))
            .map(|label| PickerRow {
                label: label.clone(),
                assignment: if assigned.iter().any(|name| name.key() == label.name.key()) {
                    Assignment::Assigned
                } else {
                    Assignment::Unassigned
                },
                pending: self.pending.get(&label.name.key()).copied(),
            })
            .collect()
    }

    /// Reserve one name before dispatch. Assignment is never changed optimistically.
    /// # Errors
    /// Refuses offline/unsupported hosts and duplicate pending toggles.
    pub fn begin(&mut self, host: HostState, name: &Name, ticket: Ticket) -> Result<(), Failure> {
        host.writable()?;
        let key = name.key();
        if self.pending.contains_key(&key) {
            return Err(Failure::Pending);
        }
        self.error = None;
        let _previous_ticket = self.pending.insert(key, ticket);
        Ok(())
    }

    /// Ignore stale replies from an old surface or an earlier toggle.
    #[must_use]
    pub fn finish(&mut self, name: &Name, ticket: Ticket, result: Result<(), Failure>) -> bool {
        let key = name.key();
        if self.pending.get(&key) != Some(&ticket) {
            return false;
        }
        let _settled_ticket = self.pending.remove(&key);
        if let Err(failure) = result {
            self.error = Some(failure);
        }
        true
    }

    /// Last failed assignment, kept next to its picker.
    #[must_use]
    pub const fn error(&self) -> Option<Failure> {
        self.error
    }
}
