//! Selection identity survives passive refreshes. String-backed entries retain
//! their original API while contribution ids stay typed; both use the same
//! comparison and wrapping policy below.

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
}

impl<I: Clone + PartialEq> PaletteCursor<I> {
    /// Resolve an opaque source or file identity without string encoding.
    #[must_use]
    pub fn resolved_identity<'a>(&self, ids: &[&'a I]) -> Option<&'a I> {
        kept_ref(self.active.as_ref(), ids)
    }

    /// Retain the actual fallback when a source disappears during a refresh.
    pub fn settle_identity<'a>(&mut self, ids: &[&'a I]) -> Option<&'a I> {
        let active = self.resolved_identity(ids);
        self.active = active.cloned();
        active
    }

    /// Move through typed source identities with the existing wrapping policy.
    pub fn advance_identity<'a>(&mut self, ids: &[&'a I], direction: Direction) -> Option<&'a I> {
        let next = moved_ref(self.resolved_identity(ids), ids, direction);
        self.active = next.cloned();
        next
    }
}

impl PaletteCursor<String> {
    /// The current id if still listed, otherwise the first result.
    #[must_use]
    pub fn resolved<'a>(&self, ids: &[&'a str]) -> Option<&'a str> {
        kept_ref(self.active.as_deref(), ids)
    }

    /// Accept a passive refresh without requesting a scroll.
    pub fn settle<'a>(&mut self, ids: &[&'a str]) -> Option<&'a str> {
        let active = self.resolved(ids);
        self.active = active.map(str::to_owned);
        active
    }

    /// Move once; passive refreshes never commandeer the viewport.
    pub fn advance<'a>(&mut self, ids: &[&'a str], direction: Direction) -> Option<&'a str> {
        let next = moved_ref(self.resolved(ids), ids, direction);
        self.active = next.map(str::to_owned);
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
        assert_eq!(
            cursor.advance_identity(&[&one, &two], Direction::Next),
            Some(&two)
        );
        assert_eq!(cursor.settle_identity(&[&two]), Some(&two));
        assert_eq!(cursor.settle_identity(&[&one, &two]), Some(&two));
        cursor.reset();
        assert_eq!(cursor.resolved_identity(&[&one, &two]), Some(&one));
    }
}
