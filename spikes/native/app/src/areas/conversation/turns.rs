//! Turn disclosure filters mounted rows while the complete followed log stays intact.

use std::collections::HashSet;

use vessel_app_kit::ui::Context;
use vessel_app_view::timeline::{Rows, TurnFolds};

use super::{ConversationArea, Seat};
use crate::shell::Shell;

#[derive(Default)]
pub(super) struct Turns {
    pub(super) folds: TurnFolds,
    pub(super) open: HashSet<usize>,
}

/// Existing capture override gains the third display without exposing a Settings
/// row before its shared translated getter and typed choice have landed.
pub(super) fn pinned() -> bool {
    std::env::var("VESSEL_APP_CONVERSATION").is_ok_and(|value| value == "turns")
}

impl ConversationArea {
    /// Find and record jumps reveal the containing turn before looking up its row.
    /// The entry identity stays the same; this never asks for a different log window.
    pub(crate) fn reveal_turn(&mut self, entry: usize) {
        let Some(turns) = &mut self.turns else {
            return;
        };
        let Some(host) = turns.folds.containing(entry).map(|turn| turn.host) else {
            return;
        };
        if turns.open.insert(host) {
            self.turn_rows_changed(host);
        }
    }

    fn toggle_turn(&mut self, host: usize) {
        let Some(turns) = &mut self.turns else {
            return;
        };
        if turns.folds.at_host(host).is_none() {
            return;
        }
        if !turns.open.remove(&host) {
            let _new = turns.open.insert(host);
        }
        self.turn_rows_changed(host);
    }

    fn turn_rows_changed(&mut self, host: usize) {
        let before = std::mem::replace(&mut self.rows, Rows::All(0));
        self.rows_again();
        // Include the host: its disclosure and measured summary changed too.
        let from = before.row_index(host, None, &self.logs).unwrap_or(0);
        self.list.splice(
            self.at(from)..self.at(before.len()),
            self.rows.len().saturating_sub(from),
        );
        self.tail_dropped();
    }
}

impl Shell {
    pub(crate) fn toggle_turn(&mut self, seat: Seat, host: usize, cx: &mut Context<'_, Self>) {
        let before = self.areas.conversation.head();
        if let Some(area) = (seat.get_mut)(self) {
            area.toggle_turn(host);
            self.origin_toggled(seat, before);
            cx.notify();
        }
    }
}
