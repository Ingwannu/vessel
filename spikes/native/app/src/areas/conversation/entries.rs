//! One entry of the timeline, drawn as GOAT's chat draws its kind: a
//! person's prompt as a bubble on the right, the agent's reply as text in
//! the lane, thoughts and tool calls as the work log's quiet rows, what
//! happened as a meta line or a divider (GOAT's `TimelineUserMessageRow`,
//! `TimelineAssistantRows`, `TimelineWorkEntryRow`,
//! `TimelineSystemDivider`). The words are the page's (ADR 0036).

use chrono::Local;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
    relative,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, markdown};
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{Entry, EntryKind, Prompt, plan_progress, settled};

use super::look::{BUBBLE_RADIUS, BUBBLE_SHARE, Look, WORK_LINE, WORK_SIZE};
use super::tool;
use crate::shell::Shell;

/// What every row needs to draw itself.
pub(super) struct Paint<'a> {
    pub(super) words: &'static ConversationWords,
    pub(super) tokens: &'a Tokens,
    pub(super) look: Look,
}

/// Entry `index` as its row.
pub(super) fn entry(
    paint: &Paint<'_>,
    index: usize,
    entry: &Entry,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = paint.words;
    match &entry.kind {
        EntryKind::Prompt(prompt) => person(paint, prompt, None),
        EntryKind::Steer { text, attachments } => person(
            paint,
            &Prompt {
                text: text.clone(),
                replayed: false,
                turn: 0,
                attachments: attachments.clone(),
            },
            Some(words.steered().to_owned()),
        ),
        EntryKind::Message { text } => div()
            .px(px(4.0))
            .py(px(2.0))
            .type_(Type::Body)
            .text_color(paint.tokens.neutral.foreground)
            .child(markdown(
                ElementId::NamedInteger("message".into(), id(index)),
                text.clone(),
            ))
            .into_any_element(),
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
        EntryKind::Tool(view) => tool::card(paint, index, view, open, cx),
        EntryKind::Question { question, outcome } => question_card(
            paint,
            &question.message,
            outcome.as_ref().map(|outcome| words.question_line(outcome)),
        ),
        EntryKind::Plan { entries } => {
            let (done, total) = plan_progress(entries);
            meta(paint, words.plan_line(done, total))
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
        EntryKind::TurnEnded { reason, usage } => {
            divider(paint, words.turn_ended(*reason, usage.as_ref()), false)
        }
        EntryKind::TurnFailed { .. } => divider(paint, words.spoken(entry), true),
        EntryKind::Log { line } => div()
            .px(px(4.0))
            .type_(Type::Mono)
            .text_size(px(12.0))
            .text_color(paint.look.secondary)
            .child(line.clone())
            .into_any_element(),
        EntryKind::Status { .. } => meta(
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
fn person(paint: &Paint<'_>, prompt: &Prompt, corner: Option<String>) -> AnyElement {
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
            div()
                .max_w(relative(BUBBLE_SHARE))
                .min_w(px(0.0))
                .px(px(16.0))
                .py(px(10.0))
                .rounded(px(BUBBLE_RADIUS))
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
                            .child(prompt.text.clone()),
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
    div()
        .id(ElementId::NamedInteger("work".into(), id(index)))
        .flex()
        .items_center()
        .gap(px(6.0))
        .min_h(px(WORK_LINE))
        .px(px(4.0))
        .rounded(px(6.0))
        .text_size(px(WORK_SIZE))
        .line_height(px(WORK_LINE))
        .text_color(paint.look.secondary)
        .cursor_pointer()
        .hover(move |row| row.text_color(hover))
        .on_click(cx.listener(move |shell, _event, _window, cx| shell.toggle_entry(index, cx)))
        .child(
            div()
                .w(px(16.0))
                .flex()
                .justify_center()
                .child(if open { "▾" } else { "▸" }),
        )
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
        .text_size(px(WORK_SIZE))
        .line_height(px(WORK_LINE))
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
                .text_size(px(12.0))
                .text_color(ink)
                .child(text),
        )
        .child(rule())
        .into_any_element()
}

/// A question the agent asked, and how it was settled.
fn question_card(paint: &Paint<'_>, message: &str, settled: Option<&'static str>) -> AnyElement {
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
                .child(message.to_owned()),
        )
        .children(settled.map(|word| {
            div()
                .text_size(px(WORK_SIZE))
                .text_color(paint.look.secondary)
                .child(word)
        }))
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
