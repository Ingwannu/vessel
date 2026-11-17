//! Paseo's import sheet (TRACKER 777): every provider of one host, read in
//! parallel and merged; a row press imports that one session.
//!
//! [Decision Log]
//! - Purpose: the coordinator's 777 rulings (2026-11-17): the sheet follows
//!   Paseo's `import-session-sheet.tsx` and its view model, not the earlier
//!   one-agent, multi-select design (the `History` model it replaces).
//! - Reference: `fetchRecentProviderSessions` per provider, merged and sorted
//!   by activity (`aggregateSessionEntries`); already-imported sessions are
//!   not listed; `PAGE_LIMITS` 15 / 45 / 90 / 200 per provider behind "Load
//!   more"; a client filter over providers ("All" or one); per-provider
//!   failures with their own Retry; one import at a time.
//! - Constraint: the node's `agent.history` has no limit, only opaque cursor
//!   pages. A feed therefore reads page after page until it holds its limit
//!   of not-yet-imported sessions or the node has no next page.
//! - Choice: one `Feed` per provider (`Source`), each with its own tickets,
//!   rows and failure; `answer` returns the follow-up reads a feed still
//!   needs, so the owner issues them as it issues the first.
//! - Impact: the default account only (the node lists without one); saved
//!   accounts and the prompt preview wait for node work (filed).

use super::{Ask, Batch, Cursor, Entry, Listed, Outcome, Problem, Reply, Scope};
use crate::connection::ConnectionContext;
use crate::devices::{MachineKey, Ticket, Tickets};
use crate::matcher::Query;
use std::path::PathBuf;
use vessel_proto::{HarnessRef, HarnessSessionId, NodeId, SessionInfo, SessionTitle};

/// Paseo's `PAGE_LIMITS`: how many sessions a provider shows, then after each
/// "Load more".
pub const PAGE_LIMITS: [usize; 4] = [FIRST_LIMIT, 45, 90, 200];
const FIRST_LIMIT: usize = 15;

/// Reads outstanding at once. The node lists at most two at once (its
/// `service::history::AT_ONCE`) and answers more `unavailable`, which would
/// show as a failed provider; the rest wait here and go as answers come.
const READS_AT_ONCE: usize = 2;

/// The host (and folder) the sheet was opened for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The authenticated connection the reads go through.
    pub context: ConnectionContext,
    /// The host's book identity.
    pub machine: MachineKey,
    /// The workspace's folder; `None` lists the whole host.
    pub cwd: Option<PathBuf>,
}

/// A feed's read state; rows already read stay while a new read runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedState {
    /// Nothing outstanding.
    Idle,
    /// A page is outstanding.
    Pending(Ticket),
    /// A read waits for one of the host's reading slots.
    Queued,
    /// The last read failed.
    Failed(Problem),
}

#[derive(Debug)]
struct Feed {
    harness: HarnessRef,
    rows: Vec<Entry>,
    next: Option<Cursor>,
    /// A fresh read replaces rows on success; a continuation appends.
    continuing: bool,
    state: FeedState,
    node: Option<NodeId>,
    read: bool,
}

/// Which providers the list shows (Paseo's provider filter).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every provider.
    #[default]
    All,
    /// One provider.
    Provider(HarnessRef),
}

/// A listed session and its provider.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    /// The provider it was read from.
    pub harness: &'a HarnessRef,
    /// The session.
    pub entry: &'a Entry,
}

/// Why the list is empty (Paseo's `computeEmptyState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Empty {
    /// A query matches nothing.
    NoMatches,
    /// The filtered provider has none while others do.
    NoProviderSessions(HarnessRef),
    /// Every session read is already in Vessel.
    AlreadyImported,
    /// Nothing to import.
    NoRecent,
}

#[derive(Debug)]
struct Pending {
    scope: Scope,
    ticket: Ticket,
    id: HarnessSessionId,
}

/// What an answer changed and what to read next.
#[must_use]
#[derive(Debug, Default)]
pub struct Answer {
    /// The reply was the outstanding one and changed the sheet.
    pub applied: bool,
    /// Further pages a feed still needs.
    pub follow: Vec<Ask>,
}

/// The sheet's pure state; its ticket source survives closing and reopening.
#[derive(Debug, Default)]
pub struct Sheet {
    opening: Option<Opening>,
    show_all: bool,
    feeds: Vec<Feed>,
    filter: Filter,
    limit: usize,
    importing: Option<Pending>,
    import_failed: Option<Problem>,
    imported: Option<Box<SessionInfo>>,
    tickets: Tickets,
}

impl Sheet {
    /// Open for a host and its providers, reading each from the start.
    /// # Errors
    /// Ticket space is exhausted; nothing was asked.
    pub fn open(
        &mut self,
        opening: Opening,
        providers: Vec<HarnessRef>,
    ) -> Result<Vec<Ask>, Problem> {
        self.close();
        self.opening = Some(opening);
        self.feeds = providers
            .into_iter()
            .map(|harness| Feed {
                harness,
                rows: Vec::new(),
                next: None,
                continuing: false,
                state: FeedState::Idle,
                node: None,
                read: false,
            })
            .collect();
        self.refresh()
    }

    /// Forget the host and every read; an import already sent keeps its receipt.
    pub fn close(&mut self) {
        self.opening = None;
        self.show_all = false;
        self.feeds.clear();
        self.filter = Filter::All;
        self.limit = 0;
        self.import_failed = None;
        self.imported = None;
    }

    /// The host the sheet is open for.
    #[must_use]
    pub const fn opening(&self) -> Option<&Opening> {
        self.opening.as_ref()
    }

    /// Whether the list is limited to the workspace's folder.
    #[must_use]
    pub fn scoped(&self) -> bool {
        !self.show_all && self.opening.as_ref().is_some_and(|open| open.cwd.is_some())
    }

    /// Paseo's "Show all": the whole host, read again from the start.
    /// # Errors
    /// Ticket space is exhausted.
    pub fn show_all(&mut self) -> Result<Vec<Ask>, Problem> {
        if !self.scoped() || self.importing.is_some() {
            return Ok(Vec::new());
        }
        self.show_all = true;
        self.limit = 0;
        self.refresh()
    }

    /// Every provider from the start; rows stay until each answer.
    /// # Errors
    /// Ticket space is exhausted.
    pub fn refresh(&mut self) -> Result<Vec<Ask>, Problem> {
        let mut asks = Vec::new();
        for index in 0..self.feeds.len() {
            asks.extend(self.read(index, None)?);
        }
        Ok(asks)
    }

    /// One provider from the start (its banner's Retry).
    /// # Errors
    /// Ticket space is exhausted.
    pub fn retry(&mut self, harness: &HarnessRef) -> Result<Vec<Ask>, Problem> {
        let mut asks = Vec::new();
        for index in 0..self.feeds.len() {
            if self
                .feeds
                .get(index)
                .is_some_and(|feed| &feed.harness == harness)
            {
                asks.extend(self.read(index, None)?);
            }
        }
        Ok(asks)
    }

    /// Paseo's "Load more": the next limit, reading on where a provider has more.
    /// # Errors
    /// Ticket space is exhausted.
    pub fn more(&mut self) -> Result<Vec<Ask>, Problem> {
        if !self.has_more() {
            return Ok(Vec::new());
        }
        self.limit = self
            .limit
            .saturating_add(1)
            .min(PAGE_LIMITS.len().saturating_sub(1));
        let mut asks = Vec::new();
        for index in 0..self.feeds.len() {
            asks.extend(self.continue_feed(index)?);
        }
        Ok(asks)
    }

    /// The current per-provider limit.
    #[must_use]
    pub fn limit(&self) -> usize {
        PAGE_LIMITS.get(self.limit).copied().unwrap_or(FIRST_LIMIT)
    }

    /// Whether "Load more" can show more: a provider holds or may read more
    /// than the current limit, and a larger limit exists.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.limit.saturating_add(1) < PAGE_LIMITS.len()
            && self
                .feeds
                .iter()
                .any(|feed| feed.next.is_some() || listable(feed).count() > self.limit())
    }

    /// Whether any provider is reading.
    #[must_use]
    pub fn loading(&self) -> bool {
        self.feeds
            .iter()
            .any(|feed| matches!(feed.state, FeedState::Pending(_) | FeedState::Queued))
    }

    /// Whether any provider has been read at least once.
    #[must_use]
    pub fn read_any(&self) -> bool {
        self.feeds.iter().any(|feed| feed.read)
    }

    /// The providers, in the host's order (the filter's choices).
    #[must_use]
    pub fn providers(&self) -> Vec<&HarnessRef> {
        let mut providers: Vec<&HarnessRef> = Vec::new();
        for feed in &self.feeds {
            if !providers.contains(&&feed.harness) {
                providers.push(&feed.harness);
            }
        }
        providers
    }

    /// The providers whose last read failed, and why.
    #[must_use]
    pub fn failures(&self) -> Vec<(&HarnessRef, Problem)> {
        self.feeds
            .iter()
            .filter_map(|feed| match feed.state {
                FeedState::Failed(problem) => Some((&feed.harness, problem)),
                FeedState::Idle | FeedState::Pending(_) | FeedState::Queued => None,
            })
            .collect()
    }

    /// The provider filter.
    #[must_use]
    pub const fn filter(&self) -> &Filter {
        &self.filter
    }

    /// Show one provider or all of them; an unknown provider shows all.
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = match filter {
            Filter::Provider(harness) if !self.providers().contains(&&harness) => Filter::All,
            filter => filter,
        };
    }

    /// The listed sessions: not yet imported, at most the limit per
    /// provider, through the filter and the query, newest first.
    #[must_use]
    pub fn rows(&self, query: &Query) -> Vec<Row<'_>> {
        let limit = self.limit();
        let mut rows: Vec<Row<'_>> = self
            .feeds
            .iter()
            .filter(|feed| match &self.filter {
                Filter::All => true,
                Filter::Provider(harness) => &feed.harness == harness,
            })
            .flat_map(|feed| {
                listable(feed).take(limit).map(move |entry| Row {
                    harness: &feed.harness,
                    entry,
                })
            })
            .filter(|row| matches(row.entry, query))
            .collect();
        rows.sort_by(|left, right| {
            right
                .entry
                .updated_at
                .cmp(&left.entry.updated_at)
                .then_with(|| left.entry.id.as_str().cmp(right.entry.id.as_str()))
        });
        rows
    }

    /// Paseo's `computeEmptyState`: an empty state only once every provider
    /// has answered, not every one of them failed, and nothing is listed.
    #[must_use]
    pub fn empty_state(&self, query: &Query) -> Option<Empty> {
        let settled = !self.feeds.is_empty() && !self.loading();
        let all_failed = self
            .feeds
            .iter()
            .all(|feed| matches!(feed.state, FeedState::Failed(_)));
        (settled && !all_failed && self.rows(query).is_empty()).then(|| self.empty(query))
    }

    /// Why `rows(query)` is empty (Paseo's order of empty states).
    #[must_use]
    pub fn empty(&self, query: &Query) -> Empty {
        if !query.is_empty() {
            return Empty::NoMatches;
        }
        if let Filter::Provider(harness) = &self.filter
            && self.rows(&Query::new("")).is_empty()
            && self
                .feeds
                .iter()
                .any(|feed| listable(feed).next().is_some())
        {
            return Empty::NoProviderSessions(harness.clone());
        }
        if self
            .feeds
            .iter()
            .any(|feed| feed.rows.iter().any(|entry| entry.in_vessel.is_some()))
        {
            return Empty::AlreadyImported;
        }
        Empty::NoRecent
    }

    /// Import one listed session; nothing while another import or no read.
    /// # Errors
    /// Ticket space is exhausted.
    pub fn import(
        &mut self,
        harness: &HarnessRef,
        id: &HarnessSessionId,
    ) -> Result<Option<Ask>, Problem> {
        if self.importing.is_some() {
            return Ok(None);
        }
        let Some(feed) = self.feeds.iter().find(|feed| &feed.harness == harness) else {
            return Ok(None);
        };
        let Some(entry) = listable(feed).find(|entry| &entry.id == id).cloned() else {
            return Ok(None);
        };
        let (Some(node), Some(scope)) = (feed.node, self.scope(&feed.harness)) else {
            return Ok(None);
        };
        let ticket = self.ticket()?;
        self.import_failed = None;
        self.importing = Some(Pending {
            scope: scope.clone(),
            ticket,
            id: entry.id.clone(),
        });
        Ok(Some(Ask::Import(Batch {
            node,
            scope,
            ticket,
            entries: vec![entry],
        })))
    }

    /// The session being imported (its row says "Importing...").
    #[must_use]
    pub fn importing(&self) -> Option<&HarnessSessionId> {
        self.importing.as_ref().map(|pending| &pending.id)
    }

    /// Why the last import failed, until the next one.
    #[must_use]
    pub const fn import_failed(&self) -> Option<Problem> {
        self.import_failed
    }

    /// The session the last import created; the owner closes the sheet and
    /// opens it.
    pub fn take_imported(&mut self) -> Option<Box<SessionInfo>> {
        self.imported.take()
    }

    /// Accept only an outstanding reply; say what to read next.
    pub fn answer(&mut self, reply: Reply) -> Answer {
        match reply {
            Reply::Profiles { .. } => Answer::default(),
            Reply::Listed {
                scope,
                ticket,
                result,
            } => self.listed(&scope, ticket, result),
            Reply::Imported {
                scope,
                ticket,
                outcomes,
            } => {
                let applied = self.imported(&scope, ticket, outcomes);
                Answer {
                    applied,
                    follow: if applied {
                        self.queued_or_failed()
                    } else {
                        Vec::new()
                    },
                }
            }
        }
    }

    fn listed(&mut self, scope: &Scope, ticket: Ticket, result: Result<Listed, Problem>) -> Answer {
        let Some(index) = self.feeds.iter().position(|feed| {
            feed.state == FeedState::Pending(ticket)
                && self.scope(&feed.harness).as_ref() == Some(scope)
        }) else {
            return Answer::default();
        };
        let Some(feed) = self.feeds.get_mut(index) else {
            return Answer::default();
        };
        feed.read = true;
        match result {
            Ok(page) if feed.continuing && feed.node.is_some_and(|node| node != page.node) => {
                feed.state = FeedState::Failed(Problem::IdentityChanged);
            }
            Ok(page) => {
                let retained = if feed.continuing {
                    feed.rows.as_slice()
                } else {
                    &[]
                };
                if let Err(problem) = super::bounds::admit(retained, &page.entries) {
                    feed.state = FeedState::Failed(problem);
                    return Answer {
                        applied: true,
                        follow: Vec::new(),
                    };
                }
                if !feed.continuing {
                    feed.rows.clear();
                }
                for entry in page.entries {
                    if let Some(old) = feed.rows.iter_mut().find(|old| old.id == entry.id) {
                        *old = entry;
                    } else {
                        feed.rows.push(entry);
                    }
                }
                feed.rows.sort_by(|left, right| {
                    right
                        .updated_at
                        .cmp(&left.updated_at)
                        .then_with(|| left.id.as_str().cmp(right.id.as_str()))
                });
                feed.node = Some(page.node);
                feed.next = page.next;
                feed.state = FeedState::Idle;
            }
            Err(problem) => feed.state = FeedState::Failed(problem),
        }
        let mut follow: Vec<Ask> = match self.continue_feed(index) {
            Ok(ask) => ask.into_iter().collect(),
            Err(problem) => {
                if let Some(feed) = self.feeds.get_mut(index) {
                    feed.state = FeedState::Failed(problem);
                }
                Vec::new()
            }
        };
        follow.extend(self.queued_or_failed());
        Answer {
            applied: true,
            follow,
        }
    }

    fn imported(&mut self, scope: &Scope, ticket: Ticket, outcomes: Vec<Outcome>) -> bool {
        let Some(pending) = &self.importing else {
            return false;
        };
        if pending.ticket != ticket || &pending.scope != scope {
            return false;
        }
        let id = pending.id.clone();
        self.importing = None;
        let receipt = outcomes.into_iter().find(|outcome| outcome.id == id);
        match receipt.map(|outcome| outcome.result) {
            Some(Ok(info)) => {
                for feed in &mut self.feeds {
                    if let Some(entry) = feed.rows.iter_mut().find(|entry| entry.id == id) {
                        entry.in_vessel = Some(info.id);
                    }
                }
                self.imported = Some(info);
            }
            Some(Err(problem)) => self.import_failed = Some(problem),
            None => self.import_failed = Some(Problem::InvalidReply),
        }
        true
    }

    /// The scope a provider is read and imported through.
    fn scope(&self, harness: &HarnessRef) -> Option<Scope> {
        let opening = self.opening.as_ref()?;
        Some(Scope {
            context: opening.context.clone(),
            machine: opening.machine.clone(),
            harness: harness.clone(),
            profile: None,
            cwd: if self.show_all {
                None
            } else {
                opening.cwd.clone()
            },
        })
    }

    fn ticket(&mut self) -> Result<Ticket, Problem> {
        // Tickets only returns the fieldless Exhausted variant; keep its meaning.
        self.tickets
            .next_ticket()
            .map_err(|_exhausted| Problem::Exhausted)
    }

    /// Read a feed from `cursor` (the start without one).
    fn read(&mut self, index: usize, cursor: Option<Cursor>) -> Result<Option<Ask>, Problem> {
        if self.importing.is_some() {
            return Ok(None);
        }
        let Some(harness) = self.feeds.get(index).map(|feed| feed.harness.clone()) else {
            return Ok(None);
        };
        let Some(scope) = self.scope(&harness) else {
            return Ok(None);
        };
        let in_flight = self
            .feeds
            .iter()
            .filter(|feed| matches!(feed.state, FeedState::Pending(_)))
            .count();
        let Some(feed) = self.feeds.get_mut(index) else {
            return Ok(None);
        };
        feed.continuing = cursor.is_some();
        if cursor.is_none() {
            feed.next = None;
        }
        // A feed already reading gives its slot to this read.
        if in_flight >= READS_AT_ONCE && !matches!(feed.state, FeedState::Pending(_)) {
            feed.state = FeedState::Queued;
            return Ok(None);
        }
        let ticket = self.ticket()?;
        if let Some(feed) = self.feeds.get_mut(index) {
            feed.state = FeedState::Pending(ticket);
        }
        Ok(Some(Ask::List {
            scope,
            ticket,
            cursor,
        }))
    }

    /// The waiting reads that can go now; out of tickets, they fail as one.
    fn queued_or_failed(&mut self) -> Vec<Ask> {
        self.dispatch_queued().unwrap_or_else(|problem| {
            for feed in &mut self.feeds {
                if feed.state == FeedState::Queued {
                    feed.state = FeedState::Failed(problem);
                }
            }
            Vec::new()
        })
    }

    /// Send the waiting reads, in the host's order, while slots are free.
    fn dispatch_queued(&mut self) -> Result<Vec<Ask>, Problem> {
        let mut asks = Vec::new();
        if self.importing.is_some() {
            return Ok(asks);
        }
        while let Some(index) = self
            .feeds
            .iter()
            .position(|feed| feed.state == FeedState::Queued)
        {
            let cursor = self
                .feeds
                .get(index)
                .and_then(|feed| feed.continuing.then(|| feed.next.clone()).flatten());
            if let Some(feed) = self.feeds.get_mut(index) {
                feed.state = FeedState::Idle;
            }
            match self.read(index, cursor)? {
                Some(ask) => asks.push(ask),
                // Still no slot (or an import began): it waits again.
                None => break,
            }
        }
        Ok(asks)
    }

    /// Read on while a feed holds fewer listable sessions than the limit.
    fn continue_feed(&mut self, index: usize) -> Result<Option<Ask>, Problem> {
        let limit = self.limit();
        let Some(feed) = self.feeds.get(index) else {
            return Ok(None);
        };
        if feed.state != FeedState::Idle || listable(feed).count() >= limit {
            return Ok(None);
        }
        match feed.next.clone() {
            Some(cursor) => self.read(index, Some(cursor)),
            None => Ok(None),
        }
    }
}

/// A feed's sessions not yet in Vessel (Paseo lists only those).
fn listable(feed: &Feed) -> impl Iterator<Item = &Entry> {
    feed.rows.iter().filter(|entry| entry.in_vessel.is_none())
}

/// Paseo searches titles; the folder is matched too, as it is shown.
fn matches(entry: &Entry, query: &Query) -> bool {
    let path = entry.cwd.to_string_lossy();
    let title = entry.title.as_ref().map_or("", SessionTitle::as_str);
    query.is_empty() || query.score(&[title, &path], None).is_some()
}

#[cfg(test)]
mod tests;
