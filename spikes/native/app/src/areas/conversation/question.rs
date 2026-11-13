//! A question the agent asks, answered in the session: Paseo's question
//! card (`components/question-form-card.tsx`) in the slot over the
//! composer, above the queue. The form's steps, drafts and checks are the
//! view's (`view::timeline::question`, the page's logic).
//!
//! [Decision Log]
//! - Purpose: Claude's `AskUserQuestion` (and any agent's form) was drawn
//!   in the timeline as its message only, with no way to answer it: the
//!   turn waited on an answer the app could not give.
//! - Constraint: the composer's card and the permission panel are the
//!   lead's (`areas/session.rs`); the conversation draws in its slot over
//!   the card (`dock`). A text box needs a window to be made, and the
//!   slot draws from a shared borrow of the shell.
//! - The reference: Paseo's card, which its permission card shares
//!   (surface1, a hairline, radius 8, 12 inside, 12 between its parts): a
//!   tab per question (its header, a check once answered; none for a lone
//!   question), the question, its options as rows (a radio or a checkbox
//!   of 18, the label muted until picked, the description under it), an
//!   "Other..." box, then Dismiss and Next or Submit (stacked on a
//!   phone). Picking a single choice clears the Other box and typing in it
//!   clears the pick, as in Claude Code.
//! - Choice: Paseo's card, one step at a time. A yes-or-no field is two
//!   radio rows; a text or number field is the box alone ("Type your
//!   answer..."). One text box serves every step (its text and its
//!   placeholder change with the step), made the frame after the question
//!   arrives (`defer_in`, the slot cannot make it). Dismiss declines; the
//!   answer is checked by the node's own check (`Question::check`) before
//!   it goes, and a refusal (another client answered first, a value the
//!   node will not take) is said under the card with the node's words.
//! - Differences, kept: a question and its options are plain text (Paseo
//!   renders their Markdown); no option previews (Claude's `preview`
//!   reaches the node only under `_meta`, which the node does not keep);
//!   no spinner while the answer goes (the buttons dim); the option rows
//!   scroll inside the card past 360 (280 on a phone), where Paseo's card
//!   scrolls with the conversation.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{
    CardLook, CardState, TextField, Tokens, Type, TypeExt as _, breakpoint, card_button,
};
use vessel_app_link::ActKind;
use vessel_app_view::timeline::{
    Draft, Drafts, EntryKind, Problem, Step, answer_of, answerable, refused_field, starting_drafts,
    step_answered, step_problem, step_ready, steps_of,
};
use vessel_proto::conversation::{FieldKind, QuestionField};
use vessel_proto::{Question, QuestionAnswer, QuestionId};

use super::{Answer, ConversationArea, Place};
use crate::shell::Shell;

/// The tallest the option rows stand before they scroll: a phone's, and
/// any wider window's.
const OPTIONS_PHONE: f32 = 280.0;
const OPTIONS: f32 = 360.0;

/// A question being answered: what each field holds and the step shown.
pub(crate) struct Answering {
    id: QuestionId,
    drafts: Drafts,
    step: usize,
    /// The step's text box: a free-text step's answer, or a choice's
    /// "Other".
    field: TextField,
    /// The answer went; the node has not settled the question yet.
    sent: bool,
}

impl std::fmt::Debug for Answering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answering")
            .field("id", &self.id)
            .field("step", &self.step)
            .field("sent", &self.sent)
            .finish_non_exhaustive()
    }
}

/// One row of a step's options: what it sends, its words.
struct Choice {
    value: String,
    title: String,
    description: Option<String>,
}

/// The question waiting first in `area`, if any: the log's, else one the
/// node says waits whose record the log no longer holds.
fn waiting(area: &ConversationArea) -> Option<Question> {
    let timeline = &area.timeline;
    timeline
        .waiting_questions()
        .first()
        .and_then(|entry| {
            if let EntryKind::Question { question, .. } = &entry.kind {
                Some(question.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            timeline
                .unseen_questions(&area.live)
                .first()
                .map(|question| (*question).clone())
        })
}

/// The question waiting first at `P`, if any.
fn waiting_at<P: Place>(shell: &Shell) -> Option<Question> {
    (P::SEAT.get)(shell).and_then(waiting)
}

/// The field a step's text box writes: Claude's "Other", or a free-text
/// step's own field.
fn text_target(step: Step<'_>) -> Option<&QuestionField> {
    match &step.field.kind {
        FieldKind::Choice { .. } | FieldKind::Boolean { .. } => step.other,
        FieldKind::Text { .. } | FieldKind::Number { .. } | FieldKind::Integer { .. } => {
            Some(step.field)
        }
        FieldKind::Unknown => None,
    }
}

/// A step's option rows, and whether several may be picked.
fn choices(step: Step<'_>, (yes, no): (&str, &str)) -> (Vec<Choice>, bool) {
    match &step.field.kind {
        FieldKind::Choice {
            options, multiple, ..
        } => (
            options
                .iter()
                .map(|option| Choice {
                    value: option.value.clone(),
                    title: option.title.clone(),
                    description: option.description.clone(),
                })
                .collect(),
            *multiple,
        ),
        FieldKind::Boolean { .. } => (
            [("true", yes), ("false", no)]
                .into_iter()
                .map(|(value, title)| Choice {
                    value: value.to_owned(),
                    title: title.to_owned(),
                    description: None,
                })
                .collect(),
            false,
        ),
        FieldKind::Text { .. }
        | FieldKind::Number { .. }
        | FieldKind::Integer { .. }
        | FieldKind::Unknown => (Vec::new(), false),
    }
}

/// Whether `value` is picked in `step`.
fn picked(step: Step<'_>, drafts: &Drafts, value: &str) -> bool {
    match drafts.get(&step.field.name) {
        Some(Draft::Picked(values)) => values.iter().any(|one| one == value),
        Some(Draft::Boolean(Some(on))) => (value == "true") == *on,
        Some(Draft::Text(_) | Draft::Boolean(None)) | None => false,
    }
}

/// The step's tab label: its header (the field's title), else its name.
fn header(step: Step<'_>) -> String {
    step.field
        .title
        .clone()
        .unwrap_or_else(|| step.field.name.as_str().to_owned())
}

/// The step's question: the field's own words when the form has several
/// (Claude puts each question's text there), else the question's message.
fn prompt(question: &Question, step: Step<'_>) -> String {
    step.field
        .description
        .clone()
        .unwrap_or_else(|| question.message.clone())
}

impl Shell {
    /// Make the card's state for the question waiting first, or drop it
    /// when none waits.
    fn question_prepare<P: Place>(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(question) = waiting_at::<P>(self) else {
            if (P::SEAT.get_mut)(self).is_some_and(|area| area.answering.take().is_some()) {
                cx.notify();
            }
            return;
        };
        if (P::SEAT.get)(self)
            .and_then(|area| area.answering.as_ref())
            .is_some_and(|answering| answering.id == question.id)
        {
            return;
        }
        let words = self.t.lang.conversation();
        let field = TextField::with_change(
            words.question_other_placeholder(),
            Self::question_entered::<P>,
            Self::question_typed::<P>,
            window,
            cx,
        );
        let Some(area) = (P::SEAT.get_mut)(self) else {
            return;
        };
        area.answering = Some(Answering {
            id: question.id,
            drafts: starting_drafts(&question),
            step: 0,
            field,
            sent: false,
        });
        self.question_show::<P>(0, window, cx);
    }

    /// Show step `index`: its text box holds that step's text.
    fn question_show<P: Place>(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.conversation();
        let Some(question) = waiting_at::<P>(self) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.as_mut())
        else {
            return;
        };
        let Some(step) = steps.get(index).copied() else {
            return;
        };
        answering.step = index;
        let target = text_target(step);
        let text = match target.and_then(|field| answering.drafts.get(&field.name)) {
            Some(Draft::Text(text)) => text.clone(),
            Some(Draft::Picked(_) | Draft::Boolean(_)) | None => String::new(),
        };
        let placeholder = if matches!(
            step.field.kind,
            FieldKind::Choice { .. } | FieldKind::Boolean { .. }
        ) {
            words.question_other_placeholder()
        } else {
            words.question_answer_placeholder()
        };
        answering.field.set_placeholder(placeholder, window, cx);
        answering.field.set_text(&text, window, cx);
        cx.notify();
    }

    /// The text box changed: the step's text, and for a single choice the
    /// pick cleared (an option and a custom answer replace each other).
    fn question_typed<P: Place>(&mut self, text: String, cx: &mut Context<'_, Self>) {
        let Some(question) = waiting_at::<P>(self) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.as_mut())
        else {
            return;
        };
        let Some(step) = steps.get(answering.step).copied() else {
            return;
        };
        let Some(target) = text_target(step) else {
            return;
        };
        let single = matches!(
            step.field.kind,
            FieldKind::Choice {
                multiple: false,
                ..
            } | FieldKind::Boolean { .. }
        );
        if single && !text.is_empty() {
            let cleared = if matches!(step.field.kind, FieldKind::Boolean { .. }) {
                Draft::Boolean(None)
            } else {
                Draft::Picked(Vec::new())
            };
            let _was = answering.drafts.insert(step.field.name.clone(), cleared);
        }
        let _was = answering
            .drafts
            .insert(target.name.clone(), Draft::Text(text));
        cx.notify();
    }

    fn question_entered<P: Place>(
        &mut self,
        _text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.question_primary::<P>(window, cx);
    }

    /// Pick `value` in the step shown: a radio's or a yes-or-no field's
    /// one value (picking it again clears it), a checkbox's toggle.
    fn question_pick<P: Place>(
        &mut self,
        value: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(question) = waiting_at::<P>(self) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.as_mut())
        else {
            return;
        };
        let Some(step) = steps.get(answering.step).copied() else {
            return;
        };
        let was = picked(step, &answering.drafts, value);
        let draft = match &step.field.kind {
            FieldKind::Boolean { .. } => Draft::Boolean((!was).then_some(value == "true")),
            FieldKind::Choice { multiple: true, .. } => {
                let mut values = match answering.drafts.get(&step.field.name) {
                    Some(Draft::Picked(values)) => values.clone(),
                    Some(Draft::Text(_) | Draft::Boolean(_)) | None => Vec::new(),
                };
                if was {
                    values.retain(|one| one != value);
                } else {
                    values.push(value.to_owned());
                }
                Draft::Picked(values)
            }
            FieldKind::Choice { .. } => Draft::Picked(if was {
                Vec::new()
            } else {
                vec![value.to_owned()]
            }),
            FieldKind::Text { .. }
            | FieldKind::Number { .. }
            | FieldKind::Integer { .. }
            | FieldKind::Unknown => return,
        };
        let _was = answering.drafts.insert(step.field.name.clone(), draft);
        let single = !matches!(step.field.kind, FieldKind::Choice { multiple: true, .. });
        if single
            && !was
            && let Some(other) = step.other
        {
            let _was = answering
                .drafts
                .insert(other.name.clone(), Draft::Text(String::new()));
            answering.field.set_text("", window, cx);
        }
        cx.notify();
    }

    /// Next, or Submit on the last step.
    fn question_primary<P: Place>(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(question) = waiting_at::<P>(self) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get)(self).and_then(|area| area.answering.as_ref()) else {
            return;
        };
        let Some(step) = steps.get(answering.step).copied() else {
            return;
        };
        if !step_ready(step, &answering.drafts) {
            return;
        }
        let next = answering.step.saturating_add(1);
        if next < steps.len() {
            self.question_show::<P>(next, window, cx);
            return;
        }
        if let Some(unready) = steps
            .iter()
            .position(|step| !step_ready(*step, &answering.drafts))
        {
            self.question_show::<P>(unready, window, cx);
            return;
        }
        let answer = answer_of(&question, &answering.drafts);
        if let Err(refused) = question.check(&answer) {
            let message = refused.to_string();
            let at = refused_field(&question, &message).and_then(|name| {
                steps.iter().position(|step| {
                    step.field.name == *name || step.other.is_some_and(|other| other.name == *name)
                })
            });
            if let Some(area) = (P::SEAT.get_mut)(self) {
                area.refused = Some((ActKind::Answer, message));
            }
            if let Some(at) = at {
                self.question_show::<P>(at, window, cx);
            }
            cx.notify();
            return;
        }
        self.question_send::<P>(question.id, answer, cx);
    }

    /// Decline the question.
    fn question_dismiss<P: Place>(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(id) = (P::SEAT.get)(self)
            .and_then(|area| area.answering.as_ref())
            .map(|answering| answering.id)
        {
            self.question_send::<P>(id, QuestionAnswer::Decline, cx);
        }
    }

    fn question_send<P: Place>(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(area) = (P::SEAT.get_mut)(self) {
            if let Some(answering) = area.answering.as_mut() {
                answering.sent = true;
            }
            area.refused = None;
        }
        P::answer(self, Answer::Question(id, answer), cx);
        cx.notify();
    }
}

/// The card for the question waiting first in the conversation at `P`,
/// when one waits.
pub(crate) fn card<P: Place>(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = (P::SEAT.get)(shell)?;
    let question = waiting(area);
    let answering = match (&question, &area.answering) {
        (None, None) => return None,
        (Some(question), Some(answering)) if answering.id == question.id => answering,
        // Made (or dropped) on the next frame: the slot cannot make a box.
        _ => {
            cx.defer_in(window, |shell, window, cx| {
                shell.question_prepare::<P>(window, cx);
            });
            return None;
        }
    };
    let question = question?;
    let steps = steps_of(&question);
    let step = steps.get(answering.step).copied()?;
    let words = shell.t.lang.conversation();
    let neutral = tokens.neutral;
    let narrow = breakpoint::phone(window);
    let refused = area
        .refused
        .as_ref()
        .filter(|(kind, _)| *kind == ActKind::Answer)
        .map(|(_, message)| message.clone());
    let waiting_on_node = (answering.sent && refused.is_none()) || P::busy(shell);
    let last = answering.step.saturating_add(1) >= steps.len();
    let ready = if last {
        steps
            .iter()
            .all(|step| step_ready(*step, &answering.drafts))
    } else {
        step_ready(step, &answering.drafts)
    };
    let problem = match step_problem(step, &answering.drafts) {
        Some(problem @ Problem::Wrong(_)) => Some(words.question_problem(&problem)),
        Some(Problem::ChooseAnAnswer | Problem::Required) | None => None,
    };
    let block = answers::<P>(
        step,
        answering,
        words.question_yes_no(),
        (narrow, waiting_on_node),
        tokens,
        cx,
    );
    let said = refused
        .map(|message| words.question_refused(&message))
        .or(problem)
        .or_else(|| {
            (!answerable(&question))
                .then(|| words.question_wrong(&vessel_app_view::timeline::Wrong::Unanswerable))
        });
    Some(
        div()
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
                    .child(prompt(&question, step)),
            )
            .child(block)
            .children(said.map(|said| {
                div()
                    .px(px(4.0))
                    .text_size(px(13.0))
                    .text_color(neutral.destructive)
                    .child(said)
            }))
            .child(actions::<P>(
                words.question_dismiss(),
                if last {
                    words.question_submit()
                } else {
                    words.question_next()
                },
                (ready && !waiting_on_node, !waiting_on_node),
                narrow,
                tokens,
                cx,
            ))
            .into_any_element(),
    )
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
    let mut block = div().flex().flex_col().gap(px(8.0));
    if !options.is_empty() {
        let rows = options.into_iter().enumerate().map(|(index, choice)| {
            let on = picked(step, &answering.drafts, &choice.value);
            option_row::<P>(index, choice, on, multiple, busy, tokens, cx)
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
            .child(div().truncate().child(header(*step)))
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.question_show::<P>(index, window, cx);
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

/// One option: Paseo's row, a radio or a checkbox at its left.
fn option_row<P: Place>(
    index: usize,
    choice: Choice,
    on: bool,
    multiple: bool,
    busy: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let control = div()
        .mt(px(2.0))
        .size(px(18.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(if on {
            neutral.accent
        } else {
            neutral.foreground_extra_muted
        })
        .map(|control| {
            if multiple {
                control.rounded(px(4.0))
            } else {
                control.rounded_full()
            }
        })
        .when(on && multiple, |control| {
            control
                .bg(neutral.accent)
                .child(glyph(Glyph::Check, 12.0, neutral.accent_foreground))
        })
        .when(on && !multiple, |control| {
            control.child(div().size(px(8.0)).rounded_full().bg(neutral.accent))
        });
    let value = choice.value;
    div()
        .id(("question-option", index))
        .flex()
        .items_start()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .when(on, |row| row.bg(neutral.surface2))
        .hover(move |style| style.bg(neutral.surface2))
        .cursor_pointer()
        .when(!busy, |row| {
            row.on_click(cx.listener(move |shell, _event, window, cx| {
                shell.question_pick::<P>(&value, window, cx);
            }))
        })
        .child(control)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .line_height(px(22.0))
                        .text_color(if on {
                            neutral.foreground
                        } else {
                            neutral.foreground_muted
                        })
                        .child(choice.title),
                )
                .children(choice.description.map(|description| {
                    div()
                        .text_size(px(13.0))
                        .text_color(neutral.foreground_muted)
                        .child(description)
                })),
        )
}

/// Dismiss, then Next or Submit (the accent): in a row, stacked on a
/// phone. `(primary, dismiss)`: whether each answers a click.
fn actions<P: Place>(
    dismiss_words: &'static str,
    primary_words: &'static str,
    (primary, dismiss): (bool, bool),
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let dismiss_button = card_button(
        "question-dismiss",
        Glyph::Remove,
        dismiss_words,
        CardLook::Muted,
        if dismiss {
            CardState::Ready
        } else {
            CardState::Waiting
        },
        tokens,
    )
    .when(dismiss, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| {
            shell.question_dismiss::<P>(cx);
        }))
    });
    let primary_button = card_button(
        "question-primary",
        Glyph::Check,
        primary_words,
        CardLook::Accent,
        if primary {
            CardState::Ready
        } else {
            CardState::Faded
        },
        tokens,
    )
    .when(primary, |button| {
        button.on_click(cx.listener(|shell, _event, window, cx| {
            shell.question_primary::<P>(window, cx);
        }))
    });
    div()
        .flex()
        .gap(px(8.0))
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
