//! A session: its conversation as it grows, and the composer (DESIGN.md
//! section 6.15: the prompt in mono, the agent's reply as body text,
//! thoughts quieter, tool calls as rows with their state; the composer a
//! surface at the foot).
//!
//! The conversation is a list that draws only what is in view (GPUI's
//! `list`, measured rows), so its cost does not grow with its length.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement, ParentElement as _, Styled as _, Window, div,
    list, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button, markdown};
use vessel_app_view::{Asked, Speaker, ToolState};

use super::title_bar;
use crate::shell::Shell;

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Some(open) = &shell.open else {
        return div();
    };
    let detail = format!("{} · {}", open.machine, open.agent);
    let body: AnyElement = if open.structured {
        let entries = list(
            shell.entries.clone(),
            cx.processor(|this: &mut Shell, index: usize, _window, cx| {
                entry(this, index, &vessel_app_kit::tokens(cx))
            }),
        )
        .flex_1()
        .size_full();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(entries)
            .into_any_element()
    } else {
        div()
            .flex_1()
            .p(px(24.0))
            .type_(Type::Body)
            .text_color(tokens.text_secondary)
            .child("A terminal session. Terminals are not in this build yet.")
            .into_any_element()
    };
    let working = shell.conversation.turn_running();
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(open.title.clone(), Some(detail), tokens))
        .child(body)
        .children(
            shell
                .conversation
                .asked()
                .first()
                .map(|asked| permission(asked, tokens, cx)),
        )
        .when(open.structured, |frame| {
            frame.child(composer(shell, tokens, working, cx))
        })
}

fn composer(shell: &Shell, tokens: &Tokens, working: bool, cx: &mut Context<'_, Shell>) -> Div {
    div()
        .p(px(12.0))
        .border_t_1()
        .border_color(tokens.border_faint)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().child(shell.composer.element()))
                .child(
                    button("send", if working { "Queue" } else { "Send" }, true, tokens).on_click(
                        cx.listener(|this, _event, window, cx| {
                            let text = this.composer.text(cx);
                            this.submit_prompt(text, window, cx);
                        }),
                    ),
                ),
        )
        .child(
            div()
                .pt(px(6.0))
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(if working {
                    "The agent is working; a prompt now waits in the queue."
                } else {
                    "Enter sends."
                }),
        )
}

/// The oldest permission waiting: what it is for, and a button per answer
/// (the allowing ones primary).
fn permission(asked: &Asked, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut answers = div().flex().flex_row().gap(px(8.0));
    for (index, (option, words, allows)) in asked.options.iter().enumerate() {
        let id = asked.id.clone();
        let option = option.clone();
        answers = answers.child(
            button(
                ElementId::Name(format!("answer-{index}").into()),
                words.clone(),
                *allows,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.answer(id.clone(), option.clone(), cx);
            })),
        );
    }
    div()
        .mx(px(12.0))
        .mb(px(8.0))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(tokens.st_needs_you)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.text_primary)
                .child(format!("The agent asks to run: {}", asked.what)),
        )
        .child(answers)
}

fn entry(shell: &Shell, index: usize, tokens: &Tokens) -> AnyElement {
    let Some(entry) = shell.conversation.entries().get(index) else {
        return div().into_any_element();
    };
    let row = div().w_full().px(px(24.0)).py(px(6.0));
    match entry.speaker {
        Speaker::Person => row
            .child(
                div()
                    .p(px(10.0))
                    .bg(tokens.bg_surface)
                    .border_1()
                    .border_color(tokens.border_subtle)
                    .rounded(px(vessel_app_kit::theme::radius::R2))
                    .type_(Type::Mono)
                    .text_color(tokens.text_primary)
                    .child(entry.text.clone()),
            )
            .into_any_element(),
        Speaker::Agent => row
            .type_(Type::Body)
            .text_color(tokens.text_primary)
            .child(markdown(
                ElementId::Name(format!("entry-{index}").into()),
                entry.text.clone(),
            ))
            .into_any_element(),
        Speaker::Thought => row
            .type_(Type::Meta)
            .text_color(tokens.text_tertiary)
            .child(entry.text.clone())
            .into_any_element(),
        Speaker::Tool => {
            let state = entry.tool.unwrap_or(ToolState::Pending);
            let color = match state {
                ToolState::Pending | ToolState::Running => tokens.st_building,
                ToolState::Done => tokens.st_ready,
                ToolState::Failed => tokens.danger,
            };
            row.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(color)
                            .child(state.word()),
                    )
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.text_secondary)
                            .child(entry.text.clone()),
                    ),
            )
            .into_any_element()
        }
        Speaker::Notice => row
            .type_(Type::Meta)
            .text_color(tokens.text_tertiary)
            .child(entry.text.clone())
            .into_any_element(),
    }
}

use vessel_app_kit::ui::FluentBuilder as _;
use vessel_app_kit::ui::StatefulInteractiveElement as _;
