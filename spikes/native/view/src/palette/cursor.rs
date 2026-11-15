//! Selection identity survives passive fleet refreshes. A query is a
//! command to choose afresh: keeping a previously selected result after
//! re-ranking would make Enter act on a lower-ranked result.

use super::{kept, moved};

/// A keyboard move through the palette's projected result order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The previous row, wrapping at the start.
    Previous,
    /// The next row, wrapping at the end.
    Next,
}

/// Stable keyboard selection, independent of the native scroll handle.
#[derive(Debug, Default)]
pub struct PaletteCursor {
    active: Option<String>,
}

impl PaletteCursor {
    /// Opening, closing or typing a query discards the previous selection.
    pub fn reset(&mut self) {
        self.active = None;
    }

    /// The current id if still listed, otherwise the first result.
    #[must_use]
    pub fn resolved<'a>(&self, ids: &[&'a str]) -> Option<&'a str> {
        kept(self.active.as_deref(), ids)
    }

    /// Accept a passive result refresh without requesting a scroll. The
    /// fallback becomes the new identity, so a removed row reappearing
    /// later cannot silently take selection back.
    pub fn settle<'a>(&mut self, ids: &[&'a str]) -> Option<&'a str> {
        let active = self.resolved(ids);
        self.active = active.map(str::to_owned);
        active
    }

    /// Move once in the current projection. The caller reveals this row;
    /// a passive refresh calls `settle` without moving the viewport.
    pub fn advance<'a>(&mut self, ids: &[&'a str], direction: Direction) -> Option<&'a str> {
        let next = moved(self.resolved(ids), ids, direction == Direction::Next);
        self.active = next.map(str::to_owned);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, PaletteCursor};

    #[test]
    fn a_query_selects_the_best_result_even_if_the_previous_id_still_matches() {
        let mut cursor = PaletteCursor::default();
        assert_eq!(
            cursor.advance(&["a", "b", "c", "d"], Direction::Next),
            Some("b")
        );
        cursor.reset();
        assert_eq!(cursor.resolved(&["a", "b"]), Some("a"));
    }

    #[test]
    fn passive_refresh_keeps_selection_while_removal_falls_back_to_the_first() {
        let mut cursor = PaletteCursor::default();
        assert_eq!(
            cursor.advance(&["a", "b", "c", "d"], Direction::Next),
            Some("b")
        );
        assert_eq!(cursor.settle(&["a", "b", "c"]), Some("b"));
        assert_eq!(cursor.settle(&["c", "d"]), Some("c"));
        assert_eq!(cursor.settle(&["a", "b", "c"]), Some("c"));
        assert_eq!(cursor.settle(&[]), None);
    }

    #[test]
    fn closing_discards_selection_and_arrows_wrap() {
        let mut cursor = PaletteCursor::default();
        assert_eq!(
            cursor.advance(&["a", "b", "c", "d"], Direction::Previous),
            Some("d")
        );
        assert_eq!(
            cursor.advance(&["a", "b", "c", "d"], Direction::Next),
            Some("a")
        );
        cursor.reset();
        assert_eq!(cursor.resolved(&["a", "b"]), Some("a"));
        assert_eq!(cursor.advance(&["a"], Direction::Next), Some("a"));
        assert_eq!(cursor.advance(&[], Direction::Next), None);
    }
}
