//! One entry of the timeline, drawn as GOAT's chat draws its kind: a
//! person's prompt as a bubble on the right, the agent's reply as text in
//! the lane, thoughts and tool calls as the work log's quiet rows, what
//! happened as a meta line or a divider (GOAT's `TimelineUserMessageRow`,
//! `TimelineAssistantRows`, `TimelineWorkEntryRow`,
//! `TimelineSystemDivider`). The words are the page's (ADR 0036).

use chrono::Local;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, HighlightStyle,
    InteractiveElement as _, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, StyledText, Window, div, px, relative,
};
use vessel_app_kit::{FindMarks, FindPalette, Tokens, Type, TypeExt as _, chat_markdown};
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{Entry, EntryKind, Prompt, answered_lines, settled, spans_of};
use vessel_proto::{Question, QuestionOutcome};

use super::Seat;
use super::find::Marking;
use super::look::{
    AVATAR, AVATAR_GAP, BUBBLE_RADIUS, BUBBLE_SHARE, CORNER, Look, WORK_LINE, WORK_SIZE,
};
use super::tool;
use crate::shell::Shell;

/// What every row needs to draw itself.
pub(super) struct Paint<'a> {
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
}

/// Entry `index` as its row.
pub(super) fn entry(
    paint: &Paint<'_>,
    index: usize,
    entry: &Entry,
    open: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = paint.words;
    match &entry.kind {
        EntryKind::Prompt(prompt) => person(paint, index, prompt, None),
        EntryKind::Steer { text, attachments } => person(
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
        EntryKind::Message { text } => message(paint, index, text, entry.rev, window, cx),
        EntryKind::Thought { text } => {
            let head = work_row(paint, index, words.thought().to_owned(), None, open, cx);
            div()
                .child(head)
                .when(open, |row| {
                    row.child(
                        div()
                            .ml(px(28.0))
                            .py(px(4.0))
                            .type_(Type::Body)
                            .text_color(paint.look.secondary)
                            .child(text.clone()),
                    )
                })
                .into_any_element()
        }
        EntryKind::Tool(view) => tool::card(paint, index, view, open, window, cx),
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
        EntryKind::TurnEnded { reason, .. } => {
            super::footer::completed(paint, index, *reason, window, cx)
        }
        EntryKind::TurnFailed { .. } => divider(paint, words.spoken(entry), true),
        EntryKind::Log { line } => div()
            .px(px(4.0))
            .type_(Type::Mono)
            .text_size(px(vessel_app_kit::fonts::ui(12.0)))
            .text_color(paint.look.secondary)
            .child(line.clone())
            .into_any_element(),
        EntryKind::Status { .. } | EntryKind::Parking(_) => meta(
            paint,
            format!(
                "{} · {}",
                words.spoken(entry),
                words.when(entry.at, Local::now().fixed_offset())
            ),
        ),
        EntryKind::Ready { .. } | EntryKind::Profile { .. } => meta(paint, words.spoken(entry)),
        EntryKind::Model(call) => meta(paint, words.model_line(call)),
    }
}

/// An element id from an entry's index.
pub(super) fn id(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

/// A person's prompt or steer: GOAT's bubble on the right, 70% of the
/// lane at most, on the message surface; its corner word above it.
/// The person's message the moment it is sent, before the node records
/// it (Paseo's pending `UserMessage`): drawn as the message it becomes,
/// at `index`, the entry it will be.
pub(super) fn pending(paint: &Paint<'_>, index: usize, text: &str) -> AnyElement {
    person(
        paint,
        index,
        &Prompt {
            text: text.to_owned(),
            replayed: false,
            turn: 0,
            attachments: Vec::new(),
        },
        None,
    )
}

fn person(paint: &Paint<'_>, index: usize, prompt: &Prompt, corner: Option<String>) -> AnyElement {
    let corner = corner.or_else(|| prompt.replayed.then(|| paint.words.prompt_corner(true, 0)));
    let files = (!prompt.attachments.is_empty()).then(|| {
        div()
            .flex()
            .flex_wrap()
            .gap(px(4.0))
            .children(prompt.attachments.iter().map(|file| {
                div()
                    .px(px(8.0))
                    .py(px(2.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(paint.look.border)
                    .type_(Type::Meta)
                    .text_color(paint.look.secondary)
                    .child(file.name.to_string())
            }))
    });
    div()
        .flex()
        .flex_col()
        .items_end()
        .gap(px(4.0))
        .children(corner.map(|word| {
            div()
                .mr(px(4.0))
                .type_(Type::Meta)
                .text_color(paint.look.secondary)
                .child(word)
        }))
        .child(
            // Paseo's `UserMessage` bubble: 16 inside, radius 16 but the
            // corner toward its writer at 4, `surface3`, as wide as the
            // column needs.
            div()
                .max_w(relative(BUBBLE_SHARE))
                .min_w(px(0.0))
                .p(px(16.0))
                .rounded(px(BUBBLE_RADIUS))
                .rounded_tr(px(CORNER))
                .bg(paint.look.message)
                .flex()
                .flex_col()
                .gap(px(6.0))
                .children(files)
                .when(!prompt.text.is_empty(), |bubble| {
                    bubble.child(
                        div()
                            .type_(Type::Body)
                            .text_color(paint.tokens.neutral.foreground)
                            .child(marked_text(paint, index, &prompt.text)),
                    )
                }),
        )
        .into_any_element()
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
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.toggle_entry(seat, index, cx);
        }))
        .child(disclosure(open, paint.look.secondary))
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

/// A divider with its words in the middle (GOAT's system divider): a
/// turn's end; in danger when it failed.
fn divider(paint: &Paint<'_>, text: String, failed: bool) -> AnyElement {
    let (line, ink) = if failed {
        (
            vessel_app_kit::ui::Hsla {
                a: 0.3,
                ..paint.tokens.neutral.destructive
            },
            paint.tokens.neutral.destructive,
        )
    } else {
        (paint.look.border, paint.look.secondary)
    };
    let rule = || div().h(px(1.0)).flex_1().bg(line);
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .py(px(4.0))
        .child(rule())
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(vessel_app_kit::fonts::ui(12.0)))
                .text_color(ink)
                .child(text),
        )
        .child(rule())
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
fn marked_text(paint: &Paint<'_>, index: usize, text: &str) -> StyledText {
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

/// The agent's reply: its Markdown on the lane, find's matches marked.
fn message(
    paint: &Paint<'_>,
    index: usize,
    text: &str,
    rev: u32,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let spans = |rendered: &str| {
        paint
            .find
            .map_or_else(Vec::new, |find| spans_of(rendered, find.query))
    };
    let marks = paint.find.map(|find| FindMarks {
        list: find.list,
        clear_top: find.clear_top,
        key: find.key(index),
        spans: &spans,
        current: find.current_in(index),
    });
    // The owner's reply bubble (slice M; Paseo's reply has none): the
    // person's bubble mirrored, its corner toward the agent at 4, on its
    // own surface with a hairline, as wide as the column so a code block
    // or a table scrolls inside it; the agent's mark in a circle beside a
    // turn's first reply, its place kept beside the rest so they line up.
    let bubble = div()
        .flex_1()
        .min_w(px(0.0))
        .p(px(16.0))
        .rounded(px(BUBBLE_RADIUS))
        .rounded_tl(px(CORNER))
        .bg(paint.look.reply)
        .border_1()
        .border_color(paint.look.border)
        .type_(Type::Body)
        .text_color(paint.tokens.neutral.foreground)
        .child(chat_markdown(
            ElementId::NamedInteger("message".into(), id(index)),
            text,
            u64::from(rev),
            marks.as_ref(),
            paint.tokens,
            window,
            cx,
        ));
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(AVATAR_GAP))
        .child(
            div()
                .flex_none()
                .w(px(AVATAR))
                .children(first_reply(paint.entries, index).then(|| avatar(paint))),
        )
        .child(bubble)
        .into_any_element()
}

/// The circle beside a turn's first reply with the session's agent's mark
/// in it (the owner's, slice M). Worker B's star-to-circle morph is to be
/// mounted here for the first reply after a session starts.
fn avatar(paint: &Paint<'_>) -> Div {
    div()
        .size(px(AVATAR))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(paint.tokens.neutral.surface2)
        .border_1()
        .border_color(paint.look.border)
        .child(crate::agent_mark::mark(
            paint.agent,
            vessel_app_kit::agent_mark::Size::Sixteen,
            vessel_app_kit::agent_mark::Space::Compact,
            paint.tokens,
        ))
}

/// Whether the message at `index` is the first reply of its turn: no
/// reply since the person's last message.
fn first_reply(entries: &[Entry], index: usize) -> bool {
    entries.get(..index).is_none_or(|before| {
        before
            .iter()
            .rev()
            .find(|entry| {
                matches!(
                    entry.kind,
                    EntryKind::Message { .. } | EntryKind::Prompt(_) | EntryKind::Steer { .. }
                )
            })
            .is_none_or(|entry| !matches!(entry.kind, EntryKind::Message { .. }))
    })
}
