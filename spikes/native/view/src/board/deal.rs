//! The board: tree rows dealt into lanes and the archive (the page's
//! `board`, `data/board.ts`), and the one clock a card's "6m ago" is
//! written from (`data/clock.ts`).

use std::time::Duration;

use super::{Fleet, Lane, Row};

/// How often the cards' "6m ago" is read again: one clock for every card,
/// running only while a card shows it (the page's `clock.ts`, milestone
/// 11's Decision Log). The labels count minutes, so a label is at most
/// this late.
pub const TICK: Duration = Duration::from_secs(15);

/// Each lane's cards, and the archive.
#[derive(Debug, Clone, Default)]
pub struct Board<'a> {
    building: Vec<Row<'a>>,
    validating: Vec<Row<'a>>,
    in_review: Vec<Row<'a>>,
    ready: Vec<Row<'a>>,
    /// The archived sessions (and, from an older machine, the ended
    /// ones), newest first.
    pub archive: Vec<Row<'a>>,
}

impl<'a> Board<'a> {
    /// `rows` dealt into lanes and the archive. Inside a lane the sessions
    /// that wait on their owner come first, then the most recently active,
    /// as on agent-orchestrator's board; the archive is newest first. The
    /// tree's order is not kept: a card names the session that started it
    /// instead (ADR 0013's attribution must not depend on two sessions
    /// sharing a lane).
    #[must_use]
    pub fn deal(rows: Vec<Row<'a>>) -> Self {
        let mut board = Self::default();
        for row in rows {
            match row.card.lane {
                Some(lane) => board.lane_mut(lane).push(row),
                None => board.archive.push(row),
            }
        }
        for lane in Lane::ALL {
            // Stable, as the page's two sorts: needs-you first, each part
            // newest first, equal times in the tree's order.
            board.lane_mut(lane).sort_by(|one, two| {
                two.card
                    .needs_you()
                    .cmp(&one.card.needs_you())
                    .then(two.card.at.cmp(&one.card.at))
            });
        }
        board
            .archive
            .sort_by_key(|row| std::cmp::Reverse(row.card.at));
        board
    }

    /// One lane's cards, in order.
    #[must_use]
    pub fn lane(&self, lane: Lane) -> &[Row<'a>] {
        match lane {
            Lane::Building => &self.building,
            Lane::Validating => &self.validating,
            Lane::InReview => &self.in_review,
            Lane::Ready => &self.ready,
        }
    }

    /// How many cards the lanes hold.
    #[must_use]
    pub fn on_board(&self) -> usize {
        Lane::ALL.iter().map(|lane| self.lane(*lane).len()).sum()
    }

    /// Whether it has no session at all (the board's empty state).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.on_board() == 0 && self.archive.is_empty()
    }

    const fn lane_mut(&mut self, lane: Lane) -> &mut Vec<Row<'a>> {
        match lane {
            Lane::Building => &mut self.building,
            Lane::Validating => &mut self.validating,
            Lane::InReview => &mut self.in_review,
            Lane::Ready => &mut self.ready,
        }
    }
}

impl Fleet {
    /// The board of the machine `scope` names, or of every machine.
    #[must_use]
    pub fn board(&self, scope: Option<&str>) -> Board<'_> {
        Board::deal(self.tree(scope))
    }
}
