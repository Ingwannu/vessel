//! One entry of the timeline, drawn by its kind: a person's prompt and
//! the agent's reply as bubbles (`bubbles`, slice M), thoughts and tool
//! calls as the work log's quiet rows, what happened as a meta line or a
//! divider (GOAT's `TimelineUserMessageRow`,
//! `TimelineAssistantRows`, `TimelineWorkEntryRow`,
//! `TimelineSystemDivider`). The words are the page's (ADR 0036).

use chrono::Local;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, HighlightStyle,
    InteractiveElement as _, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{FindPalette, Tokens, Type, TypeExt as _};
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{
    Entry, EntryKind, Prompt, TextPhase, answered_lines, repeats_failure, settled, spans_of,
};
use vessel_proto::{Question, QuestionOutcome};

use super::Seat;
use super::find::Marking;
use super::look::{Look, WORK_LINE, WORK_SIZE};
use super::{bubbles, tool};
use crate::shell::Shell;

/// What every row needs to draw itself.
pub(super) struct Paint<'a> {
    /// Prepared catalogs for the areas already moved out of legacy tables.
    pub(super) catalog: &'static vessel_app_view::localization::Translator,
    pub(super) words: &'static ConversationWords,
    pub(super) tokens: &'a Tokens,
    pub(super) look: Look,
    /// What find marks, while its bar shows matches.
    pub(super) find: Option<Marking<'a>>,
    /// Where the conversation drawn is kept: its rows' toggles change it.
    pub(super) seat: Seat,
    /// The conversation's entries (a turn's footer reads its turn).
    pub(super) entries: &'a [Entry],
    /// The session's agent: its mark in a reply's avatar.
    pub(super) agent: Option<&'a vessel_proto::HarnessRef>,
    /// The conversation's timeline, when a rewind may be asked now (the
    /// session area's live conversation, between turns): a person's
    /// message offers it where it can go (`rewind`).
    pub(super) rewinds: Option<&'a vessel_app_view::timeline::Timeline>,
    /// The conversation's timeline and its fork state, when the
    /// conversation is the session area's own (not a side question's): a
    /// turn's footer offers a fork where it can go (`fork`).
    pub(super) forks: Option<(
        &'a vessel_app_view::timeline::Timeline,
        &'a super::fork::Forking,
    )>,
    /// The machines' sessions: a forked session's first line names its
    /// source by its title.
    pub(super) fork_headers: &'a vessel_app_view::fork::Headers,
    /// What a fork's first line has above or under it (`origin`).
    pub(super) above: super::origin::Above<'a>,
}

/// Entry `index` as its row.
pub(super) fn entry(
    paint: &Paint<'_>,
    index: usize,
    entry: &Entry,
    open: bool,
    phase: TextPhase,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = paint.words;
    match &entry.kind {
        EntryKind::Prompt(prompt) => bubbles::person(paint, index, prompt, None),
        EntryKind::Steer { text, attachments } => bubbles::person(
            paint,
            index,
            &Prompt {
                text: text.clone(),
                replayed: false,
                turn: 0,
                attachments: attachments.clone(),
            },
            Some(words.steered().to_owned()),
        ),
        // Claude's reply that only repeats the failure ending its turn
        // (Paseo holds it back): the failure says it once.
        EntryKind::Message { .. } if repeats_failure(paint.entries, index) => {
            div().into_any_element()
        }
        EntryKind::Message { text } => {
            let text = paced(paint, index, entry, text, phase, window, cx);
            bubbles::message(paint, index, text, entry.rev, window, cx)
        }
        EntryKind::Thought { text } => {
            let text = if open {
                paced(paint, index, entry, text, phase, window, cx)
            } else {
                text
            };
            thought(paint, index, text, open, window, cx)
        }
        EntryKind::Tool(view) => tool::card(paint, index, view, open, window, cx),
        EntryKind::Notification(notice) => {
            super::notices::notification(paint, index, notice, window, cx)
        }
        EntryKind::Compaction(item) => super::notices::compaction(paint, index, item),
        EntryKind::Plugin(item) => super::notices::plugin_unavailable(paint, item),
        // While it waits its form is over the composer: here only a quiet
        // line (the page's), so the question is not drawn twice.
        EntryKind::Question { outcome: None, .. } => {
            meta(paint, words.question_waiting().to_owned())
        }
        EntryKind::Question {
            question,
            outcome: Some(outcome),
        } => question_card(paint, index, question, outcome),
        EntryKind::Plan { entries, activity } => {
            super::plan::row(paint, index, (entries, activity), open, cx)
        }
        // While it waits its card is over the composer, with the command
        // in its box: here only a quiet line, so the command is not read
        // twice more.
        EntryKind::Permission(permission) if permission.outcome.is_none() => {
            meta(paint, words.permission_waiting().to_owned())
        }
        EntryKind::Permission(permission) => meta(
            paint,
            words.settled_line(
                permission
                    .title
                    .as_deref()
                    .unwrap_or(words.unseen_tool_call()),
                &settled(&permission.request.options, permission.outcome.as_ref()),
            ),
        ),
        // Paseo's turn footer: the reply's copy button and how long the
        // turn worked; the turn's tokens are the context meter's.
        EntryKind::TurnEnded { .. } => super::footer::completed(paint, index, window, cx),
        EntryKind::TurnFailed { .. } => div()
            .child(failure(paint, words.spoken(entry, paint.catalog)))
            .child(super::footer::completed(paint, index, window, cx))
            .into_any_element(),
        EntryKind::Log { line } => log_line(paint, line),
        EntryKind::Forked { from, .. } => {
            let head = paint.fork_headers.words(*from, paint.catalog);
            super::origin::forked_line(paint, dated(paint, head, entry))
        }
        EntryKind::Status { .. } | EntryKind::Parking(_) => meta(
            paint,
            dated(paint, &words.spoken(entry, paint.catalog), entry),
        ),
        EntryKind::Ready { .. } | EntryKind::Profile { .. } => {
            meta(paint, words.spoken(entry, paint.catalog))
        }
        EntryKind::Model(call) => meta(paint, words.model_line(call)),
    }
}

/// Only the selected find result bypasses display pacing.
fn paced<'a>(
    paint: &Paint<'_>,
    index: usize,
    entry: &Entry,
    text: &'a str,
    phase: TextPhase,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> &'a str {
    let phase = if paint
        .find
        .is_some_and(|find| find.current_in(index).is_some())
    {
        TextPhase::Complete
    } else {
        phase
    };
    let end = super::pacing::visible_end(
        ElementId::NamedInteger("text-reveal".into(), entry.seq.get()),
        text,
        entry.rev,
        phase,
        window,
        cx,
    );
    text.get(..end).unwrap_or(text)
}

/// A thought: its work row, and its words under it while open.
fn thought(
    paint: &Paint<'_>,
    index: usize,
    text: &str,
    open: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let head = marked_work_row(
        paint,
        index,
        (Some(Glyph::Brain), paint.words.thought().to_owned()),
        None,
        open,
        cx,
    );
    div()
        .child(head)
        .when(open, |row| {
            row.child(
                div()
                    .ml(px(28.0))
                    .py(px(4.0))
                    .child(super::tool_detail::prose(
                        paint,
                        ElementId::NamedInteger("reasoning-prose".into(), id(index)),
                        text,
                        window,
                        cx,
                    )),
            )
        })
        .into_any_element()
}

/// A line of the agent's log, in the code face.
fn log_line(paint: &Paint<'_>, line: &str) -> AnyElement {
    div()
        .px(px(4.0))
        .type_(Type::Mono)
        .text_size(px(vessel_app_kit::fonts::ui(12.0)))
        .text_color(paint.look.secondary)
        .child(line.to_owned())
        .into_any_element()
}

/// `head`, then when `entry` happened.
fn dated(paint: &Paint<'_>, head: &str, entry: &Entry) -> String {
    let when = paint.words.when(entry.at, Local::now().fixed_offset());
    format!("{head} · {when}")
}

/// An element id from an entry's index.
pub(super) fn id(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

/// A work log row: a disclosure's chevron, its label, a quiet detail
/// (GOAT's `WorkLogRow`: 12.5 points in the secondary label's colour).
pub(super) fn work_row(
    paint: &Paint<'_>,
    index: usize,
    label: String,
    detail: Option<(String, vessel_app_kit::ui::Hsla)>,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    marked_work_row(paint, index, (None, label), detail, open, cx)
}

/// Canonical semantic marks share the existing disclosure/hit surface.
pub(super) fn marked_work_row(
    paint: &Paint<'_>,
    index: usize,
    (mark, label): (Option<Glyph>, String),
    detail: Option<(String, vessel_app_kit::ui::Hsla)>,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let hover = paint.tokens.neutral.foreground;
    let seat = paint.seat;
    div()
        .id(ElementId::NamedInteger("work".into(), id(index)))
        .flex()
        .items_center()
        .gap(px(6.0))
        .min_h(px(WORK_LINE))
        .px(px(4.0))
        .rounded(px(6.0))
        .face(Type::Meta)
        .text_size(px(vessel_app_kit::fonts::ui(WORK_SIZE)))
        .line_height(px(vessel_app_kit::fonts::ui(WORK_LINE)))
        .text_color(paint.look.secondary)
        .cursor_pointer()
        .hover(move |row| row.text_color(hover))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            let inline_details = seat
                .get(shell)
                .is_some_and(|area| area.tool_sheet.group_open());
            if !inline_details
                && vessel_app_kit::breakpoint::phone(window)
                && shell.entry_sheet(seat, index, window, cx)
            {
                return;
            }
            shell.toggle_entry(seat, index, cx);
        }))
        .child(disclosure(open, paint.look.secondary))
        .children(mark.map(|mark| glyph(mark, 14.0, paint.look.secondary)))
        .child(div().min_w(px(0.0)).truncate().child(label))
        .children(detail.map(|(detail, color)| {
            div()
                .ml_auto()
                .flex_shrink_0()
                .text_color(color)
                .child(detail)
        }))
        .into_any_element()
}

/// A meta line: what happened, quietly.
fn meta(paint: &Paint<'_>, text: String) -> AnyElement {
    div()
        .px(px(4.0))
        .face(Type::Meta)
        .text_size(px(vessel_app_kit::fonts::ui(WORK_SIZE)))
        .line_height(px(vessel_app_kit::fonts::ui(WORK_LINE)))
        .text_color(paint.look.secondary)
        .child(text)
        .into_any_element()
}

/// A failed turn: its one sentence in Paseo's error alert (the kit's,
/// a circled cross), under the replies (the coordinator's finding on
/// 0.0.0.0.15; the sentence is `view::timeline::turn_failure`'s).
fn failure(paint: &Paint<'_>, sentence: String) -> AnyElement {
    div()
        .ml(px(super::look::BUBBLE_COLUMN))
        .child(vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(sentence.into()),
                description: None,
                action: None,
            },
            vessel_app_kit::alert::Variant::Error,
            vessel_app_kit::alert::Size::Small,
            paint.tokens,
        ))
        .into_any_element()
}

/// A settled question in the timeline: what the agent asked, each answer
/// by its field (the page's answered card, `question-entry.tsx`), and how
/// it was settled.
fn question_card(
    paint: &Paint<'_>,
    index: usize,
    question: &Question,
    outcome: &QuestionOutcome,
) -> AnyElement {
    let words = paint.words;
    let lines = answered_lines(question, outcome, words.question_yes_no());
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .px(px(12.0))
        .py(px(10.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(paint.look.border)
        .child(
            div()
                .type_(Type::Body)
                .text_color(paint.tokens.neutral.foreground)
                // What find searches in a question (`view::timeline::find`),
                // marked as a person's message is.
                .child(marked_text(paint, index, &question.message)),
        )
        .children(lines.into_iter().map(|line| {
            div()
                .flex()
                .gap(px(8.0))
                .type_(Type::Body)
                .child(
                    div()
                        .flex_none()
                        .text_color(paint.look.secondary)
                        .child(line.field),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .text_color(paint.tokens.neutral.foreground)
                        .child(line.value),
                )
        }))
        .child(
            div()
                .text_size(px(vessel_app_kit::fonts::ui(WORK_SIZE)))
                .text_color(paint.look.secondary)
                .child(words.question_line(outcome)),
        )
        .into_any_element()
}

/// An empty lane's words.
pub(super) fn empty(paint: &Paint<'_>) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .type_(Type::Body)
        .text_color(paint.look.secondary)
        .child(paint.words.send_to_start())
}

/// A disclosure's caret: down when open, right when closed (the kit's
/// glyphs; GOAT's chevron at 14).
pub(super) fn disclosure(open: bool, color: vessel_app_kit::ui::Hsla) -> Div {
    let which = if open {
        Glyph::ChevronDown
    } else {
        Glyph::ChevronRight
    };
    div()
        .w(px(16.0))
        .flex()
        .justify_center()
        .child(glyph(which, 14.0, color))
}

/// A plain text (a person's message) with find's matches in it marked:
/// every one on yellow, the current one on amber in dark ink (GOAT's
/// find highlights).
pub(super) fn marked_text(paint: &Paint<'_>, index: usize, text: &str) -> StyledText {
    let styled = StyledText::new(text.to_owned());
    let Some(find) = paint.find else {
        return styled;
    };
    let palette = FindPalette::of(paint.tokens);
    let current = find.current_in(index);
    let marks = spans_of(text, find.query)
        .into_iter()
        .enumerate()
        .map(|(nth, span)| {
            let style = if Some(nth) == current {
                HighlightStyle {
                    background_color: Some(palette.current),
                    color: Some(palette.current_ink),
                    ..HighlightStyle::default()
                }
            } else {
                HighlightStyle {
                    background_color: Some(palette.found),
                    ..HighlightStyle::default()
                }
            };
            (span, style)
        });
    styled.with_highlights(marks)
}
