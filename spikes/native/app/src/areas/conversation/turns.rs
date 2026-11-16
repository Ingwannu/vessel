//! Turn disclosure filters mounted rows while the complete followed log stays intact.

use std::collections::HashSet;

use super::entries::Paint;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, Styled as _, div, px,
};
use vessel_app_view::timeline::EntryKind;
use vessel_app_view::timeline::{Rows, TurnFolds};

use super::{ConversationArea, Seat};
use crate::shell::Shell;

#[derive(Default)]
pub(super) struct Turns {
    pub(super) folds: TurnFolds,
    pub(super) open: HashSet<usize>,
}

/// Settings and the existing capture override select the same consumed display.
pub(super) fn enabled(display: Option<vessel_app_view::prefs::ConversationDisplay>) -> bool {
    use vessel_app_view::prefs::ConversationDisplay;
    match std::env::var("VESSEL_APP_CONVERSATION").as_deref() {
        Ok("turns") => true,
        Ok("full" | "collapse") => false,
        _ => match display.unwrap_or_default() {
            ConversationDisplay::Full | ConversationDisplay::CollapseTools => false,
            ConversationDisplay::CollapseTurns => true,
        },
    }
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
        // Splicing log rows retains and shifts the already mounted tail rows.
        // Keep their identity; clearing only its bookkeeping would mount a second tail.
    }
}

impl Shell {
    pub(crate) fn toggle_turn(&mut self, seat: &Seat, host: usize, cx: &mut Context<'_, Self>) {
        let before = seat.origin_height(self);
        if let Some(area) = seat.get_mut(self) {
            area.toggle_turn(host);
            self.origin_toggled(seat, before);
            cx.notify();
        }
    }
}

/// One post-entry hook adds its hosted summary and the expanded indent rail.
pub(super) fn decorate(
    paint: &Paint<'_>,
    area: &ConversationArea,
    index: usize,
    mut content: Div,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    if let Some(turn) = area
        .turns
        .as_ref()
        .and_then(|turns| turns.folds.at_host(index))
    {
        let expanded = area
            .turns
            .as_ref()
            .is_some_and(|turns| turns.open.contains(&index));
        let summary = paint.words.run_summary(&turn.summary);
        let summary = if summary.is_empty() {
            paint.catalog.timeline_turns().details().to_owned()
        } else {
            summary
        };
        content = content.child(super::timeline::run_header(
            paint,
            (super::timeline::RunHeaderKind::Turn, turn.host),
            summary,
            expanded,
            Shell::toggle_turn,
            cx,
        ));
    }
    let retained = paint.entries.get(index).is_some_and(|entry| {
        matches!(
            entry.kind,
            EntryKind::Steer { .. } | EntryKind::Compaction(_)
        )
    });
    let nested = !retained
        && area.turns.as_ref().is_some_and(|turns| {
            turns
                .folds
                .containing(index)
                .is_some_and(|turn| turns.open.contains(&turn.host))
        });
    if nested {
        div()
            .ml(px(spacing::S2))
            .pl(px(spacing::S3))
            .border_l_1()
            .border_color(paint.tokens.neutral.border)
            .child(content)
            .into_any_element()
    } else {
        content.into_any_element()
    }
}

#[cfg(test)]
mod tests;
