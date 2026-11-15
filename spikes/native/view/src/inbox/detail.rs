//! One exact entry's confirmation and action state. Selecting another entry
//! discards its form; late replies cannot touch the replacement selection.

use super::{Entry, Key, Problem, Request};

/// The phase of a selected entry's remote operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Fetch the current live request before drawing answer controls.
    Load,
    /// Set shared read attention, independently of resolution.
    Mark,
    /// Send an answer once, then confirm the authoritative state.
    Answer,
}

/// Detail state is independent of background list hydration, so a refresh
/// never erases a person's partially filled question form.
#[derive(Debug, Default)]
pub struct Detail {
    key: Option<Key>,
    confirmed: bool,
    entry: Option<Entry>,
    pending: Option<(Request, Operation)>,
    problem: Option<Problem>,
}
impl Detail {
    /// Select and confirm one exact attention source.
    pub fn select(&mut self, key: Key, request: Request) {
        if self.key != Some(key) {
            self.entry = None;
        }
        self.confirmed = false;
        self.key = Some(key);
        self.pending = Some((request, Operation::Load));
        self.problem = None;
    }
    /// Start a user action only when this exact entry has been confirmed.
    #[must_use]
    pub fn begin(&mut self, request: Request, operation: Operation) -> Option<Key> {
        if self.pending.is_some() || !self.confirmed || self.entry.is_none() {
            return None;
        }
        let key = self.key?;
        self.pending = Some((request, operation));
        self.problem = None;
        Some(key)
    }
    /// A current response can refresh or withdraw a form. A failure retains
    /// it with a typed explanation; it never implies the answer succeeded.
    #[must_use]
    pub fn receive(
        &mut self,
        key: Key,
        request: Request,
        result: Result<Option<Entry>, Problem>,
    ) -> bool {
        if self.key != Some(key) || !self.pending.is_some_and(|(ticket, _)| ticket == request) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(entry) if entry.as_ref().is_none_or(|entry| entry.key == key) => {
                self.confirmed = true;
                self.entry = entry;
                self.problem = None;
            }
            Ok(_) => self.problem = Some(Problem::InvalidReply),
            Err(problem) => self.problem = Some(problem),
        }
        true
    }
    /// End an old admission without discarding the retained form or draft.
    /// A replacement confirmation gets a fresh ticket before controls reopen.
    pub fn retire(&mut self) {
        self.confirmed = false;
        self.pending = None;
        self.problem = Some(Problem::Offline);
    }
    /// Whether a full form was confirmed on the current selected admission.
    #[must_use]
    pub const fn confirmed(&self) -> bool {
        self.confirmed
    }
    /// Source currently shown, including resolved entries.
    #[must_use]
    pub const fn key(&self) -> Option<Key> {
        self.key
    }
    /// Fresh full form; absent while loading or once resolved.
    #[must_use]
    pub fn entry(&self) -> Option<&Entry> {
        self.entry.as_ref()
    }
    /// User-visible failure of the last confirmation or action.
    #[must_use]
    pub const fn problem(&self) -> Option<Problem> {
        self.problem
    }
    /// Current action, so controls stay disabled until its result is known.
    #[must_use]
    pub fn pending(&self) -> Option<Operation> {
        self.pending.map(|(_, operation)| operation)
    }
}
