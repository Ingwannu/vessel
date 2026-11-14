//! The end of an agent's turn, as Paseo draws it
//! (`agent-stream/turn-footer.tsx`, `components/message.tsx`
//! `AssistantTurnFooter`, `assistant-turn-footer-label.ts`): once the turn
//! is done, a copy button for its reply and "Worked for 14s", which
//! shows when the turn ended while pointed at; while it runs, a spinner and
//! the time it has run.
//!
//! [Decision Log]
//! - Purpose: a turn ended with a rule saying "Turn ended · end turn ·
//!   30 tokens · 20 in · 10 out" (the owner's report on 0.0.0.0.13: the
//!   conversation was half lifecycle and accounting lines); Paseo ends a
//!   turn with its footer and nothing else.
//! - The reference: Paseo's completed footer is the copy button, the fork
//!   menu and the label "Worked for {duration}" (`formatDuration`: 14s,
//!   2m 5s, 1h 3m) in the muted ink at the stream's metadata size, the
//!   label swapping to the turn's end time while pointed at, its width
//!   kept to the longer of the two; the running footer is a 14-point
//!   spinner and the live elapsed time.
//! - Choice: the same. A turn that did not end normally (cancelled,
//!   refused, out of tokens or turns) adds the reason after the duration:
//!   it is something the person must know (Paseo shows its interruptions
//!   in the stream as an error). The turn's tokens leave the timeline: the
//!   context meter's card keeps the session's figures.
//! - The fork menu is `fork`'s, here between the copy button and the
//!   label, where the turn's end can be forked.

use chrono::{DateTime, Local, Utc};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{CopyLook, Type, TypeExt as _, breakpoint, copy_button};
use vessel_app_view::timeline::{Entry, EntryKind};
use vessel_proto::StopReason;

use super::entries::{Paint, id};
use super::look::BUBBLE_COLUMN;
use crate::shell::Shell;

/// Paseo's `STREAM_METADATA_FONT_SIZE`.
const METADATA: f32 = 12.0;

/// The footer of the turn that ended at entry `index` (its `turn_ended`).
pub(super) fn completed(
    paint: &Paint<'_>,
    index: usize,
    reason: StopReason,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let entries = paint.entries;
    let ended = entries.get(index).map(|entry| entry.at);
    let start = turn_start(entries, index);
    let words = paint.words;
    let worked = match (start, ended) {
        (Some(start), Some(ended)) => Some(words.worked_for(&duration_label(start, ended))),
        _ => None,
    };
    let worked = match (worked, reason) {
        (Some(worked), StopReason::EndTurn) => Some(worked),
        (Some(worked), other) => Some(format!("{worked} · {}", words.stop_reason(other))),
        (None, StopReason::EndTurn) => None,
        (None, other) => Some(words.stop_reason(other).to_owned()),
    };
    let when = ended.map(|ended| words.when(ended, Local::now().fixed_offset()));
    let reply = reply_text(entries, index);
    let muted = paint.tokens.neutral.foreground_muted;
    let group = format!("turn-footer-{index}");
    let label = worked.map(|worked| {
        div()
            .relative()
            .group(group.clone())
            .text_size(px(METADATA))
            .text_color(muted)
            .child(
                div()
                    .when(when.is_some(), |shown| {
                        shown.group_hover(group.clone(), Styled::invisible)
                    })
                    .child(worked),
            )
            .children(when.map(|when| {
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .whitespace_nowrap()
                    .invisible()
                    .group_hover(group.clone(), Styled::visible)
                    .child(when)
            }))
    });
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .min_h(px(24.0))
        .ml(px(BUBBLE_COLUMN))
        .children((!reply.is_empty()).then(|| {
            copy_button(
                ElementId::NamedInteger("turn-copy".into(), id(index)),
                reply.into(),
                CopyLook {
                    glyph: muted,
                    hover: paint.tokens.neutral.surface2,
                },
                window,
                cx,
            )
        }))
        .children(super::fork::mark(
            paint,
            index,
            breakpoint::phone(window),
            cx,
        ))
        .children(label)
        .into_any_element()
}

/// The footer while a turn runs: Paseo's spinner and, once the turn's
/// prompt is in, how long it has run. Its first child is the empty
/// avatar column: where the next reply's circle will be drawn, the place
/// worker B's star travels to while the reply is awaited (its bounds go
/// to the brand once B's slot lands).
pub(super) fn working(paint: &Paint<'_>, since: Option<DateTime<Utc>>) -> AnyElement {
    let muted = paint.tokens.neutral.foreground_muted;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(super::look::AVATAR_GAP))
        .min_h(px(24.0))
        .child(div().flex_none().size(px(super::look::AVATAR)))
        .child(vessel_app_kit::slider::spinner(
            "turn-working",
            14.0,
            paint.tokens.neutral.foreground,
        ))
        .children(since.map(|since| {
            div()
                .face(Type::Meta)
                .text_size(px(METADATA))
                .text_color(muted)
                .child(duration_label(since, Utc::now()))
        }))
        .into_any_element()
}

/// When the turn ending at `index` began: the person's prompt (or steer)
/// before it.
pub(super) fn turn_start(entries: &[Entry], index: usize) -> Option<DateTime<Utc>> {
    entries
        .get(..index)?
        .iter()
        .rev()
        .find(|entry| matches!(entry.kind, EntryKind::Prompt(_)))
        .map(|entry| entry.at)
}

/// The agent's reply in the turn ending at `index`, its messages joined
/// (what Paseo's copy button copies).
fn reply_text(entries: &[Entry], index: usize) -> String {
    let Some(before) = entries.get(..index) else {
        return String::new();
    };
    let start = before
        .iter()
        .rposition(|entry| matches!(entry.kind, EntryKind::Prompt(_)))
        .map_or(0, |at| at.saturating_add(1));
    before
        .get(start..)
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            if let EntryKind::Message { text } = &entry.kind {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Paseo's `formatDuration`: seconds under a minute, minutes and seconds
/// under an hour, then hours and minutes.
fn duration_label(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let seconds = end.signed_duration_since(start).num_seconds().max(0);
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds.div_euclid(60);
    if minutes < 60 {
        let rest = seconds.rem_euclid(60);
        return if rest == 0 {
            format!("{minutes}m")
        } else {
            format!("{minutes}m {rest}s")
        };
    }
    let hours = minutes.div_euclid(60);
    let rest = minutes.rem_euclid(60);
    if rest == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {rest}m")
    }
}
