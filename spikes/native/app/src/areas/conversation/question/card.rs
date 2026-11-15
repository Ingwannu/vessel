//! The question's card as Paseo draws it (`question-form-card.tsx`): its
//! tabs, the step's question, its option rows and text box, what is
//! wrong or refused, and Dismiss with Next or Submit. Its state and what
//! its controls do are the parent module's; the step's rules are the
//! view's (`view::timeline::question_step`).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{
    CardLook, CardState, Tokens, Type, TypeExt as _, breakpoint, card_button, chat_markdown,
    markdown, markdown_label, version_of,
};
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{
    Choice, Problem, Step, Wrong, answerable, choices, picked, step_answered, step_header,
    step_problem, step_prompt, step_ready, steps_of, text_target,
};
use vessel_proto::Question;

use super::{Answering, Cursor, ResponseAction, waiting};
use crate::areas::conversation::Place;
use crate::shell::Shell;

mod options;
use options::option_row;

/// The tallest the option rows stand before they scroll: a phone's, and
/// any wider window's.
const OPTIONS_PHONE: f32 = 280.0;
const OPTIONS: f32 = 360.0;

/// The card for the question waiting first in the conversation at `P`,
/// when one waits.
pub(crate) fn card<P: Place>(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = (P::SEAT.get)(shell)?;
    let pending = waiting(area);
    if area.answering.len() != pending.len()
        || pending
            .iter()
            .any(|question| area.answering.get(question.id).is_none())
    {
        cx.defer_in(window, |shell, window, cx| {
            shell.question_prepare::<P>(window, cx)
        });
    }
    let cards: Vec<_> = pending
        .into_iter()
        .filter_map(|question| super::card_at::<P>(shell, question.id, tokens, window, cx))
        .collect();
    if cards.is_empty() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S2))
            .children(cards)
            .into_any_element(),
    )
}

/// Inbox and other exact-request views choose an identity, never a list position.
pub(crate) fn card_at<P: Place>(
    shell: &Shell,
    id: vessel_proto::QuestionId,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = (P::SEAT.get)(shell)?;
    let question = waiting(area)
        .into_iter()
        .find(|question| question.id == id)?;
    let Some(answering) = area.answering.get(id) else {
        cx.defer_in(window, |shell, window, cx| {
            shell.question_prepare::<P>(window, cx)
        });
        return None;
    };
    let steps = steps_of(question);
    let step = steps.get(answering.step).copied()?;
    let words = shell.t.lang.conversation();
    let neutral = tokens.neutral;
    let narrow = breakpoint::phone(window);
    let refused = area
        .response_failures
        .get(vessel_app_view::responses::Request::Question(question.id))
        .map(|failure| crate::areas::responses::problem(shell.words, failure).to_owned())
        .or_else(|| answering.validation.clone());
    let sending = answering.sending.filter(|_| refused.is_none());
    let waiting_on_node = sending.is_some()
        || P::busy(shell)
        || !area
            .response_failures
            .retryable(vessel_app_view::responses::Request::Question(question.id));
    let last = answering.step.saturating_add(1) >= steps.len();
    let ready = ready(&steps, step, answering);
    answering.field.set_editability(
        if waiting_on_node {
            vessel_app_kit::field::Editability::ReadOnly
        } else {
            vessel_app_kit::field::Editability::Editable
        },
        cx,
    );
    let block = answers::<P>(
        step,
        answering,
        words.question_yes_no(),
        (narrow, waiting_on_node),
        tokens,
        cx,
    );
    let said = said(refused, question, step, answering, *words);
    let prompt = step_prompt(question, step);
    let prompt_body = chat_markdown(
        answering.field.id().element_id("question-prompt"),
        &prompt,
        version_of(&prompt),
        None,
        tokens,
        window,
        cx,
    );
    Some(
        div()
            .id(answering.field.id().element_id("question-card"))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(12.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(neutral.border)
            .bg(neutral.surface1)
            .face(Type::Body)
            .text_size(px(14.0))
            .children((steps.len() > 1).then(|| tabs::<P>(&steps, answering, tokens, cx)))
            .child(
                div()
                    .px(px(12.0))
                    .pb(px(4.0))
                    .text_color(neutral.foreground)
                    .child(prompt_body),
            )
            .child(block)
            .children(said.map(|said| said_row(said, tokens)))
            .child(actions::<P>(
                answering.cursor(),
                (
                    words.question_dismiss(),
                    if last {
                        words.question_submit()
                    } else {
                        words.question_next()
                    },
                ),
                (ready && !waiting_on_node, !waiting_on_node),
                sending,
                narrow,
                tokens,
                cx,
            ))
            .into_any_element(),
    )
}

/// Whether Next (Submit on the last step) may be pressed: this step is
/// ready, or on the last step every step is.
fn ready(steps: &[Step<'_>], step: Step<'_>, answering: &Answering) -> bool {
    if answering.step.saturating_add(1) >= steps.len() {
        steps
            .iter()
            .all(|step| step_ready(*step, &answering.drafts))
    } else {
        step_ready(step, &answering.drafts)
    }
}

/// What the card says under the answers, the first that applies: the
/// node's refusal, what is wrong with this step's answer, or that the
/// question cannot be answered here.
fn said(
    refused: Option<String>,
    question: &Question,
    step: Step<'_>,
    answering: &Answering,
    words: ConversationWords,
) -> Option<String> {
    let problem = match step_problem(step, &answering.drafts) {
        Some(problem @ Problem::Wrong(_)) => Some(words.question_problem(&problem)),
        Some(Problem::ChooseAnAnswer | Problem::Required) | None => None,
    };
    refused
        .or(problem)
        .or_else(|| (!answerable(question)).then(|| words.question_wrong(&Wrong::Unanswerable)))
}

/// What the card says, in the danger colour.
fn said_row(said: String, tokens: &Tokens) -> Div {
    div()
        .px(px(4.0))
        .text_size(px(13.0))
        .text_color(tokens.neutral.destructive)
        .child(said)
}

/// A step's answers: its option rows (scrolling past a height), then its
/// text box. `(narrow, busy)`: a phone's width; the answer has gone.
fn answers<P: Place>(
    step: Step<'_>,
    answering: &Answering,
    yes_no: (&str, &str),
    (narrow, busy): (bool, bool),
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let (options, multiple) = choices(step, yes_no);
    let mut block = div().flex().flex_col().gap(px(spacing::S2));
    if !options.is_empty() {
        let rows = options.into_iter().enumerate().map(|(index, choice)| {
            let on = picked(step, &answering.drafts, &choice.value);
            option_row::<P>(
                (index, answering.cursor(), answering.field.id()),
                choice,
                on,
                multiple,
                busy,
                tokens,
                cx,
            )
        });
        // Paseo's card scrolls with the conversation; here it stands over
        // the composer, so a long list scrolls inside it instead of
        // pushing the conversation out of the window.
        block = block.child(
            div()
                .id("question-options")
                .flex()
                .flex_col()
                .gap(px(4.0))
                .max_h(px(if narrow { OPTIONS_PHONE } else { OPTIONS }))
                .overflow_y_scroll()
                .children(rows),
        );
    }
    if text_target(step).is_some() {
        block = block.child(answering.field.element(tokens, narrow, cx));
    }
    block
}

/// Paseo's titled tabs, a check on the answered ones.
fn tabs<P: Place>(
    steps: &[Step<'_>],
    answering: &Answering,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let cursor = answering.cursor();
    let tabs = steps.iter().enumerate().map(|(index, step)| {
        let active = index == answering.step;
        let ink = if active {
            neutral.foreground
        } else {
            neutral.foreground_muted
        };
        div()
            .id(("question-tab", index))
            .flex()
            .items_center()
            .gap(px(4.0))
            .min_h(px(28.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if active {
                neutral.foreground_muted
            } else {
                neutral.border
            })
            .bg(if active {
                neutral.surface2
            } else {
                neutral.surface1
            })
            .hover(move |style| style.bg(neutral.surface2))
            .cursor_pointer()
            .text_color(ink)
            .when(step_answered(*step, &answering.drafts), |tab| {
                tab.child(glyph(Glyph::Check, 12.0, ink))
            })
            .child(div().truncate().child(step_header(*step)))
            .on_click(cx.listener(move |shell, _event, window, cx| {
                if shell.question_cursor::<P>(cursor) {
                    shell.question_show_at::<P>(cursor.id, index, window, cx);
                }
            }))
    });
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(4.0))
        .px(px(12.0))
        .children(tabs)
}

/// Dismiss, then Next or Submit (the accent): in a row, stacked on a
/// phone. `(primary, dismiss)`: whether each answers a click.
fn actions<P: Place>(
    cursor: Cursor,
    (dismiss_words, primary_words): (&'static str, &'static str),
    (primary, dismiss): (bool, bool),
    sending: Option<ResponseAction>,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let dismiss_button = card_button(
        "question-dismiss",
        Glyph::Remove,
        dismiss_words,
        CardLook::Muted,
        if sending == Some(ResponseAction::Dismiss) {
            CardState::Sending
        } else if dismiss {
            CardState::Ready
        } else {
            CardState::Waiting
        },
        tokens,
    )
    .when(dismiss, |button| {
        button.on_click(cx.listener(move |shell, _event, _window, cx| {
            if shell.question_cursor::<P>(cursor) {
                shell.question_dismiss_at::<P>(cursor.id, cx);
            }
        }))
    });
    let primary_button = card_button(
        "question-primary",
        Glyph::Check,
        primary_words,
        CardLook::Accent,
        if sending == Some(ResponseAction::Submit) {
            CardState::Sending
        } else if sending.is_some() {
            CardState::Waiting
        } else if primary {
            CardState::Ready
        } else {
            CardState::Faded
        },
        tokens,
    )
    .when(primary, |button| {
        button.on_click(cx.listener(move |shell, _event, window, cx| {
            if shell.question_cursor::<P>(cursor) {
                shell.question_primary_at::<P>(cursor.id, window, cx);
            }
        }))
    });
    div()
        .flex()
        .gap(px(spacing::S2))
        .map(|row| {
            if narrow {
                row.flex_col()
            } else {
                row.flex_row().items_center()
            }
        })
        .child(dismiss_button)
        .child(primary_button)
}

#[cfg(test)]
mod tests;
