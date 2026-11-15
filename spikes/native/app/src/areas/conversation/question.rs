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
//!   answer..."). Each pending question owns one text box for its steps (text and
//!   placeholder change with the step), made the frame after the question
//!   arrives (`defer_in`, the slot cannot make it). Dismiss declines; the
//!   answer is checked by the node's own check (`Question::check`) before
//!   it goes, and a refusal (another client answered first, a value the
//!   node will not take) is said under the card with the node's words.
//! - Differences, kept: no option previews (Claude's `preview`
//!   reaches the node only under `_meta`, which the node does not keep);
//!   the option rows scroll inside the card past 360 (280 on a phone), where Paseo's card
//!   scrolls with the conversation.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{TextField, field::FieldId};
use vessel_app_view::timeline::{
    Draft, EntryKind, Pick, Step, answer_of, box_is_other, empty_drafts, pick, refused_field,
    step_after_pick, step_ready, steps_of, text_target, typed,
};
use vessel_proto::{Question, QuestionAnswer, QuestionId};

use super::{Answer, ConversationArea, Place, Seat, SessionPlace};
use crate::shell::Shell;

mod card;

pub(crate) use card::{card, card_at};

mod forms;
pub(crate) use forms::Forms;
use forms::{Answering, Cursor, ResponseAction};

/// Every pending question in log order, then requests whose log record was pruned.
fn waiting(area: &ConversationArea) -> Vec<&Question> {
    area.timeline
        .waiting_questions()
        .into_iter()
        .filter_map(|entry| {
            if let EntryKind::Question { question, .. } = &entry.kind {
                Some(question)
            } else {
                None
            }
        })
        .chain(area.timeline.unseen_questions(&area.live))
        .collect()
}

fn waiting_at<P: Place>(shell: &Shell, id: QuestionId) -> Option<Question> {
    waiting((P::SEAT.get)(shell)?)
        .into_iter()
        .find(|question| question.id == id)
        .cloned()
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
    /// A submitted answer owns the form until its reply or inline refusal;
    /// queued field callbacks must not change the answer already in flight.
    fn question_editable<P: Place>(&self, id: QuestionId) -> bool {
        (P::SEAT.get)(self).is_some_and(|area| {
            area.answering.get(id).is_some_and(|answering| {
                answering.sending.is_none()
                    || area
                        .response_failures
                        .get(vessel_app_view::responses::Request::Question(answering.id))
                        .is_some()
            })
        }) && !P::busy(self)
    }

    /// Old input subscriptions can outlive the request that owned the engine.
    fn question_source<P: Place>(&self, source: FieldId) -> Option<QuestionId> {
        let area = (P::SEAT.get)(self)?;
        let id = area.answering.source(source)?;
        waiting(area)
            .into_iter()
            .any(|question| question.id == id)
            .then_some(id)
    }

    /// A queued click belongs to the request and step drawn for it.
    fn question_cursor<P: Place>(&self, cursor: Cursor) -> bool {
        (P::SEAT.get)(self)
            .and_then(|area| area.answering.get(cursor.id))
            .is_some_and(|form| {
                form.cursor() == cursor && waiting_at::<P>(self, cursor.id).is_some()
            })
    }

    /// Update only this session's retained step hint; other places own theirs.
    pub(crate) fn refresh_question_words(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let questions = (Seat::SESSION.get)(self)
            .map(|area| waiting(area).into_iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let words = self.t.lang.conversation();
        for question in questions {
            let steps = steps_of(&question);
            if let Some(form) =
                (Seat::SESSION.get_mut)(self).and_then(|area| area.answering.get_mut(question.id))
                && let Some(step) = steps.get(form.step)
            {
                form.field
                    .set_placeholder(field_placeholder(*step, *words), window, cx);
            }
        }
    }

    /// Reconcile local form resources with the authoritative pending identities.
    fn question_prepare<P: Place>(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(area) = (P::SEAT.get)(self) else {
            return;
        };
        let pending = waiting(area);
        let ids = pending.iter().map(|question| question.id).collect();
        let missing: Vec<_> = pending
            .into_iter()
            .filter(|question| area.answering.get(question.id).is_none())
            .cloned()
            .collect();
        if let Some(area) = (P::SEAT.get_mut)(self) {
            area.answering.retain(&ids);
        }
        for question in missing {
            let words = self.t.lang.conversation();
            let field = TextField::with_source(
                words.question_other_placeholder(),
                Self::question_entered::<P>,
                Self::question_typed::<P>,
                window,
                cx,
            );
            if let Some(area) = (P::SEAT.get_mut)(self) {
                area.answering.insert(Answering {
                    id: question.id,
                    drafts: empty_drafts(&question),
                    step: 0,
                    field,
                    sending: None,
                    validation: None,
                });
            }
            self.question_show_at::<P>(question.id, 0, window, cx);
        }
        cx.notify();
    }

    /// Show step `index`: its text box holds that step's text.
    fn question_show_at<P: Place>(
        &mut self,
        id: QuestionId,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.question_editable::<P>(id) {
            return;
        }
        let words = self.t.lang.conversation();
        let Some(question) = waiting_at::<P>(self, id) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.get_mut(id))
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
    fn question_typed<P: Place>(
        &mut self,
        source: FieldId,
        text: String,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(id) = self.question_source::<P>(source) else {
            return;
        };
        if !self.question_editable::<P>(id) {
            return;
        }
        let Some(question) = waiting_at::<P>(self, id) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.get_mut(id))
        else {
            return;
        };
        let Some(step) = steps.get(answering.step).copied() else {
            return;
        };
        typed(step, &mut answering.drafts, text);
        answering.validation = None;
        cx.notify();
    }

    fn question_entered<P: Place>(
        &mut self,
        source: FieldId,
        _text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(id) = self.question_source::<P>(source) {
            self.question_primary_at::<P>(id, window, cx);
        }
    }

    /// Pick `value` in the step shown: a radio's or a yes-or-no field's
    /// one value (picking it again clears it), a checkbox's toggle.
    fn question_pick_at<P: Place>(
        &mut self,
        id: QuestionId,
        value: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.question_editable::<P>(id) {
            return;
        }
        let Some(question) = waiting_at::<P>(self, id) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get_mut)(self).and_then(|area| area.answering.get_mut(id))
        else {
            return;
        };
        let Some(step) = steps.get(answering.step).copied() else {
            return;
        };
        answering.validation = None;
        match pick(step, &mut answering.drafts, value) {
            Pick::Ignored => return,
            Pick::Picked => {}
            Pick::OtherCleared => answering.field.set_text("", window, cx),
        }
        if let Some(next) = step_after_pick(&steps, answering.step, &answering.drafts) {
            self.question_show_at::<P>(id, next, window, cx);
        }
        cx.notify();
    }

    /// Next, or Submit on the last step.
    fn question_primary_at<P: Place>(
        &mut self,
        id: QuestionId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.question_editable::<P>(id) {
            return;
        }
        let Some(question) = waiting_at::<P>(self, id) else {
            return;
        };
        let steps = steps_of(&question);
        let Some(answering) = (P::SEAT.get)(self).and_then(|area| area.answering.get(id)) else {
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
            self.question_show_at::<P>(id, next, window, cx);
            return;
        }
        if let Some(unready) = steps
            .iter()
            .position(|step| !step_ready(*step, &answering.drafts))
        {
            self.question_show_at::<P>(id, unready, window, cx);
            return;
        }
        let answer = answer_of(&question, &answering.drafts);
        if let Err(refused) = question.check(&answer) {
            let message = refused.to_string();
            let at = refused_field(&question, &refused).and_then(|name| {
                steps.iter().position(|step| {
                    step.field.name == *name || step.other.is_some_and(|other| other.name == *name)
                })
            });
            if let Some(area) = (P::SEAT.get_mut)(self) {
                if let Some(form) = area.answering.get_mut(id) {
                    form.validation = Some(message);
                }
            }
            if let Some(at) = at {
                self.question_show_at::<P>(id, at, window, cx);
            }
            cx.notify();
            return;
        }
        self.question_send::<P>(question.id, answer, ResponseAction::Submit, cx);
    }

    /// Decline the question.
    fn question_dismiss_at<P: Place>(&mut self, id: QuestionId, cx: &mut Context<'_, Self>) {
        if !self.question_editable::<P>(id) {
            return;
        }
        if waiting_at::<P>(self, id).is_some() {
            self.question_send::<P>(id, QuestionAnswer::Decline, ResponseAction::Dismiss, cx);
        }
    }

    fn question_send<P: Place>(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
        action: ResponseAction,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(area) = (P::SEAT.get_mut)(self) {
            if let Some(answering) = area.answering.get_mut(id) {
                // Dismiss may legitimately send an empty accepted answer;
                // progress belongs to the pressed action, not its encoding.
                answering.sending = Some(action);
                answering.validation = None;
            }
            if let Some(answering) = area.answering.get(id) {
                answering
                    .field
                    .set_editability(vessel_app_kit::field::Editability::ReadOnly, cx);
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
mod compat_tests;

#[cfg(test)]
mod multi_tests;

#[cfg(test)]
mod residency_tests;
