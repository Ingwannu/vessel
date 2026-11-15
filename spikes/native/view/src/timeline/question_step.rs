//! One step of a question's form as its card shows and edits it: the
//! option rows, whether one is picked, the tab's label and the question's
//! words, the field the text box writes, and what a pick or a keystroke
//! does to the drafts (Paseo's `question-form-card.tsx`, with Claude
//! Code's rule that an option and a custom answer replace each other).
//!
//! [Decision Log]
//! - Purpose: these rules lived in the app's card (`areas/conversation/
//!   question.rs`), untested beside its drawing; the owner's word 44 puts
//!   rules and models in `view` with tests.
//! - Choice: plain functions over a [`Step`] and its [`Drafts`], the
//!   card's handlers calling them; the text box itself (a window's
//!   element) stays the app's, which is told when to empty it.

use vessel_proto::Question;
use vessel_proto::conversation::{FieldKind, QuestionField};

use super::question::{Draft, Drafts, Step};

/// One row of a step's options: what it sends, its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// The value sent when it is picked.
    pub value: String,
    /// Its title.
    pub title: String,
    /// The line under the title, when the agent gave one.
    pub description: Option<String>,
}

/// A step's option rows, and whether several may be picked; a yes-or-no
/// field's two rows read `yes` and `no`.
#[must_use]
pub fn choices(step: Step<'_>, (yes, no): (&str, &str)) -> (Vec<Choice>, bool) {
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
#[must_use]
pub fn picked(step: Step<'_>, drafts: &Drafts, value: &str) -> bool {
    match drafts.get(&step.field.name) {
        Some(Draft::Picked(values)) => values.iter().any(|one| one == value),
        Some(Draft::Boolean(Some(on))) => (value == "true") == *on,
        Some(Draft::Text(_) | Draft::Boolean(None)) | None => false,
    }
}

/// The step's tab label: its header (the field's title), else its name.
#[must_use]
pub fn step_header(step: Step<'_>) -> String {
    step.field
        .title
        .clone()
        .unwrap_or_else(|| step.field.name.as_str().to_owned())
}

/// The step's question: the field's own words when the form has several
/// (Claude puts each question's text there), else the question's message.
#[must_use]
pub fn step_prompt(question: &Question, step: Step<'_>) -> String {
    step.field
        .description
        .clone()
        .unwrap_or_else(|| question.message.clone())
}

/// Whether the step's text box is an "Other" beside options (a choice or a
/// yes-or-no field), not the step's own answer.
#[must_use]
pub const fn box_is_other(step: Step<'_>) -> bool {
    matches!(
        step.field.kind,
        FieldKind::Choice { .. } | FieldKind::Boolean { .. }
    )
}

/// The field a step's text box writes: Claude's "Other", or a free-text
/// step's own field; `None` when the step has no box.
#[must_use]
pub fn text_target(step: Step<'_>) -> Option<&QuestionField> {
    match &step.field.kind {
        FieldKind::Choice { .. } | FieldKind::Boolean { .. } => step.other,
        FieldKind::Text { .. } | FieldKind::Number { .. } | FieldKind::Integer { .. } => {
            Some(step.field)
        }
        FieldKind::Unknown => None,
    }
}

/// The step's text box now holds `text`: its field takes it, and for a
/// single choice a non-empty text clears the pick (an option and a custom
/// answer replace each other). Nothing when the step has no box.
pub fn typed(step: Step<'_>, drafts: &mut Drafts, text: String) {
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
        let _was = drafts.insert(step.field.name.clone(), cleared);
    }
    let _was = drafts.insert(target.name.clone(), Draft::Text(text));
}

/// What a pick did, for the text box the app keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// The step has no options: nothing changed.
    Ignored,
    /// The drafts changed; the box keeps its text.
    Picked,
    /// A single choice was picked: its "Other" was emptied, and so must
    /// the box be.
    OtherCleared,
}

/// Pick `value` in `step`: a radio's or a yes-or-no field's one value
/// (picking it again clears it), a checkbox's toggle. Picking a single
/// choice empties its "Other".
pub fn pick(step: Step<'_>, drafts: &mut Drafts, value: &str) -> Pick {
    let was = picked(step, drafts, value);
    let draft = match &step.field.kind {
        FieldKind::Boolean { .. } => Draft::Boolean((!was).then_some(value == "true")),
        FieldKind::Choice { multiple: true, .. } => {
            let mut values = match drafts.get(&step.field.name) {
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
        | FieldKind::Unknown => return Pick::Ignored,
    };
    let _was = drafts.insert(step.field.name.clone(), draft);
    let single = !matches!(step.field.kind, FieldKind::Choice { multiple: true, .. });
    if single
        && !was
        && let Some(other) = step.other
    {
        let _was = drafts.insert(other.name.clone(), Draft::Text(String::new()));
        return Pick::OtherCleared;
    }
    Pick::Picked
}
