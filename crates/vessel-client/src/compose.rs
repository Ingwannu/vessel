//! What a client sends in a conversation (protocol 10), built from what a
//! person types or an agent gives: an answer to a question from
//! `field=value` texts, and a waiting prompt's content with new text.
//!
//! [Decision Log]
//! - Purpose: `vessel session answer` and the agent tool `answer_question`
//!   both take a question's values untyped (words on a command line, JSON
//!   from an agent); the question's own form says each field's kind.
//!   Editing a waiting prompt from either replaces its words and should
//!   not lose the files sent with it.
//! - Alternatives: each caller builds `AnswerValue`s itself (the same
//!   kind-by-kind reading twice, drifting apart); callers name each type
//!   (`way:choice=left`), which the form already says.
//! - Choice: pure functions here, in the lowest crate both build on. A
//!   value is read by its field's kind only; bounds, options and required
//!   fields are the node's to check (`Question::check`), so the rules are
//!   not written twice and the node's typed refusal is what a caller sees.
//! - Cost: a value of the wrong shape for its field (`count=many`) is
//!   refused here, before any call; one outside the field's bounds only
//!   by the node.

use std::collections::BTreeMap;

use vessel_proto::conversation::{AnswerValue, FieldKind, FieldName, FiniteNumber, QuestionField};
use vessel_proto::{ContentBlock, Question, QuestionAnswer};

/// Why values do not make an answer to a question.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FillError {
    /// A name none of the question's fields has.
    #[error("the question has no field `{0}`")]
    UnknownField(String),
    /// A second value for a field that takes one.
    #[error("field `{0}` takes one value")]
    Repeated(FieldName),
    /// A value its field's kind cannot read.
    #[error("field `{field}` takes {expected}")]
    Unreadable {
        /// Which field.
        field: FieldName,
        /// What it takes, for people.
        expected: &'static str,
    },
    /// A field of a kind this build does not know: the question can only
    /// be declined.
    #[error("field `{0}` is of a kind this client cannot fill; decline the question")]
    Unanswerable(FieldName),
}

/// One value as given, before it is in the answer.
enum Given {
    /// A field that takes one value.
    One(AnswerValue),
    /// One of a field of several choices' values.
    Several(String),
}

/// An accepting answer to `question` from `(field, value)` texts, each
/// read by its field's kind: text as given, numbers and whole numbers in
/// decimal, a boolean as `true`/`yes`/`y`/`1` or `false`/`no`/`n`/`0`, a
/// choice by its option's value. A field of several choices takes one
/// pair per chosen value; any other field one pair.
///
/// # Errors
/// [`FillError`]: a field the question does not have, a value given twice,
/// one its field cannot read, or a field of an unknown kind.
pub fn fill<'a>(
    question: &Question,
    given: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<QuestionAnswer, FillError> {
    let mut values = BTreeMap::new();
    let mut several: BTreeMap<FieldName, Vec<String>> = BTreeMap::new();
    for (name, text) in given {
        let field = question
            .fields
            .iter()
            .find(|field| field.name.as_str() == name)
            .ok_or_else(|| FillError::UnknownField(name.to_owned()))?;
        match read(field, text)? {
            Given::One(value) => {
                if values.insert(field.name.clone(), value).is_some() {
                    return Err(FillError::Repeated(field.name.clone()));
                }
            }
            Given::Several(value) => several.entry(field.name.clone()).or_default().push(value),
        }
    }
    values.extend(
        several
            .into_iter()
            .map(|(name, chosen)| (name, AnswerValue::Choices(chosen))),
    );
    Ok(QuestionAnswer::Accept { values })
}

/// `text` as a value of `field`.
fn read(field: &QuestionField, text: &str) -> Result<Given, FillError> {
    let unreadable = |expected| FillError::Unreadable {
        field: field.name.clone(),
        expected,
    };
    match &field.kind {
        FieldKind::Text { .. } => Ok(Given::One(AnswerValue::Text(text.to_owned()))),
        FieldKind::Number { .. } => text
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(FiniteNumber::new)
            .map(|number| Given::One(AnswerValue::Number(number)))
            .ok_or_else(|| unreadable("a number")),
        FieldKind::Integer { .. } => text
            .trim()
            .parse::<i64>()
            .map(|number| Given::One(AnswerValue::Integer(number)))
            .map_err(|_not_whole| unreadable("a whole number")),
        FieldKind::Boolean { .. } => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "y" | "1" => Ok(Given::One(AnswerValue::Boolean(true))),
            "false" | "no" | "n" | "0" => Ok(Given::One(AnswerValue::Boolean(false))),
            _ => Err(unreadable("yes or no")),
        },
        FieldKind::Choice {
            multiple: false, ..
        } => Ok(Given::One(AnswerValue::Choice(text.to_owned()))),
        FieldKind::Choice { multiple: true, .. } => Ok(Given::Several(text.to_owned())),
        FieldKind::Unknown => Err(FillError::Unanswerable(field.name.clone())),
    }
}

/// A waiting prompt's content with its words replaced by `text`: the
/// files sent with it stay, after the new text, so editing a prompt's
/// words never drops what was attached to it.
#[must_use]
pub fn with_text(content: &[ContentBlock], text: String) -> Vec<ContentBlock> {
    let kept = content.iter().filter(|block| match block {
        ContentBlock::Attachment { .. } => true,
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. } => false,
    });
    std::iter::once(ContentBlock::Text { text })
        .chain(kept.cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use vessel_proto::conversation::ChoiceOption;
    use vessel_proto::{AttachmentId, AttachmentKind, AttachmentRef, QuestionId};

    use super::*;

    fn field(name: &str, kind: FieldKind) -> QuestionField {
        QuestionField {
            name: name.parse().unwrap(),
            title: None,
            description: None,
            required: false,
            kind,

            presentation: None,
        }
    }

    fn choice(multiple: bool) -> FieldKind {
        FieldKind::Choice {
            options: ["left", "right"]
                .map(|value| ChoiceOption {
                    value: value.to_owned(),
                    title: value.to_uppercase(),
                    description: None,

                    preview: None,
                })
                .to_vec(),
            multiple,
            min_items: None,
            max_items: None,
            default: Vec::new(),
        }
    }

    fn question() -> Question {
        Question {
            id: QuestionId::random(),
            message: "Which way?".to_owned(),
            tool_call: None,
            fields: vec![
                field(
                    "name",
                    FieldKind::Text {
                        min_length: None,
                        max_length: None,
                        format: None,
                        pattern: None,
                        default: None,
                    },
                ),
                field(
                    "score",
                    FieldKind::Number {
                        minimum: None,
                        maximum: None,
                        default: None,
                    },
                ),
                field(
                    "count",
                    FieldKind::Integer {
                        minimum: None,
                        maximum: None,
                        default: None,
                    },
                ),
                field("sure", FieldKind::Boolean { default: None }),
                field("way", choice(false)),
                field("ways", choice(true)),
                field("later", FieldKind::Unknown),
            ],

            lifetime: Default::default(),
            delivery_state: Default::default(),
        }
    }

    fn name(text: &str) -> FieldName {
        text.parse().unwrap()
    }

    #[test]
    fn each_value_is_read_by_its_fields_kind() {
        let answer = fill(
            &question(),
            [
                ("name", "a=b c"),
                ("score", " 2.5 "),
                ("count", "-3"),
                ("sure", "Yes"),
                ("way", "left"),
                ("ways", "left"),
                ("ways", "right"),
            ],
        )
        .unwrap();
        let QuestionAnswer::Accept { values } = answer else {
            panic!("an accepting answer");
        };
        assert_eq!(values[&name("name")], AnswerValue::Text("a=b c".to_owned()));
        assert_eq!(
            values[&name("score")],
            AnswerValue::Number(FiniteNumber::new(2.5).unwrap())
        );
        assert_eq!(values[&name("count")], AnswerValue::Integer(-3));
        assert_eq!(values[&name("sure")], AnswerValue::Boolean(true));
        assert_eq!(values[&name("way")], AnswerValue::Choice("left".to_owned()));
        assert_eq!(
            values[&name("ways")],
            AnswerValue::Choices(vec!["left".to_owned(), "right".to_owned()])
        );
        // Nothing given is an empty accept: the node decides what is
        // required.
        assert_eq!(
            fill(&question(), []).unwrap(),
            QuestionAnswer::Accept {
                values: BTreeMap::new()
            }
        );
    }

    #[test]
    fn values_a_field_cannot_take_are_refused_by_name() {
        let refused = |pairs: &[(&'static str, &'static str)]| {
            fill(&question(), pairs.iter().copied()).unwrap_err()
        };
        assert_eq!(
            refused(&[("colour", "red")]),
            FillError::UnknownField("colour".to_owned())
        );
        assert_eq!(
            refused(&[("way", "left"), ("way", "right")]),
            FillError::Repeated(name("way"))
        );
        for (field, value) in [
            ("count", "1.5"),
            ("score", "NaN"),
            ("score", "many"),
            ("sure", "maybe"),
        ] {
            assert!(
                matches!(refused(&[(field, value)]), FillError::Unreadable { field: named, .. } if named == name(field)),
                "{field}={value}"
            );
        }
        assert_eq!(
            refused(&[("later", "x")]),
            FillError::Unanswerable(name("later"))
        );
    }

    #[test]
    fn new_text_keeps_the_prompts_attachments() {
        let attachment = ContentBlock::Attachment {
            attachment: AttachmentRef {
                id: AttachmentId::random(),
                name: "notes.txt".parse().unwrap(),
                kind: AttachmentKind::File,
                mime: "text/plain".parse().unwrap(),
                size: 3,
            },
        };
        let before = vec![
            ContentBlock::Text {
                text: "old".to_owned(),
            },
            attachment.clone(),
            ContentBlock::Text {
                text: "more".to_owned(),
            },
        ];
        assert_eq!(
            with_text(&before, "new".to_owned()),
            vec![
                ContentBlock::Text {
                    text: "new".to_owned()
                },
                attachment
            ]
        );
    }
}
