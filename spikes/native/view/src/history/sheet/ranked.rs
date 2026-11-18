//! The sheet against a node of `RANKED_HISTORY_PROTOCOL` (TRACKER 790, ADR
//! 0109): Paseo's own reads. Each provider is asked its limit and the
//! search; the node answers that many sessions not yet in Vessel, each with
//! its prompts, and how many it left out.
//!
//! [Decision Log]
//! - Purpose: the 777 sheet stood in for what the node could not do (a
//!   client filter over cursor pages, a two-read queue, no preview). This
//!   drops the stand-ins on a node that can do it.
//! - Choice: Paseo's `useImportSession` rules. A search, sent by the app
//!   after Paseo's 200 ms debounce, starts again at the first limit ("a
//!   narrower or wider list starts at page one"). "Load more" asks every
//!   provider again with the next limit, and the rows shown stay until each
//!   answer. "Load more" stays while a provider answered a full page, or
//!   still shows rows while its larger read runs (Paseo's
//!   `hasMoreSessions` with placeholder data).

use super::{FeedState, PAGE_LIMITS, Sheet};
use crate::history::{Ask, Problem, Ranked};

impl Sheet {
    /// Whether the host answers ranked listings (`Opening::ranked`).
    #[must_use]
    pub fn ranked(&self) -> bool {
        self.opening.as_ref().is_some_and(|opening| opening.ranked)
    }

    /// The node's search: `text` trimmed, read again from the first limit
    /// when it changed. Nothing on an older node, whose rows the app's query
    /// filters (`rows`).
    /// # Errors
    /// Ticket space is exhausted.
    pub fn search(&mut self, text: &str) -> Result<Vec<Ask>, Problem> {
        let query = text.trim();
        if !self.ranked() || query == self.query {
            return Ok(Vec::new());
        }
        query.clone_into(&mut self.query);
        self.limit = 0;
        self.refresh()
    }

    /// What a ranked read asks now.
    pub(super) fn ranked_ask(&self) -> Ranked {
        Ranked {
            limit: u16::try_from(self.limit()).unwrap_or(u16::MAX),
            query: (!self.query.is_empty()).then(|| self.query.clone()),
        }
    }

    /// Paseo's `hasMoreSessions`: a larger limit exists and a provider
    /// answered a full page, or still shows rows while its next read runs.
    pub(super) fn ranked_has_more(&self) -> bool {
        let limit = self.limit();
        self.limit.saturating_add(1) < PAGE_LIMITS.len()
            && self.feeds.iter().any(|feed| match feed.state {
                FeedState::Pending(_) | FeedState::Queued => !feed.rows.is_empty(),
                FeedState::Idle | FeedState::Failed(_) => feed.rows.len() >= limit,
            })
    }

    /// Sessions the node left out as already in Vessel, every provider
    /// together (Paseo's `sumFilteredAlreadyImportedCount`).
    #[must_use]
    pub fn already_imported(&self) -> u32 {
        self.feeds.iter().fold(0, |sum: u32, feed| {
            sum.saturating_add(feed.already_imported)
        })
    }
}

#[cfg(test)]
mod tests;
