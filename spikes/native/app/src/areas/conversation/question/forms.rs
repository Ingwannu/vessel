//! Real input resources belong to each exact pending question, never its position.
use std::collections::{BTreeMap, BTreeSet};
use vessel_app_kit::{TextField, field::FieldId};
use vessel_app_view::timeline::Drafts;
use vessel_proto::QuestionId;

/// A question being answered: what each field holds and the step shown.
pub(crate) struct Answering {
    pub(super) id: QuestionId,
    pub(super) drafts: Drafts,
    pub(super) step: usize,
    /// The step's text box: a free-text step's answer, or a choice's
    /// "Other".
    pub(super) field: TextField,
    /// The answer went; the node has not settled the question yet.
    pub(super) sending: Option<ResponseAction>,
    pub(super) validation: Option<String>,
}

impl Answering {
    /// A late reply may settle only the retained form that sent it.
    fn is_sending(&self, id: QuestionId) -> bool {
        self.id == id && self.sending.is_some()
    }
}

/// Render-time identity prevents clicks from a replaced request or step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cursor {
    pub(super) id: QuestionId,
    pub(super) step: usize,
    /// Same step reopened later is a different displayed input visit.
    pub(super) source: FieldId,
}

impl Answering {
    pub(super) fn cursor(&self) -> Cursor {
        Cursor {
            id: self.id,
            step: self.step,
            source: self.field.id(),
        }
    }
}

/// Submit and dismiss share a request but show progress on their own button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResponseAction {
    Submit,
    Dismiss,
}

impl std::fmt::Debug for Answering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answering")
            .field("id", &self.id)
            .field("step", &self.step)
            .field("sending", &self.sending)
            .finish_non_exhaustive()
    }
}

/// Draft/input resources only; pending-request authority stays in timeline/live.
#[derive(Debug, Default)]
pub(crate) struct Forms(BTreeMap<QuestionId, Answering>);
impl Forms {
    pub(super) fn get(&self, id: QuestionId) -> Option<&Answering> {
        self.0.get(&id)
    }
    pub(super) fn get_mut(&mut self, id: QuestionId) -> Option<&mut Answering> {
        self.0.get_mut(&id)
    }
    pub(super) fn insert(&mut self, form: Answering) {
        let _previous = self.0.insert(form.id, form);
    }
    pub(super) fn retain(&mut self, ids: &BTreeSet<QuestionId>) {
        self.0.retain(|id, _| ids.contains(id));
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn source(&self, source: FieldId) -> Option<QuestionId> {
        self.0
            .values()
            .find(|form| form.field.id() == source)
            .map(|form| form.id)
    }
    pub(crate) fn is_sending(&self, id: QuestionId) -> bool {
        self.get(id).is_some_and(|form| form.is_sending(id))
    }
    // Single-form fixtures keep their existing input probe; production always names an id.
    #[cfg(test)]
    pub(super) fn as_ref(&self) -> Option<&Answering> {
        self.0.values().next()
    }
}
