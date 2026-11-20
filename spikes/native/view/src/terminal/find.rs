//! Find in a terminal: Paseo's terminal find (`terminal/find`, xterm.js's
//! search addon as `terminal-emulator-runtime.ts` sets it up).
//!
//! [Decision Log]
//! - Purpose: find a word in a terminal's history and screen, every match
//!   marked and one current, as Paseo's desktop terminal does.
//! - Paseo's rules: literal text, case-insensitive, no regex and no whole
//!   word (`regex: false`, `caseSensitive: false`); matches are counted up
//!   to [`FIND_LIMIT`] (`FIND_HIGHLIGHT_LIMIT`) and the count then shows a
//!   "+"; a match may cross a soft wrap; matches do not overlap.
//! - Constraint: history holds up to 1,000,000 lines; a search over all of
//!   them must not hold the window while the person types.
//! - Choice: a [`Finder`] goes through logical lines in steps of a row
//!   budget, so the screen runs it a slice a frame (the conversation
//!   find's pattern) and drops it for a newer query. Matches are kept by
//!   their cells' [`TextAt`], so they stay on their text as output
//!   scrolls.
//! - Cost: a step reads the grid, so it runs only while the terminal's
//!   model is home from its parse; output while the bar is open makes the
//!   screen search again.

use super::grid::{Grid, HistoryRow, TextAt};

/// The most matches counted (Paseo's `FIND_HIGHLIGHT_LIMIT`); past it the
/// count shows a "+".
pub const FIND_LIMIT: usize = 20_000;

/// The longest query searched, in scalars; a longer one is cut there.
pub const MAX_FIND_QUERY: usize = 1_024;

/// One match: its first and last cell, both included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundMatch {
    /// The first cell.
    pub start: TextAt,
    /// The last cell.
    pub end: TextAt,
}

/// A search through one grid, a slice at a time.
#[derive(Debug, Clone)]
pub struct Finder {
    /// The query, lowercased.
    query: Vec<char>,
    /// The logical line searched next; `None` once done.
    next: Option<HistoryRow>,
    matches: Vec<FoundMatch>,
    limited: bool,
}

impl Finder {
    /// A search for `query` from the oldest row `grid` can scroll to;
    /// `None` for an empty query.
    #[must_use]
    pub fn new(query: &str, grid: &Grid) -> Option<Self> {
        let query: Vec<char> = query
            .chars()
            .take(MAX_FIND_QUERY)
            .flat_map(char::to_lowercase)
            .collect();
        (!query.is_empty()).then(|| Self {
            query,
            next: Some(grid.text_first()),
            matches: Vec::new(),
            limited: false,
        })
    }

    /// Search about `rows` more rows (whole logical lines). Whether the
    /// search is done.
    pub fn step(&mut self, grid: &Grid, rows: usize) -> bool {
        let mut left = rows;
        while left > 0
            && let Some(row) = self.next
        {
            // Rows dropped from the top since the last step are skipped.
            let row = row.max(grid.text_first());
            let Some((line, next)) = grid.logical_line(row) else {
                self.next = None;
                break;
            };
            left = left.saturating_sub(next.index_from(row).unwrap_or(1).max(1));
            self.next = Some(next);
            if !self.search_line(&line.chars, &line.places) {
                self.limited = true;
                self.next = None;
            }
        }
        self.next.is_none()
    }

    /// Record the matches in one logical line; `false` once the limit is
    /// reached.
    fn search_line(&mut self, chars: &[char], places: &[TextAt]) -> bool {
        let printed = chars
            .iter()
            .rposition(|c| *c != ' ')
            .map_or(0, |last| last.saturating_add(1));
        let mut lowered = Vec::with_capacity(printed);
        let mut where_ = Vec::with_capacity(printed);
        for (c, place) in chars.iter().zip(places).take(printed) {
            for lower in c.to_lowercase() {
                lowered.push(lower);
                where_.push(*place);
            }
        }
        let width = self.query.len();
        let mut at = 0_usize;
        while let Some(end) = at.checked_add(width)
            && end <= lowered.len()
        {
            if lowered.get(at..end) == Some(self.query.as_slice()) {
                let last = end.saturating_sub(1);
                if let (Some(start), Some(end)) = (where_.get(at), where_.get(last)) {
                    if self.matches.len() >= FIND_LIMIT {
                        return false;
                    }
                    self.matches.push(FoundMatch {
                        start: *start,
                        end: *end,
                    });
                }
                at = end;
            } else {
                at = at.saturating_add(1);
            }
        }
        true
    }

    /// Whether every row was searched (or the limit stopped it).
    #[must_use]
    pub const fn done(&self) -> bool {
        self.next.is_none()
    }

    /// The matches so far, oldest first.
    #[must_use]
    pub fn matches(&self) -> &[FoundMatch] {
        &self.matches
    }

    /// Whether the count stopped at [`FIND_LIMIT`] (shown with a "+").
    #[must_use]
    pub const fn limited(&self) -> bool {
        self.limited
    }
}

/// The match a search shows first: the first that starts at or after
/// `from` (the view's top, as xterm.js searches forward from the
/// viewport), else the first of all.
#[must_use]
pub fn first_from(matches: &[FoundMatch], from: TextAt) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .iter()
            .position(|found| found.start >= from)
            .unwrap_or(0),
    )
}

/// The match after `current`, round to the first.
#[must_use]
pub const fn next_of(current: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else if current.saturating_add(1) >= count {
        0
    } else {
        current.saturating_add(1)
    }
}

/// The match before `current`, round to the last.
#[must_use]
pub const fn previous_of(current: usize, count: usize) -> usize {
    match current.checked_sub(1) {
        Some(before) if before < count => before,
        _ => count.saturating_sub(1),
    }
}
