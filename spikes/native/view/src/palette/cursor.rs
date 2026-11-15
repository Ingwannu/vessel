//! Selection identity survives passive fleet refreshes. A query is a
//! command to choose afresh: keeping a previously selected result after
//! re-ranking would make Enter act on a lower-ranked result.

use std::borrow::Borrow;

/// A keyboard move through the palette's projected result order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The previous row, wrapping at the start.
    Previous,
    /// The next row, wrapping at the end.
    Next,
}

/// Stable keyboard selection, independent of the native scroll handle.
#[derive(Debug)]
pub struct PaletteCursor<I = String> {
    active: Option<I>,
}

impl<I> Default for PaletteCursor<I> {
    fn default() -> Self {
        Self { active: None }
    }
}

impl<I> PaletteCursor<I> {
    /// Opening, closing or typing a query discards the previous selection.
    pub fn reset(&mut self) {
        self.active = None;
    }

    /// The current id if still listed, otherwise the first result.
    #[must_use]
    pub fn resolved<'a, Q: PartialEq + ?Sized>(&self, ids: &[&'a Q]) -> Option<&'a Q>
    where
        I: Borrow<Q>,
    {
        kept_ref(self.active.as_ref().map(Borrow::borrow), ids)
    }

    /// Accept a passive result refresh without requesting a scroll. The
    /// fallback becomes the new identity, so a removed row reappearing
    /// later cannot silently take selection back.
    pub fn settle<'a, Q: PartialEq + ToOwned<Owned = I> + ?Sized>(
        &mut self,
        ids: &[&'a Q],
    ) -> Option<&'a Q>
    where
        I: Borrow<Q>,
    {
        let active = self.resolved(ids);
        self.active = active.map(ToOwned::to_owned);
        active
    }

    /// Move once in the current projection. The caller reveals this row;
    /// a passive refresh calls `settle` without moving the viewport.
    pub fn advance<'a, Q: PartialEq + ToOwned<Owned = I> + ?Sized>(
        &mut self,
        ids: &[&'a Q],
        direction: Direction,
    ) -> Option<&'a Q>
    where
        I: Borrow<Q>,
    {
        let next = moved_ref(self.resolved(ids), ids, direction);
        self.active = next.map(ToOwned::to_owned);
        next
    }
}

/// Generic identity comparison keeps plugin/source ids typed all the way to Enter.
pub(super) fn kept_ref<'a, I: PartialEq + ?Sized>(
    active: Option<&I>,
    ids: &[&'a I],
) -> Option<&'a I> {
    ids.iter()
        .copied()
        .find(|id| Some(*id) == active)
        .or_else(|| ids.first().copied())
}

pub(super) fn moved_ref<'a, I: PartialEq + ?Sized>(
    active: Option<&I>,
    ids: &[&'a I],
    direction: Direction,
) -> Option<&'a I> {
    let last = ids.len().checked_sub(1)?;
    let current = active.and_then(|active| ids.iter().position(|id| *id == active));
    let next = match (current, direction) {
        (None, Direction::Next) => 0,
        (None, Direction::Previous) => last,
        (Some(at), Direction::Next) if at >= last => 0,
        (Some(at), Direction::Next) => at.saturating_add(1),
        (Some(at), Direction::Previous) => at.checked_sub(1).unwrap_or(last),
    };
    ids.get(next).copied()
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

#[cfg(test)]
mod typed_identity_tests {
    use super::{Direction, PaletteCursor};
    use crate::palette::registration::ResultId;

    #[test]
    fn source_and_local_identity_survive_refresh_without_string_encoding() {
        let one = ResultId {
            source: "a:b".parse().unwrap(),
            entry: "c".parse().unwrap(),
        };
        let two = ResultId {
            source: "a".parse().unwrap(),
            entry: "b:c".parse().unwrap(),
        };
        let mut cursor: PaletteCursor<ResultId> = PaletteCursor::default();
        assert_eq!(cursor.advance(&[&one, &two], Direction::Next), Some(&two));
        assert_eq!(cursor.settle(&[&two]), Some(&two));
        assert_eq!(cursor.settle(&[&one, &two]), Some(&two));
        cursor.reset();
        assert_eq!(cursor.resolved(&[&one, &two]), Some(&one));
    }
}
