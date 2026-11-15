//! A fork's origin above its own conversation (slice F1b): the origin's
//! turns up to the fork point, read once the fork's `forked` line is
//! folded, drawn as the first rows of the session's list, read-only, with
//! the "Forked from" line under them as their divider.
//!
//! [Decision Log]
//! - Purpose: a fork that opened empty hid what its agent remembers (the
//!   coordinator on F1). The origin's conversation shows above the
//!   fork's first line, as the agent has it.
//! - The reference: Paseo shows no earlier turns; its fork's draft
//!   carries the conversation as one composer attachment, "Chat history"
//!   / "Previous conversation". Ours are the origin's real records, so
//!   they are drawn as turns are, less every mark that acts (rewind,
//!   fork); copy stays.
//! - Constraints: the node has `session.read` (pages of a log); no call
//!   reads a log up to a point, so the link reads pages from the start
//!   and stops at the fork's record (`view::timeline::Until`). The list's
//!   rows are the fork's own entries; its finds, the outline rail,
//!   reveals and toggles count them.
//! - Choice: the origin is a `ConversationArea` of its own (its own fold,
//!   rows, open runs and disclosures, reached through [`Seat::ORIGIN`]),
//!   read with one act (`Act::Log`) off the screen's thread, then drawn
//!   as the list's first rows: the session area's list ops add its row
//!   count ([`ConversationArea::at`]); the fork's own rows, finds and
//!   rail are unchanged. A removed origin leaves the line alone with a
//!   plain sentence under it.
//! - Cost: the origin shows once wholly read (one answer); find and the
//!   outline rail cover the fork's own turns only; a toggle in the
//!   origin measures its rows again.

use vessel_app_kit::skeleton;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};
use vessel_app_link::{Act, ActAnswer, ActFailed, ActKind, Acted, Command};
use vessel_app_view::fork::{Notice, PreparedNotice};
use vessel_app_view::timeline::{Folds, Logs, Until, empty_fork, origin_of};
use vessel_proto::{ErrorCode, EventRecord, SessionId};

use super::entries::Paint;
use super::{ConversationArea, Seat};
use crate::shell::Shell;

/// Where a fork's origin stands.
#[derive(Default)]
pub(crate) enum Origin {
    /// Not a fork (or its `forked` line is not read yet).
    #[default]
    None,
    /// A fork: its origin `from`, to read up to `until`.
    Wanted { from: SessionId, until: Until },
    /// Its origin's log is being read, since `asked` (the debug log
    /// says how long a read and its fold took).
    Reading {
        from: SessionId,
        until: Until,
        asked: std::time::Instant,
    },
    /// Its origin's conversation, read.
    Shown(Box<ConversationArea>),
    /// Its origin could not be read: `removed` when the node no longer
    /// has it, else why; the read is asked again when the session is
    /// opened again (`from`, `until` kept for it).
    Unread {
        removed: bool,
        notice: PreparedNotice,
        from: SessionId,
        until: Until,
    },
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Wanted { from, .. } | Self::Reading { from, .. } => {
                write!(f, "Reading({from})")
            }
            Self::Shown(area) => write!(f, "Shown({} rows)", area.rows.len()),
            Self::Unread { removed, .. } => write!(f, "Unread(removed: {removed})"),
        }
    }
}

impl Origin {
    /// Its conversation, when read.
    pub(crate) fn area(&self) -> Option<&ConversationArea> {
        if let Self::Shown(area) = self {
            Some(area)
        } else {
            None
        }
    }

    /// Its conversation, to change (a toggle in its rows).
    pub(crate) fn area_mut(&mut self) -> Option<&mut ConversationArea> {
        if let Self::Shown(area) = self {
            Some(area)
        } else {
            None
        }
    }
}

/// The loading row's width while the origin is read (a line of words).
const LOADING_WIDTH: f32 = 240.0;

/// What the fork's first line has above or under it.
#[derive(Debug, Clone, Copy)]
pub(super) enum Above<'a> {
    /// Nothing: not a fork, or an origin with no turns.
    Nothing,
    /// The origin's log is being read: a quiet loading row above the line
    /// (the kit's skeleton), so its turns do not appear without a cue.
    Reading,
    /// The origin's turns: the line is their divider.
    Turns,
    /// The origin could not be read.
    Unread(&'a PreparedNotice),
}

impl ConversationArea {
    /// How many of the list's first rows are the origin's.
    pub(super) fn head(&self) -> usize {
        self.origin.area().map_or(0, |origin| origin.rows.len())
    }

    /// The list's row for the fork's own row `row` (after the origin's).
    pub(super) fn at(&self, row: usize) -> usize {
        self.head().saturating_add(row)
    }

    /// What the fork's first line has above or under it.
    pub(super) fn above(&self) -> Above<'_> {
        match &self.origin {
            Origin::Shown(origin) if !origin.rows.is_empty() => Above::Turns,
            Origin::Unread { notice, .. } => Above::Unread(notice),
            Origin::Wanted { .. } | Origin::Reading { .. } => Above::Reading,
            Origin::None | Origin::Shown(_) => Above::Nothing,
        }
    }

    /// Whether the session is a fork that never held a conversation, as
    /// its records last said (F2).
    pub(super) fn empty_fork(&self) -> bool {
        self.timeline
            .status()
            .is_some_and(|status| empty_fork(status).is_some())
    }

    /// Entries from `from` came in: a `forked` line among them makes the
    /// origin wanted (once), unless the fork never held a conversation.
    pub(super) fn origin_seen(&mut self, from: usize) {
        if !matches!(self.origin, Origin::None) || self.empty_fork() {
            return;
        }
        let entries = self.timeline.entries();
        if let Some((from, until)) = entries.get(from..).and_then(origin_of) {
            self.origin = Origin::Wanted { from, until };
        }
    }

    /// A fork found to have never held a conversation shows no origin:
    /// what was read of it leaves the list.
    pub(super) fn origin_dropped(&mut self) {
        if matches!(self.origin, Origin::None) || !self.empty_fork() {
            return;
        }
        let head = self.head();
        self.origin = Origin::None;
        self.list.splice(0..head, 0);
    }

    /// The session shown was opened again: an origin that could not be
    /// read (not removed) is asked for again.
    pub(super) fn origin_again(&mut self) {
        if let Origin::Unread {
            removed: false,
            from,
            until,
            ..
        } = self.origin
        {
            self.origin = Origin::Wanted { from, until };
        }
    }

    /// The origin's `records` folded as its own conversation, collapsed
    /// as the fork's is.
    fn inherit(&mut self, records: &[EventRecord], collapse: bool) {
        let _change = self.timeline.apply(records);
        self.collapse = collapse;
        self.folds = collapse.then(|| Folds::of(self.timeline.entries()));
        self.logs = Logs::of(self.timeline.entries());
        self.rows_again();
    }
}

impl Shell {
    /// Only reword the retained result; reading/retrying and row state stay put.
    pub(super) fn translate_origin_notice(&mut self) {
        if let Origin::Unread { notice, .. } = &mut self.areas.conversation.origin {
            notice.translate(self.words);
        }
    }

    /// Read the shown fork's origin, when it is wanted.
    pub(super) fn origin_read(&mut self) {
        let area = &mut self.areas.conversation;
        let (Origin::Wanted { from, until }, Some(shown)) = (&area.origin, &area.shown) else {
            return;
        };
        let (from, until, machine) = (*from, *until, shown.machine.clone());
        area.origin = Origin::Reading {
            from,
            until,
            asked: std::time::Instant::now(),
        };
        self.command(Command::Act {
            machine,
            session: from,
            act: Act::Log(until),
        });
    }

    /// The origin's log came (or did not): its turns become the list's
    /// first rows, or the line says why they are missing.
    pub(super) fn origin_answered(&mut self, answer: &ActAnswer, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        let asked = match area.origin {
            Origin::Reading { from, asked, .. } if from == answer.session => Some(asked),
            Origin::None
            | Origin::Wanted { .. }
            | Origin::Reading { .. }
            | Origin::Shown(_)
            | Origin::Unread { .. } => None,
        };
        let on_machine = area
            .shown
            .as_ref()
            .is_some_and(|shown| shown.machine == answer.machine);
        let Some(asked) = asked.filter(|_| answer.kind == ActKind::Log && on_machine) else {
            return;
        };
        // The fork failed while its origin was read: nothing to show.
        if area.empty_fork() {
            area.origin = Origin::None;
            return;
        }
        match &answer.result {
            Ok(Acted::Log(records)) => {
                let read = asked.elapsed();
                let folding = std::time::Instant::now();
                let mut origin = ConversationArea::new(cx);
                // The same agent: the fork's spec is its origin's.
                origin.info.clone_from(&area.info);
                origin.inherit(records, area.collapse);
                let rows = origin.rows.len();
                tracing::debug!(
                    records = records.len(),
                    rows,
                    read_ms = read.as_millis(),
                    fold_us = folding.elapsed().as_micros(),
                    "a fork's origin: read and folded"
                );
                area.origin = Origin::Shown(Box::new(origin));
                area.list.splice(0..0, rows);
            }
            Err(ActFailed { code, message }) => {
                if let Origin::Reading { from, until, .. } = area.origin {
                    tracing::warn!(?code, %message, "fork origin unreadable");
                    area.origin = Origin::Unread {
                        removed: *code == Some(ErrorCode::NotFound),
                        notice: PreparedNotice::new(Notice::Origin((*code).into()), self.words),
                        from,
                        until,
                    };
                }
            }
            Ok(
                Acted::Session(_)
                | Acted::Deleted(_)
                | Acted::Done
                | Acted::Conversation(_)
                | Acted::Forked(_),
            ) => {}
        }
        // The first line says it now.
        let head = area.head();
        area.list.changed(head..head.saturating_add(1));
        cx.notify();
    }

    /// A toggle at `seat` happened while the origin had `before` rows:
    /// when it was in the origin's rows, the list measures them again.
    pub(super) fn origin_toggled(&mut self, seat: Seat, before: usize) {
        let area = &self.areas.conversation;
        let in_origin = area
            .origin
            .area()
            .zip((seat.get)(self))
            .is_some_and(|(origin, toggled)| std::ptr::eq(origin, toggled));
        if in_origin {
            let after = area.head();
            area.list.splice(0..before, after);
        }
    }

    /// The conversation's display changed: the origin folds as the fork.
    pub(super) fn origin_display(&mut self, collapse: bool) {
        if let Some(origin) = self.areas.conversation.origin.area_mut() {
            origin.collapse = collapse;
            origin.folds = collapse.then(|| Folds::of(origin.timeline.entries()));
            origin.open_runs.clear();
            origin.rows_again();
        }
    }
}

/// The fork's first line (`words`: "Forked from “plain” · today 18:55"):
/// a divider between the origin's turns and the fork's own when they
/// are above it, else the line alone, with the reason under it when the
/// origin could not be read.
pub(super) fn forked_line(paint: &Paint<'_>, words: String) -> AnyElement {
    let look = paint.look;
    let line = div()
        .flex_none()
        .px(px(spacing::S1))
        .face(Type::Meta)
        .text_size(px(vessel_app_kit::fonts::ui(super::look::WORK_SIZE)))
        .line_height(px(vessel_app_kit::fonts::ui(super::look::WORK_LINE)))
        .text_color(look.secondary)
        .child(words);
    match paint.above {
        Above::Turns => {
            let rule = || div().flex_1().h_px().bg(paint.tokens.neutral.border);
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .py(px(spacing::S2))
                .child(rule())
                .child(line)
                .child(rule())
                .into_any_element()
        }
        Above::Unread(notice) => div()
            .flex()
            .flex_col()
            .child(line)
            .child(
                div()
                    .px(px(spacing::S1))
                    .face(Type::Meta)
                    .text_size(px(vessel_app_kit::fonts::ui(super::look::WORK_SIZE)))
                    .text_color(look.secondary)
                    .child(vessel_app_kit::ui::SharedString::from(notice.words())),
            )
            .into_any_element(),
        Above::Reading => {
            let tokens = *paint.tokens;
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .child(skeleton::pulse("origin-reading", move |opacity| {
                    div().py(px(spacing::S1)).child(skeleton::shape(
                        skeleton::Shape {
                            size: gpui::size(px(LOADING_WIDTH), px(spacing::S3)),
                            rounding: skeleton::Rounding::Medium,
                        },
                        opacity,
                        &tokens,
                    ))
                }))
                .child(line)
                .into_any_element()
        }
        Above::Nothing => line.into_any_element(),
    }
}

#[cfg(test)]
mod language_tests;
