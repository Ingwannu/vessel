//! The sheet's reads and receipts: which reply is outstanding, follow-up
//! pages, the host's two reading slots and the single import's receipt.

use super::{Answer, FeedState, READS_AT_ONCE, Sheet, listable};
use crate::devices::Ticket;
use crate::history::{Ask, Cursor, Listed, Outcome, Problem, Scope};
use vessel_proto::HarnessRef;

impl Sheet {
    pub(super) fn listed(
        &mut self,
        scope: &Scope,
        ticket: Ticket,
        result: Result<Listed, Problem>,
    ) -> Answer {
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
                if let Err(problem) = crate::history::bounds::admit(retained, &page.entries) {
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

    pub(super) fn imported(
        &mut self,
        scope: &Scope,
        ticket: Ticket,
        outcomes: Vec<Outcome>,
    ) -> bool {
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
    pub(super) fn scope(&self, harness: &HarnessRef) -> Option<Scope> {
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

    pub(super) fn ticket(&mut self) -> Result<Ticket, Problem> {
        // Tickets only returns the fieldless Exhausted variant; keep its meaning.
        self.tickets
            .next_ticket()
            .map_err(|_exhausted| Problem::Exhausted)
    }

    /// Read a feed from `cursor` (the start without one).
    pub(super) fn read(
        &mut self,
        index: usize,
        cursor: Option<Cursor>,
    ) -> Result<Option<Ask>, Problem> {
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
    pub(super) fn queued_or_failed(&mut self) -> Vec<Ask> {
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
    pub(super) fn dispatch_queued(&mut self) -> Result<Vec<Ask>, Problem> {
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
    pub(super) fn continue_feed(&mut self, index: usize) -> Result<Option<Ask>, Problem> {
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
