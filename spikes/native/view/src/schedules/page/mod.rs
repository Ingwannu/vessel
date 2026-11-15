//! Fleet schedule list state, shared by renderers. Requests are bounded to four
//! and correlated; a node's reply, never an optimistic local state, wins.
//!
//! [Decision Log]
//! - Purpose: Paseo's aggregate list, filters, per-row operations and retry.
//! - Constraint: one slow/offline machine must not erase another's schedules;
//!   a list begun before a mutation must not undo its result when it arrives.
//! - Choice: one cache per book key, monotonic request tickets, preserved rows
//!   during refresh, and row-local authority evidence reset on a new connection. A separate ledger
//!   holds every unresolved ticket, including obsolete reads, until its reply
//!   arrives; refresh/reconnect cannot bypass the shared request bound.
//! - Cost: refreshes happen only while the screen is visited. New chats and
//!   history and editor actions retain their own typed read/form owners.

mod choices;
mod connection;
mod intent;
pub use intent::Intent;
#[cfg(test)]
mod connection_tests;
#[cfg(test)]
mod grants;
mod mutation;
pub use choices::MachineChoice;
#[cfg(test)]
mod races;
mod read;
#[cfg(test)]
mod reference_tests;
mod rows;
#[cfg(test)]
pub(super) mod tests;

use super::load::{Failure, MAX_IN_FLIGHT, MachineKey, Ticket, Tickets};
use super::{Bucket, authority::Authority};
use chrono::{DateTime, Utc};
pub use mutation::{Mutation, WriteValue};
use read::Read;
pub use read::Snapshot;
pub use rows::{EntryState, Feedback, MenuEntry, Row};
use std::collections::BTreeMap;
use vessel_proto::ScheduleId;

/// Machine/schedule identity of one row and its menu.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowKey {
    /// Book key, not the node's potentially nonunique display name.
    pub machine: MachineKey,
    /// The node's schedule id.
    pub schedule: ScheduleId,
}

/// Host filter, after Paseo's all-hosts selector.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum MachineFilter {
    /// All book machines.
    #[default]
    All,
    /// One named machine.
    One(MachineKey),
}

/// What one bounded node call asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// List schedules.
    List,
    /// Pause automatic runs.
    Pause(ScheduleId),
    /// Resume automatic runs.
    Resume(ScheduleId),
    /// Run once now.
    Run(ScheduleId),
    /// Delete the definition and history; sessions remain.
    Delete(ScheduleId),
}

/// Correlated request handed to link without reinterpreting its fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    machine: MachineKey,
    connection: crate::connection::ConnectionContext,
    /// Local request identity.
    pub ticket: Ticket,
    /// Typed operation.
    pub ask: Ask,
}

impl Request {
    /// Bind an operation once to the original authenticated physical admission.
    /// The machine key is derived here and cannot disagree with that context.
    #[must_use]
    pub fn on(connection: crate::connection::ConnectionContext, ticket: Ticket, ask: Ask) -> Self {
        Self {
            machine: MachineKey::admitted(&connection),
            connection,
            ticket,
            ask,
        }
    }
    /// Original book key; never a later lookup by the node's display name.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }
    /// The actor must select/fence this exact context, even after a queued delay.
    #[must_use]
    pub const fn connection(&self) -> &crate::connection::ConnectionContext {
        &self.connection
    }
}

/// Body state, after the reference's schedules-screen-state test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// Initial requests are pending.
    Loading,
    /// No selected machine could load.
    Failed,
    /// Loaded, with no rows in this filter.
    Empty,
    /// Rows remain visible even if another machine failed.
    Content,
}

#[derive(Debug, Default)]
struct RowState {
    authority: Authority,
    mutation: Mutation,
}

#[derive(Debug, Default)]
struct Cache {
    connection: Option<crate::connection::ConnectionContext>,
    origin: Option<crate::connection::ConnectionContext>,
    read: Read,
    rows: BTreeMap<ScheduleId, RowState>,
    scopes: Option<Vec<vessel_proto::Scope>>,
}

#[derive(Debug, Default)]
enum Book {
    #[default]
    Unread,
    Ready,
}

/// The screen's pure model. The app only forwards requests and draws its values.
#[derive(Debug)]
pub struct Page {
    caches: BTreeMap<MachineKey, Cache>,
    tickets: Tickets,
    in_flight: BTreeMap<Ticket, Request>,
    book: Book,
    /// Current host filter.
    pub machine: MachineFilter,
    /// Current status filter.
    pub bucket: Bucket,
    /// A local resource failure (such as identity exhaustion).
    pub local_failure: Option<Failure>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            caches: BTreeMap::new(),
            tickets: Tickets::default(),
            in_flight: BTreeMap::new(),
            book: Book::Unread,
            machine: MachineFilter::All,
            bucket: Bucket::Active,
            local_failure: None,
        }
    }
}

impl Page {
    /// Keep the book's keys; removed machines cannot keep stale rows or filters.
    pub fn machines(&mut self, machines: &[MachineKey]) {
        self.book = Book::Ready;
        self.caches.retain(|key, _| machines.contains(key));
        for key in machines {
            let _cache = self.caches.entry(key.clone()).or_default();
        }
        if let MachineFilter::One(key) = &self.machine
            && !self.caches.contains_key(key)
        {
            self.machine = MachineFilter::All;
        }
    }

    fn pending(&self) -> usize {
        self.in_flight.len()
    }

    fn track(&mut self, request: Request) -> Request {
        let _old = self.in_flight.insert(request.ticket, request.clone());
        request
    }

    fn retire(&mut self, request: &Request) -> bool {
        if self.in_flight.get(&request.ticket) != Some(request) {
            return false;
        }
        let _retired = self.in_flight.remove(&request.ticket);
        true
    }

    fn issue(&mut self) -> Option<Ticket> {
        let ticket = self.tickets.issue();
        if ticket.is_none() {
            self.local_failure = Some(Failure::Exhausted);
        }
        ticket
    }

    /// At most four initial/due reads. Other hosts begin when a reply frees room.
    #[must_use]
    pub fn due(&mut self, now: DateTime<Utc>) -> Vec<Request> {
        let wanted: Vec<_> = self
            .caches
            .iter()
            .filter(|(_, cache)| cache.connection.is_some() && cache.read.due(now))
            .map(|(key, _)| key.clone())
            .take(MAX_IN_FLIGHT.saturating_sub(self.pending()))
            .collect();
        wanted
            .into_iter()
            .filter_map(|machine| self.read(&machine))
            .collect()
    }

    fn read(&mut self, machine: &MachineKey) -> Option<Request> {
        let context = self.caches.get(machine)?.connection.clone()?;
        let ticket = self.issue()?;
        self.caches.get_mut(machine)?.read.begin(ticket);
        Some(self.track(Request::on(context, ticket, Ask::List)))
    }

    /// Explicit retry/refresh, preserving node-authored rows in the meantime.
    pub fn refresh(&mut self) {
        for cache in self.caches.values_mut() {
            cache.read.invalidate();
        }
    }

    /// Apply only the expected read and preserve row-local permission evidence.
    pub fn listed(&mut self, request: &Request, answer: Result<Snapshot, Failure>) {
        let _accepted = self.accept_list(request, answer);
    }

    /// True means this exact successful snapshot was accepted, so its auxiliary
    /// viewer calendar may update the comparable presentation cache.
    pub fn accept_list(&mut self, request: &Request, answer: Result<Snapshot, Failure>) -> bool {
        if request.ask != Ask::List || !self.retire(request) {
            return false;
        }
        let Some(cache) = self.caches.get_mut(&request.machine) else {
            return false;
        };
        if cache.connection.as_ref() != Some(request.connection()) {
            return false;
        }
        let answer = answer.and_then(|snapshot| {
            let mut identities = std::collections::BTreeSet::new();
            if snapshot
                .schedules
                .iter()
                .all(|info| identities.insert(info.id))
            {
                Ok(snapshot)
            } else {
                Err(Failure::Unexpected)
            }
        });
        let succeeded = answer.is_ok();
        if !cache.read.settle(request.ticket, answer) {
            return false;
        }
        if succeeded {
            cache.origin = Some(request.connection().clone());
        }
        if let Some(snapshot) = cache.read.snapshot() {
            if succeeded && let Ok(scopes) = &snapshot.scopes {
                cache.scopes = Some(scopes.clone());
            }
            cache
                .rows
                .retain(|id, _| snapshot.schedules.iter().any(|info| info.id == *id));
            for info in &snapshot.schedules {
                let state = cache.rows.entry(info.id).or_default();
                state.authority.reported(cache.scopes.as_deref());
                if succeeded {
                    state.mutation.reconciled();
                }
            }
        }
        succeeded
    }

    fn selected(&self, key: &MachineKey) -> bool {
        match &self.machine {
            MachineFilter::All => true,
            MachineFilter::One(chosen) => chosen == key,
        }
    }

    /// Retry only failed reads, never any mutation or already settled host.
    pub fn retry_failed(&mut self) {
        for cache in self.caches.values_mut() {
            if cache.read.problem().is_some_and(Failure::retryable) {
                cache.read.invalidate();
            }
        }
    }
    /// A partial failure's action retries that host only and cannot duplicate an
    /// in-flight read. False means the UI must not attach a retry action.
    pub fn retry_host(&mut self, machine: &MachineKey) -> bool {
        let Some(cache) = self.caches.get_mut(machine) else {
            return false;
        };
        if !cache.read.problem().is_some_and(Failure::retryable) {
            return false;
        }
        cache.read.invalidate();
        true
    }
    /// Total failures normally have one reference message. Specific corrective
    /// reasons instead use the shared Alerts, once per affected host.
    #[must_use]
    pub fn blocking_explanations(&self) -> Vec<(&MachineKey, &Failure)> {
        let problems = self.problems();
        if problems
            .iter()
            .any(|(_, failure)| failure.needs_explanation())
        {
            problems
        } else {
            Vec::new()
        }
    }
    /// No retry is drawn for exhausted request ids or unsupported methods.
    #[must_use]
    pub fn retry_available(&self) -> bool {
        self.problems()
            .iter()
            .any(|(_, failure)| failure.retryable())
    }

    /// Partial failures are banners; only a total initial failure replaces rows.
    #[must_use]
    pub fn body(&self) -> Body {
        if !self.visible_rows().is_empty() {
            return Body::Content;
        }
        if self.local_failure.is_some() {
            return Body::Failed;
        }
        let selected: Vec<_> = self
            .caches
            .iter()
            .filter(|(key, _)| self.selected(key))
            .map(|(_, cache)| cache)
            .collect();
        // The aggregate reference stays connecting after an empty answer
        // while another known host settles; nonempty data may already render.
        let loaded = selected
            .iter()
            .filter_map(|cache| cache.read.snapshot())
            .collect::<Vec<_>>();
        if loaded.iter().any(|snapshot| !snapshot.schedules.is_empty()) {
            return Body::Empty;
        }
        let failed = selected.iter().any(|cache| cache.read.problem().is_some());
        if loaded.is_empty() && failed {
            return Body::Failed;
        }
        if selected
            .iter()
            .any(|cache| cache.read.snapshot().is_none() && cache.read.problem().is_none())
        {
            return Body::Loading;
        }
        if !loaded.is_empty() {
            return Body::Empty;
        }
        match self.book {
            Book::Unread => Body::Loading,
            Book::Ready => Body::Empty,
        }
    }

    /// A single-machine fleet needs neither a host filter nor repeated host labels.
    #[must_use]
    pub fn multiple_machines(&self) -> bool {
        self.caches.len() > 1
    }

    /// Keep filters after selecting an empty subset, but do not offer them
    /// before any node has returned data (the reference's initial empty state).
    #[must_use]
    pub fn filters_visible(&self) -> bool {
        self.caches
            .values()
            .filter_map(|cache| cache.read.snapshot())
            .any(|snapshot| !snapshot.schedules.is_empty())
    }

    /// Every selected host error, including a failed refresh with cached rows.
    #[must_use]
    pub fn problems(&self) -> Vec<(&MachineKey, &Failure)> {
        self.caches
            .iter()
            .filter(|(key, _)| self.selected(key))
            .filter_map(|(key, cache)| cache.read.problem().map(|failure| (key, failure)))
            .collect()
    }

    /// The machine filter's choices, in stable book-key order.
    pub fn keys(&self) -> impl Iterator<Item = &MachineKey> {
        self.caches.keys()
    }
}
