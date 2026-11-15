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

use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::ActKind;
use vessel_app_view::timeline::{
    Draft, Drafts, EntryKind, Pick, Step, answer_of, box_is_other, pick, refused_field,
    starting_drafts, step_ready, steps_of, text_target, typed,
};
use vessel_proto::{Question, QuestionAnswer, QuestionId};

use super::{Answer, ConversationArea, Place, Seat, SessionPlace};
use crate::shell::Shell;

mod card;

pub(crate) use card::card;

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

/// Arrival and refresh share the step's hint without reloading its draft.
fn field_placeholder(step: Step<'_>, words: vessel_app_view::ConversationWords) -> &'static str {
    if box_is_other(step) {
        words.question_other_placeholder()
    } else {
        words.question_answer_placeholder()
    }
}

impl Shell {
    /// Update only this session's retained step hint; other places own theirs.
    pub(crate) fn refresh_question_words(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(question) = waiting_at::<SessionPlace>(self) else {
            return;
        };
        let words = self.t.lang.conversation();
        let steps = steps_of(&question);
        let Some(answering) =
            (Seat::SESSION.get_mut)(self).and_then(|area| area.answering.as_mut())
        else {
            return;
        };
        let Some(step) = steps.get(answering.step) else {
            return;
        };
        answering
            .field
            .set_placeholder(field_placeholder(*step, *words), window, cx);
    }

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
        let placeholder = field_placeholder(step, *words);
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
        typed(step, &mut answering.drafts, text);
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
        match pick(step, &mut answering.drafts, value) {
            Pick::Ignored => return,
            Pick::Picked => {}
            Pick::OtherCleared => answering.field.set_text("", window, cx),
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

#[cfg(test)]
mod language_tests;
#[cfg(test)]
mod requests_tests;

#[cfg(test)]
impl Answering {
    pub(super) fn request_source(&self, id: QuestionId) -> Option<(vessel_app_kit::FieldId, bool)> {
        (self.id == id).then(|| (self.field.id(), self.sent))
    }
    pub(super) fn request_sending(&self) -> bool {
        self.sent
    }
    pub(super) fn request_sent(&mut self) {
        self.sent = true;
    }
}
