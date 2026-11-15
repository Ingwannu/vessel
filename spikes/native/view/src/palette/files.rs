//! Presentation lifetime for command-center file search. C5 owns acquisition
//! and its request tickets; this model only invalidates UI work before debounce
//! and retains rows while another query loads in the same owner-supplied scope.

pub use crate::workspace::file_query::FileQuery;
use std::time::Duration;
use vessel_proto::workspace::InvalidText;

/// Nonempty queries wait briefly; empty files scope lists immediately.
pub const FILE_SEARCH_DEBOUNCE: Duration = Duration::from_millis(100);

/// Parse UI text into the file owner's canonical acquisition intent.
/// ECMAScript blank text requests a listing, never an invalid empty wire query.
///
/// # Errors
/// Nonblank text must satisfy the file owner's bounded query grammar.
pub fn parse_query(text: &str) -> Result<FileQuery, InvalidText> {
    if crate::matcher::Query::new(text).is_empty() {
        Ok(FileQuery::All)
    } else {
        text.parse().map(FileQuery::Matching)
    }
}

/// Scope is an opaque feature-owned address, not an authority inferred here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTicket<S> {
    serial: u64,
    scope: S,
    query: FileQuery,
}
impl<S> SearchTicket<S> {
    /// Exact scope captured when the query changed.
    #[must_use]
    pub const fn scope(&self) -> &S {
        &self.scope
    }
    /// Explicit initial listing or validated owner search text.
    #[must_use]
    pub const fn query(&self) -> &FileQuery {
        &self.query
    }
}

/// Loading and failure suppress the generic no-results message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPhase<E> {
    /// Closed, disabled or without an active workspace.
    Idle,
    /// A cancellable debounce has not dispatched yet.
    Waiting,
    /// The file owner has admitted this request.
    Loading,
    /// Current query completed, possibly with no matches.
    Ready,
    /// A prepared UI error; raw protocol errors stay at their owner boundary.
    Failed(E),
}

/// UI request identities must never wrap and admit retained callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("command-center file search identities exhausted")]
pub struct SearchExhausted;

/// One window's file-search presentation, independent of network controller.
#[derive(Debug)]
pub struct FileSearch<S, R, E> {
    serial: u64,
    active: Option<SearchTicket<S>>,
    due: Duration,
    phase: SearchPhase<E>,
    rows: Vec<R>,
}
impl<S, R, E> Default for FileSearch<S, R, E> {
    fn default() -> Self {
        Self {
            serial: 0,
            active: None,
            due: Duration::ZERO,
            phase: SearchPhase::Idle,
            rows: Vec::new(),
        }
    }
}
impl<S: Clone + Eq, R, E> FileSearch<S, R, E> {
    /// Invalidate an old response at input time, before a timer or RPC runs.
    ///
    /// # Errors
    /// Exhaustion cancels pending work rather than reusing a callback identity.
    pub fn begin(
        &mut self,
        scope: S,
        query: FileQuery,
        now: Duration,
    ) -> Result<SearchTicket<S>, SearchExhausted> {
        let Some(serial) = self.serial.checked_add(1) else {
            self.cancel();
            return Err(SearchExhausted);
        };
        if self.active.as_ref().is_none_or(|old| old.scope != scope) {
            self.rows.clear();
        }
        let delay = match &query {
            FileQuery::All => Duration::ZERO,
            FileQuery::Matching(_) => FILE_SEARCH_DEBOUNCE,
        };
        let ticket = SearchTicket {
            serial,
            scope,
            query,
        };
        self.serial = serial;
        self.due = now.saturating_add(delay);
        self.active = Some(ticket.clone());
        self.phase = SearchPhase::Waiting;
        Ok(ticket)
    }

    /// Only the current timer may dispatch, and only once at its deadline.
    #[must_use]
    pub fn dispatch(&mut self, ticket: &SearchTicket<S>, now: Duration) -> bool {
        if self.active.as_ref() != Some(ticket)
            || !matches!(self.phase, SearchPhase::Waiting)
            || now < self.due
        {
            return false;
        }
        self.phase = SearchPhase::Loading;
        true
    }

    /// A request result cannot populate another workspace/query/opening.
    #[must_use]
    pub fn settle(&mut self, ticket: &SearchTicket<S>, result: Result<Vec<R>, E>) -> bool {
        if self.active.as_ref() != Some(ticket) || !matches!(self.phase, SearchPhase::Loading) {
            return false;
        }
        match result {
            Ok(rows) => {
                self.rows = rows;
                self.phase = SearchPhase::Ready;
            }
            Err(error) => {
                self.rows.clear();
                self.phase = SearchPhase::Failed(error);
            }
        }
        true
    }

    /// Closing, mode changes and loss of owner scope remove retained results.
    pub fn cancel(&mut self) {
        self.active = None;
        self.phase = SearchPhase::Idle;
        self.rows.clear();
    }
    /// Rows retain file-owner ordering without a second local filename search.
    #[must_use]
    pub fn rows(&self) -> &[R] {
        &self.rows
    }
    /// Presentation state for loading/error/no-results and arrow-key policy.
    #[must_use]
    pub const fn phase(&self) -> &SearchPhase<E> {
        &self.phase
    }
    /// Arrows stand down while results belong to an earlier query.
    #[must_use]
    pub const fn loading(&self) -> bool {
        matches!(self.phase, SearchPhase::Waiting | SearchPhase::Loading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type Search = FileSearch<&'static str, u8, &'static str>;
    fn query(text: &str) -> FileQuery {
        parse_query(text).unwrap()
    }
    #[test]
    fn blank_listing_uses_reference_whitespace_without_weakening_wire_queries() {
        for text in ["", " \t", "\u{feff}"] {
            assert_eq!(query(text), FileQuery::All);
        }
        // ECMAScript does not trim NEL; the existing one-line protocol grammar
        // refuses it rather than silently turning that query into a listing.
        assert!(!crate::matcher::Query::new("\u{85}").is_empty());
        assert!(parse_query("\u{85}").is_err());
    }
    #[test]
    fn input_invalidates_before_debounce_and_keeps_same_scope_rows() {
        let mut search = Search::default();
        let a = search.begin("scope-a", query(""), Duration::ZERO).unwrap();
        assert!(search.dispatch(&a, Duration::ZERO));
        assert!(search.settle(&a, Ok(vec![1, 2])));
        let next = search
            .begin("scope-a", query("next"), Duration::ZERO)
            .unwrap();
        assert!(search.loading());
        assert_eq!(search.rows(), [1, 2]);
        assert!(!search.settle(&a, Ok(vec![9])));
        assert!(!search.dispatch(&next, Duration::from_millis(99)));
        assert!(search.dispatch(&next, FILE_SEARCH_DEBOUNCE));
        assert!(!search.dispatch(&next, FILE_SEARCH_DEBOUNCE));
        assert!(search.settle(&next, Err("prepared refusal")));
        assert!(search.rows().is_empty());
        assert_eq!(search.phase(), &SearchPhase::Failed("prepared refusal"));
    }
    #[test]
    fn repeated_scope_and_close_reopen_never_reuse_ui_request_identity() {
        let mut search = Search::default();
        let old = search.begin("a", query("same"), Duration::ZERO).unwrap();
        assert!(search.dispatch(&old, FILE_SEARCH_DEBOUNCE));
        let _b = search.begin("b", query("same"), Duration::ZERO).unwrap();
        let current = search.begin("a", query("same"), Duration::ZERO).unwrap();
        assert!(!search.settle(&old, Ok(vec![9])));
        assert!(!search.dispatch(&old, FILE_SEARCH_DEBOUNCE));
        assert!(search.dispatch(&current, FILE_SEARCH_DEBOUNCE));
        assert!(search.settle(&current, Ok(vec![3])));
        search.cancel();
        let reopened = search.begin("a", query("same"), Duration::ZERO).unwrap();
        assert_ne!(current, reopened);
        assert!(!search.settle(&current, Ok(vec![9])));
        assert!(search.rows().is_empty());
    }
}
