//! A question an agent asks (protocol 10's form), as the app answers it
//! above the composer: its fields as steps, what each holds while the
//! person answers, what the node would refuse, the answer sent, and a
//! settled question's answers as the timeline lists them. The page's
//! `data/question-form.ts` and `data/question-answers.ts` (`backend/m11`,
//! `67bf243`), case for case; their tests are replayed in
//! `tests/page/question.rs`.
//!
//! [Decision Log]
//! - Purpose: Claude's `AskUserQuestion` (and any agent's form) answered
//!   in the session, a step at a time as Paseo's question card walks it
//!   (`components/question-form-card.tsx`: a tab per question, its
//!   options, an "Other" box, Dismiss and Next or Submit).
//! - Constraint: Claude's adapter sends each question as a choice
//!   `question_N` (titled by the question's header, described by its
//!   text when there are several) followed by a free-text
//!   `question_N_custom`, nothing required; the node keeps a form's fields
//!   by key, so `question_10` comes before `question_2`. The node checks
//!   every answer (`Question::check`) and refuses one that does not fit.
//! - Choice: the page's: a step per field, Claude's pairs as one step with
//!   the companion as its Other box, `question_N` ordered by N; values
//!   kept as typed (a number as its text) and read into the node's types
//!   only when sent, with the node's checks run here first so a refusal is
//!   rare and one that comes names its field. Why a value does not fit is
//!   a [`Wrong`], put in words by the area, not a sentence here.

use std::collections::BTreeMap;

mod trim;
use trim::answer_text;
mod validation;
pub use validation::{answerable, refused_field};

use vessel_proto::conversation::{AnswerValue, FieldKind, FieldName, FiniteNumber, QuestionField};
use vessel_proto::{Question, QuestionAnswer, QuestionOutcome};

/// One step: a field, and Claude's free-text companion when it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step<'q> {
    /// The field.
    pub field: &'q QuestionField,
    /// Its `_custom` companion (Claude's "Other").
    pub other: Option<&'q QuestionField>,
}

/// What one field holds while the person answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draft {
    /// Text as typed (a number field's too).
    Text(String),
    /// The options picked, by value, in the order picked.
    Picked(Vec<String>),
    /// Yes, no, or neither yet.
    Boolean(Option<bool>),
}

/// Every field's draft, by name.
pub type Drafts = BTreeMap<FieldName, Draft>;

/// Why a value does not fit its field (the node's checks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrong {
    /// A field of a kind this build does not know.
    Unanswerable,
    /// Longer than the node takes.
    TooLong,
    /// Fewer characters than the field's least.
    AtLeastChars(u32),
    /// More characters than the field's most.
    AtMostChars(u32),
    /// Not a number.
    NotANumber,
    /// A fraction where a whole number is wanted.
    WholeNumber,
    /// Under the least, as the field gives it.
    AtLeast(String),
    /// Over the most, as the field gives it.
    AtMost(String),
    /// Fewer picks than the least.
    ChooseAtLeast(u32),
    /// More picks than the most.
    ChooseAtMost(u32),
}

/// A field's value as the node takes it, none, or why it does not fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Read {
    /// A value.
    Value(AnswerValue),
    /// Nothing given.
    Empty,
    /// Something given that does not fit.
    Wrong(Wrong),
}

/// Why a step cannot be left yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A value that does not fit.
    Wrong(Wrong),
    /// A required choice with nothing picked.
    ChooseAnAnswer,
    /// A required field with nothing in it.
    Required,
}

/// A settled question's answer to one step: the field's title, the value
/// in the agent's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsweredLine {
    /// The field's title (its name when it has none).
    pub field: String,
    /// The value: a choice by its option's title, Claude's free text
    /// after its question's picks.
    pub value: String,
}

/// The number `N` of Claude's `question_N`.
fn claude_number(name: &FieldName) -> Option<u64> {
    let digits = name.as_str().strip_prefix("question_")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// `question`'s steps, in the order they are shown.
#[must_use]
pub fn steps_of(question: &Question) -> Vec<Step<'_>> {
    let companion_of = |field: &QuestionField| -> Option<&QuestionField> {
        let _numbered = claude_number(&field.name)?;
        let name = format!("{}_custom", field.name.as_str());
        question
            .fields
            .iter()
            .find(|candidate| candidate.name.as_str() == name)
    };
    let companions: Vec<&FieldName> = question
        .fields
        .iter()
        .filter_map(companion_of)
        .map(|other| &other.name)
        .collect();
    let steps: Vec<Step<'_>> = question
        .fields
        .iter()
        .filter(|field| !companions.contains(&&field.name))
        .map(|field| Step {
            field,
            other: companion_of(field),
        })
        .collect();
    // Claude's numbered questions by their number; anything else keeps the
    // node's place.
    let mut numbered: Vec<Step<'_>> = steps
        .iter()
        .filter(|step| claude_number(&step.field.name).is_some())
        .copied()
        .collect();
    numbered.sort_by_key(|step| claude_number(&step.field.name));
    let mut next = numbered.into_iter();
    steps
        .into_iter()
        .map(|step| {
            if claude_number(&step.field.name).is_none() {
                step
            } else {
                next.next().unwrap_or(step)
            }
        })
        .collect()
}

/// Each field's starting draft: its default where it has one.
#[must_use]
pub fn starting_drafts(question: &Question) -> Drafts {
    question
        .fields
        .iter()
        .map(|field| (field.name.clone(), starting_draft(field)))
        .collect()
}

fn starting_draft(field: &QuestionField) -> Draft {
    match &field.kind {
        FieldKind::Text { default, .. } => Draft::Text(default.clone().unwrap_or_default()),
        FieldKind::Number { default, .. } => Draft::Text(
            default
                .map(|number| number.get().to_string())
                .unwrap_or_default(),
        ),
        FieldKind::Integer { default, .. } => {
            Draft::Text(default.map(|number| number.to_string()).unwrap_or_default())
        }
        FieldKind::Boolean { default, .. } => Draft::Boolean(*default),
        FieldKind::Choice { default, .. } => Draft::Picked(default.clone()),
        FieldKind::Unknown => Draft::Text(String::new()),
    }
}

/// What `draft` holds for `field`.
#[must_use]
pub fn read_field(field: &QuestionField, draft: Option<&Draft>) -> Read {
    let text = match draft {
        Some(Draft::Text(text)) => text.as_str(),
        _ => "",
    };
    match &field.kind {
        FieldKind::Unknown => Read::Wrong(Wrong::Unanswerable),
        FieldKind::Text {
            min_length,
            max_length,
            ..
        } => read_text(text, *min_length, *max_length),
        FieldKind::Number {
            minimum, maximum, ..
        } => read_number(answer_text(text), *minimum, *maximum),
        FieldKind::Integer {
            minimum, maximum, ..
        } => read_integer(answer_text(text), *minimum, *maximum),
        FieldKind::Boolean { .. } => match draft {
            Some(Draft::Boolean(Some(on))) => Read::Value(AnswerValue::Boolean(*on)),
            _ => Read::Empty,
        },
        FieldKind::Choice {
            multiple,
            min_items,
            max_items,
            ..
        } => {
            let picked: &[String] = match draft {
                Some(Draft::Picked(values)) => values,
                _ => &[],
            };
            read_choice(picked, *multiple, *min_items, *max_items)
        }
    }
}

fn read_text(text: &str, min_length: Option<u32>, max_length: Option<u32>) -> Read {
    let text = answer_text(text);
    if text.is_empty() {
        return Read::Empty;
    }
    if text.len() > Question::MAX_TEXT_ANSWER {
        return Read::Wrong(Wrong::TooLong);
    }
    let length = text.chars().count();
    if let Some(least) = min_length
        && length < usize::try_from(least).unwrap_or(usize::MAX)
    {
        return Read::Wrong(Wrong::AtLeastChars(least));
    }
    if let Some(most) = max_length
        && length > usize::try_from(most).unwrap_or(usize::MAX)
    {
        return Read::Wrong(Wrong::AtMostChars(most));
    }
    Read::Value(AnswerValue::Text(text.to_owned()))
}

/// Whether `text` is a number as the page takes one (`-?\d*\.?\d+`): a
/// sign, digits, at most one point, a digit last.
fn numeric(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    match unsigned.split_once('.') {
        Some((whole, fraction)) => digits(whole) && !fraction.is_empty() && digits(fraction),
        None => !unsigned.is_empty() && digits(unsigned),
    }
}

fn read_number(text: &str, minimum: Option<FiniteNumber>, maximum: Option<FiniteNumber>) -> Read {
    if text.is_empty() {
        return Read::Empty;
    }
    let Some(number) = numeric(text)
        .then(|| text.parse::<f64>().ok())
        .flatten()
        .and_then(FiniteNumber::new)
    else {
        return Read::Wrong(Wrong::NotANumber);
    };
    if let Some(least) = minimum
        && number.get() < least.get()
    {
        return Read::Wrong(Wrong::AtLeast(least.get().to_string()));
    }
    if let Some(most) = maximum
        && number.get() > most.get()
    {
        return Read::Wrong(Wrong::AtMost(most.get().to_string()));
    }
    Read::Value(AnswerValue::Number(number))
}

fn read_integer(text: &str, minimum: Option<i64>, maximum: Option<i64>) -> Read {
    if text.is_empty() {
        return Read::Empty;
    }
    if !numeric(text) {
        return Read::Wrong(Wrong::NotANumber);
    }
    // A point is a whole number only with nothing but zeros after it
    // (the page's `Number.isInteger`: "2.0" is 2).
    let whole = match text.split_once('.') {
        Some((whole, fraction)) if fraction.bytes().all(|byte| byte == b'0') => whole,
        Some(_) => return Read::Wrong(Wrong::WholeNumber),
        None => text,
    };
    let whole = match whole {
        "" | "-" => "0",
        whole => whole,
    };
    let Ok(number) = whole.parse::<i64>() else {
        return Read::Wrong(Wrong::NotANumber);
    };
    if let Some(least) = minimum
        && number < least
    {
        return Read::Wrong(Wrong::AtLeast(least.to_string()));
    }
    if let Some(most) = maximum
        && number > most
    {
        return Read::Wrong(Wrong::AtMost(most.to_string()));
    }
    Read::Value(AnswerValue::Integer(number))
}

fn read_choice(
    picked: &[String],
    multiple: bool,
    min_items: Option<u32>,
    max_items: Option<u32>,
) -> Read {
    if !multiple {
        return picked.first().map_or(Read::Empty, |first| {
            Read::Value(AnswerValue::Choice(first.clone()))
        });
    }
    if picked.is_empty() {
        return Read::Empty;
    }
    if let Some(least) = min_items
        && picked.len() < usize::try_from(least).unwrap_or(usize::MAX)
    {
        return Read::Wrong(Wrong::ChooseAtLeast(least));
    }
    if let Some(most) = max_items
        && picked.len() > usize::try_from(most).unwrap_or(usize::MAX)
    {
        return Read::Wrong(Wrong::ChooseAtMost(most));
    }
    Read::Value(AnswerValue::Choices(picked.to_vec()))
}

/// Why `step` cannot be left yet, or `None` when it can: a value that does
/// not fit, or a required field with none (Claude's pair counts as
/// answered when either half is).
#[must_use]
pub fn step_problem(step: Step<'_>, drafts: &Drafts) -> Option<Problem> {
    let read = read_field(step.field, drafts.get(&step.field.name));
    let other = step
        .other
        .map(|other| read_field(other, drafts.get(&other.name)));
    if let Read::Wrong(why) = read {
        return Some(Problem::Wrong(why));
    }
    if let Some(Read::Wrong(why)) = other {
        return Some(Problem::Wrong(why));
    }
    let other_answers = matches!(other, Some(Read::Value(_)));
    if read == Read::Empty && step.field.required && !other_answers {
        return Some(if matches!(step.field.kind, FieldKind::Choice { .. }) {
            Problem::ChooseAnAnswer
        } else {
            Problem::Required
        });
    }
    None
}

/// Whether `step` holds an answer (a value in it or in its companion):
/// Paseo's tab check.
#[must_use]
pub fn step_answered(step: Step<'_>, drafts: &Drafts) -> bool {
    let answers = |field: &QuestionField| {
        matches!(read_field(field, drafts.get(&field.name)), Read::Value(_))
    };
    answers(step.field) || step.other.is_some_and(answers)
}

/// Whether the person can leave `step` (Next, or Submit on the last):
/// nothing in it that does not fit, nothing required missing, and one of
/// Claude's questions answered: Paseo enables its buttons only then,
/// though Claude's form marks nothing required. Any other agent's form is
/// held to its own `required`.
#[must_use]
pub fn step_ready(step: Step<'_>, drafts: &Drafts) -> bool {
    step_problem(step, drafts).is_none()
        && (claude_number(&step.field.name).is_none() || step_answered(step, drafts))
}

/// The answer `drafts` make: every field that holds a value.
#[must_use]
pub fn answer_of(question: &Question, drafts: &Drafts) -> QuestionAnswer {
    let values = question
        .fields
        .iter()
        .filter_map(|field| match read_field(field, drafts.get(&field.name)) {
            Read::Value(value) => Some((field.name.clone(), value)),
            Read::Empty | Read::Wrong(_) => None,
        })
        .collect();
    QuestionAnswer::Accept { values }
}

/// The answers of `outcome`, in the steps' order; empty for a question
/// declined or withdrawn. `yes` and `no` are a boolean's words.
#[must_use]
pub fn answered_lines(
    question: &Question,
    outcome: &QuestionOutcome,
    (yes, no): (&str, &str),
) -> Vec<AnsweredLine> {
    let QuestionOutcome::Answered { values, .. } = outcome else {
        return Vec::new();
    };
    steps_of(question)
        .into_iter()
        .filter_map(|step| {
            let parts: Vec<String> = std::iter::once(step.field)
                .chain(step.other)
                .filter_map(|part| {
                    let text = value_text(part, values.get(&part.name)?, (yes, no));
                    (!text.is_empty()).then_some(text)
                })
                .collect();
            (!parts.is_empty()).then(|| AnsweredLine {
                field: step
                    .field
                    .title
                    .clone()
                    .unwrap_or_else(|| step.field.name.as_str().to_owned()),
                value: parts.join(", "),
            })
        })
        .collect()
}

/// A value in the agent's words: a choice by its option's title.
fn value_text(field: &QuestionField, value: &AnswerValue, (yes, no): (&str, &str)) -> String {
    let title_of = |picked: &str| -> String {
        match &field.kind {
            FieldKind::Choice { options, .. } => options
                .iter()
                .find(|option| option.value == picked)
                .map_or_else(|| picked.to_owned(), |option| option.title.clone()),
            FieldKind::Text { .. }
            | FieldKind::Number { .. }
            | FieldKind::Integer { .. }
            | FieldKind::Boolean { .. }
            | FieldKind::Unknown => picked.to_owned(),
        }
    };
    match value {
        AnswerValue::Text(text) => text.clone(),
        AnswerValue::Number(number) => number.get().to_string(),
        AnswerValue::Integer(number) => number.to_string(),
        AnswerValue::Boolean(on) => if *on { yes } else { no }.to_owned(),
        AnswerValue::Choice(picked) => title_of(picked),
        AnswerValue::Choices(picked) => picked
            .iter()
            .map(|one| title_of(one))
            .collect::<Vec<_>>()
            .join(", "),
    }
}
