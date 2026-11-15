//! Read-only child timeline pages keep the parent's sparse sequence positions.

use super::track::{Target, TrackProblem};
use crate::{
    devices::{Ticket, Tickets},
    timeline::Timeline,
};
use std::collections::BTreeMap;
use vessel_proto::subagents::{
    ChildCursor, ChildHistory, ProviderChildId, ProviderChildPage, bounds,
};
use vessel_proto::{EventRecord, EventSeq};

/// Navigation is scoped to the real managed parent and provider child identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildQuery {
    /// Exact parent whose durable log owns these observations.
    pub target: Target,
    /// Actual provider task/thread id; never a managed session id.
    pub child: ProviderChildId,
}
/// A page read captures direction and generation for stale-response rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRead {
    /// Scoped navigation target.
    pub query: ChildQuery,
    /// Parent cursor; absent requests a bounded retained tail.
    pub cursor: Option<ChildCursor>,
    /// Exact outstanding request.
    pub ticket: Ticket,
}
/// The child pane has no prompt, permission, stop, detach or archive command.
#[derive(Debug)]
pub struct ChildPane {
    query: ChildQuery,
    tickets: Tickets,
    pending: Option<ChildRead>,
    records: BTreeMap<EventSeq, EventRecord>,
    timeline: Timeline,
    next: EventSeq,
    history: ChildHistory,
    failure: Option<TrackProblem>,
}
impl ChildPane {
    /// Opening another child creates another generation rather than reusing rows.
    #[must_use]
    pub fn new(query: ChildQuery) -> Self {
        Self {
            query,
            tickets: Tickets::default(),
            pending: None,
            records: BTreeMap::new(),
            timeline: Timeline::default(),
            next: EventSeq::ZERO,
            history: ChildHistory::Missing,
            failure: None,
        }
    }
    /// Identity/breadcrumb source for a read-only pane.
    #[must_use]
    pub fn query(&self) -> &ChildQuery {
        &self.query
    }
    /// Durable child records, for the shared read-only conversation renderer.
    pub fn records(&self) -> impl Iterator<Item = &EventRecord> {
        self.records.values()
    }
    /// Fleet-wide source identity, independent of book aliases.
    #[must_use]
    pub fn identity(&self) -> vessel_proto::subagents::ProviderChildRef {
        vessel_proto::subagents::ProviderChildRef {
            parent: self.query.target.address,
            child: self.query.child.clone(),
        }
    }
    /// Already projected canonical entries; render using the shared timeline kit.
    #[must_use]
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }
    /// Coverage is separate from an empty or populated pane.
    #[must_use]
    pub const fn history(&self) -> ChildHistory {
        self.history
    }
    /// Read failure keeps the last good page available.
    #[must_use]
    pub const fn failure(&self) -> Option<TrackProblem> {
        self.failure
    }
    /// Whether a specific page request is outstanding.
    #[must_use]
    pub const fn loading(&self) -> bool {
        self.pending.is_some()
    }
    /// Read the retained tail, earlier page, or new parent continuation.
    /// # Errors
    /// Another request is pending or the ticket counter is exhausted.
    pub fn read(&mut self, cursor: Option<ChildCursor>) -> Result<ChildRead, TrackProblem> {
        if self.pending.is_some() {
            return Err(TrackProblem::Busy);
        }
        let ticket = self
            .tickets
            .next_ticket()
            .map_err(|_error| TrackProblem::Exhausted)?;
        let read = ChildRead {
            query: self.query.clone(),
            cursor,
            ticket,
        };
        self.pending = Some(read.clone());
        self.failure = None;
        Ok(read)
    }
    /// The preceding page uses the first actual child record as an exclusive bound.
    #[must_use]
    pub fn before(&self) -> Option<ChildCursor> {
        self.records
            .first_key_value()
            .map(|(seq, _)| ChildCursor::Before(*seq))
    }
    /// Continuation is the parent's actual scanned sequence, including empty pages.
    #[must_use]
    pub const fn after(&self) -> ChildCursor {
        ChildCursor::After(self.next)
    }
    /// Late answers from another child/direction are ignored.
    pub fn received(
        &mut self,
        read: &ChildRead,
        result: Result<ProviderChildPage, TrackProblem>,
    ) -> bool {
        if self.pending.as_ref() != Some(read) {
            return false;
        }
        self.pending = None;
        match result {
            Err(problem) => self.failure = Some(problem),
            Ok(page) => {
                if let Err(problem) = self.merge(read, &page) {
                    self.failure = Some(problem);
                }
            }
        }
        true
    }
    fn merge(&mut self, read: &ChildRead, page: &ProviderChildPage) -> Result<(), TrackProblem> {
        if page.records.as_slice().iter().any(|record| {
            record.seq >= page.next
                || match read.cursor {
                    Some(ChildCursor::Before(before)) => record.seq >= before,
                    Some(ChildCursor::After(after)) => record.seq < after,
                    None => false,
                }
        }) {
            return Err(TrackProblem::InvalidReply);
        }
        let mut proposed = self.records.clone();
        for record in page.records.as_slice() {
            if proposed.get(&record.seq).is_some_and(|old| old != record) {
                return Err(TrackProblem::InvalidReply);
            }
            proposed.insert(record.seq, record.clone());
        }
        if proposed.len() > bounds::MAX_SCAN_RECORDS {
            return Err(TrackProblem::RetentionLimit);
        }
        let mut budget = vessel_proto::subagents::ChildScanBudget::default();
        for record in proposed.values() {
            budget
                .admit(record)
                .map_err(|_error| TrackProblem::RetentionLimit)?;
        }
        let mut timeline = Timeline::default();
        timeline.apply(&proposed.values().cloned().collect::<Vec<_>>());
        self.records = proposed;
        self.timeline = timeline;
        self.next = self.next.max(page.next);
        self.history = page.history;
        self.failure = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
