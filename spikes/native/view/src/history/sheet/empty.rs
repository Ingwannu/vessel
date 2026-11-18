//! Paseo's `computeEmptyState`: why the list is empty, once it may say so.

use super::{Empty, FeedState, Filter, Sheet, listable};
use crate::matcher::Query;

impl Sheet {
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
        if self.already_imported() > 0
            || self
                .feeds
                .iter()
                .any(|feed| feed.rows.iter().any(|entry| entry.in_vessel.is_some()))
        {
            return Empty::AlreadyImported;
        }
        Empty::NoRecent
    }
}
