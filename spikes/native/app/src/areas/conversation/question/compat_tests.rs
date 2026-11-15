//! Existing single-form fixtures route through the same explicit-id controller.
use super::*;
impl Shell {
    fn first_question<P: Place>(&self) -> Option<QuestionId> {
        waiting((P::SEAT.get)(self)?)
            .first()
            .map(|question| question.id)
    }
    pub(super) fn question_show<P: Place>(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(id) = self.first_question::<P>() {
            self.question_show_at::<P>(id, index, window, cx);
        }
    }
    pub(super) fn question_pick<P: Place>(
        &mut self,
        value: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(id) = self.first_question::<P>() {
            self.question_pick_at::<P>(id, value, window, cx);
        }
    }
}
