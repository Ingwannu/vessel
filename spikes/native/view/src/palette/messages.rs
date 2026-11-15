//! Palette scopes and message-row selection; no fetching or presentation.
//!
//! Message identity includes its machine and record, so late replies cannot
//! move a keyboard selection to another session with the same title.

use crate::search::Row;

use super::Direction;

/// An explicit scope keeps command lookup from querying every transcript.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PaletteMode {
    /// Local command and session-name matching, available without a node call.
    #[default]
    Commands,
    /// Search message contents through every reachable node's index.
    Messages,
}

/// Stable message selection, including the paging action as a real row.
#[derive(Debug, Default)]
pub struct MessageCursor {
    active: Option<Row>,
}

impl MessageCursor {
    /// A new query or scope must not retain a previous row's action.
    pub fn reset(&mut self) {
        self.active = None;
    }

    /// Keep a selected identity across passive replies; otherwise choose first.
    #[must_use]
    pub fn resolved<'a>(&self, rows: &'a [Row]) -> Option<&'a Row> {
        self.active
            .as_ref()
            .and_then(|active| rows.iter().find(|row| *row == active))
            .or_else(|| rows.first())
    }

    /// Commit a passive fallback, without granting permission to scroll.
    pub fn settle<'a>(&mut self, rows: &'a [Row]) -> Option<&'a Row> {
        let next = self.resolved(rows);
        self.active = next.cloned();
        next
    }

    /// Wrap through the projected rows, returning the row to reveal.
    pub fn advance(&mut self, rows: &[Row], direction: Direction) -> Option<usize> {
        let Some(last) = rows.len().checked_sub(1) else {
            self.reset();
            return None;
        };
        let current = self
            .resolved(rows)
            .and_then(|active| rows.iter().position(|row| row == active))?;
        // Saturation keeps navigation total even at an empty/changed boundary.
        let next = match direction {
            Direction::Next if current >= last => 0,
            Direction::Next => current.saturating_add(1),
            Direction::Previous => current.checked_sub(1).unwrap_or(last),
        };
        self.active = rows.get(next).cloned();
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::{EventSeq, SessionId};

    use super::*;
    use crate::search::HitId;

    fn hit(seq: u64) -> Row {
        Row::Hit(HitId {
            machine: "test".to_owned(),
            session: SessionId::random(),
            seq: EventSeq::new(seq),
        })
    }

    #[test]
    fn scope_defaults_to_commands_and_navigation_wraps_through_more() {
        assert_eq!(PaletteMode::default(), PaletteMode::Commands);
        let rows = [hit(1), hit(2), Row::More];
        let mut cursor = MessageCursor::default();
        assert_eq!(cursor.resolved(&rows), rows.first());
        assert_eq!(cursor.advance(&rows, Direction::Previous), Some(2));
        assert_eq!(cursor.resolved(&rows), Some(&Row::More));
        assert_eq!(cursor.advance(&rows, Direction::Next), Some(0));
        assert_eq!(cursor.advance(&[], Direction::Next), None);
        cursor.reset();
        assert_eq!(cursor.resolved(&[]), None);
    }

    #[test]
    fn passive_replies_preserve_identity_and_new_queries_discard_it() {
        let first = hit(1);
        let second = hit(2);
        let rows = [first.clone(), second.clone()];
        let mut cursor = MessageCursor::default();
        assert_eq!(cursor.advance(&rows, Direction::Next), Some(1));
        assert_eq!(
            cursor.settle(&[hit(3), first.clone(), second.clone()]),
            Some(&second)
        );
        assert_eq!(cursor.settle(std::slice::from_ref(&first)), Some(&first));
        assert_eq!(cursor.settle(&rows), Some(&first));
        assert_eq!(cursor.advance(&rows, Direction::Next), Some(1));
        cursor.reset();
        assert_eq!(cursor.resolved(&rows), Some(&first));
    }
}
