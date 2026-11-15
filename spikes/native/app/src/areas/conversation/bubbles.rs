//! The message exchange (slice M, the owner's words 33 and 36–39): the
//! person's message as Paseo's `UserMessage` bubble on the right (also
//! the moment it is sent, before the node records it), and the agent's
//! reply in the owner's mirrored bubble beside the agent's circle.

use vessel_app_kit::chat_markdown::actions::CopyStyle;
use vessel_app_kit::chat_markdown::{Options, chat_markdown_with};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _,
    Window, div, px, relative,
};
use vessel_app_kit::{FindMarks, Type, TypeExt as _, plain_text, plain_text_with_marks};
use vessel_app_view::timeline::{Prompt, first_reply, presented_prompt, spans_of};

use super::entries::{Paint, id};
use super::look::{AVATAR, AVATAR_GAP, BUBBLE_RADIUS, BUBBLE_SHARE, CORNER};
use crate::shell::Shell;

/// The person's message the moment it is sent, before the node records
/// it (Paseo's pending `UserMessage`): drawn as the message it becomes,
/// at `index`, the entry it will be.
pub(super) fn pending(
    paint: &Paint<'_>,
    index: usize,
    text: &str,
    presentation: super::pacing::Presentation<'_>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
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
        presentation,
        window,
        cx,
    )
}

/// A person's prompt or steer: Paseo's `UserMessage` bubble on the
/// right, as wide as the column needs, its corner word above it.
pub(super) fn person(
    paint: &Paint<'_>,
    index: usize,
    prompt: &Prompt,
    corner: Option<String>,
    presentation: super::pacing::Presentation<'_>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let corner = corner.or_else(|| prompt.replayed.then(|| paint.words.prompt_corner(true, 0)));
    let text = presented_prompt(&prompt.text);
    let content = (!text.is_empty()).then(|| {
        let key = ElementId::NamedInteger("person-text".into(), id(index));
        let version = paint
            .entries
            .get(index)
            .map_or(0, |entry| u64::from(entry.rev));
        let body = presentation.scoped(window, |window| match paint.find {
            Some(find) => {
                let spans = |rendered: &str| spans_of(rendered, find.query);
                plain_text_with_marks(
                    key,
                    text,
                    version,
                    &FindMarks {
                        list: find.list,
                        clear_top: find.clear_top,
                        key: find.key(index),
                        spans: &spans,
                        current: find.current_in(index),
                    },
                    paint.tokens,
                    window,
                    cx,
                )
            }
            None => plain_text(key, text, version, window, cx),
        });
        div()
            .type_(Type::Body)
            .text_color(paint.tokens.neutral.foreground)
            .child(body)
    });
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
                .children(content),
        )
        .into_any_element()
}

/// The agent's reply: its Markdown in the owner's reply bubble beside the
/// agent's circle, find's matches marked.
pub(super) fn message(
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
    // own surface with a hairline. It hugs its words as the person's does
    // (a short reply is a short bubble, not a panel) and grows to the
    // column at most: a wide table or code block takes the column and
    // scrolls sideways inside it. The agent's mark in a circle beside a
    // turn's first reply, its place kept beside the rest so they line up.
    let bubble = div()
        .max_w(relative(BUBBLE_SHARE))
        .min_w(px(0.0))
        .p(px(16.0))
        .rounded(px(BUBBLE_RADIUS))
        .rounded_tl(px(CORNER))
        .bg(paint.look.reply)
        .border_1()
        .border_color(paint.look.border)
        .type_(Type::Body)
        .text_color(paint.tokens.neutral.foreground)
        .child(chat_markdown_with(
            ElementId::NamedInteger("message".into(), id(index)),
            text,
            vessel_app_kit::version_of(&(rev, text.len())),
            Options {
                marks: marks.as_ref(),
                actions: None,
                copy: CopyStyle::MarkdownCodeAware,
            },
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
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .items_start()
                .child(bubble),
        )
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

#[cfg(test)]
mod tests;
