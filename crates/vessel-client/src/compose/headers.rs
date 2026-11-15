//! Header-shaped CLI input is converted using only the canonical question metadata.

use super::{FillError, Given, read};
use std::collections::BTreeMap;
use vessel_proto::conversation::{AnswerValue, ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::{Question, QuestionAnswer};

/// Decode `{answers:{header:text}}` without provider JSON inspection or guessed field ids.
/// Canonical field-keyed `QuestionAnswer` remains available for ambiguous multi-select text.
///
/// # Errors
/// Unknown/ambiguous header, repeated field, unreadable value, or an invalid typed answer.
pub fn fill_headers(
    question: &Question,
    answers: &BTreeMap<String, String>,
) -> Result<QuestionAnswer, FillError> {
    let mut values = BTreeMap::new();
    for (header, text) in answers {
        let mut matching = question.fields.iter().filter(|field| {
            field.name.as_str() == header || field.title.as_deref() == Some(header.as_str())
        });
        let field = matching
            .next()
            .ok_or_else(|| FillError::UnknownField(header.clone()))?;
        if matching.next().is_some() {
            return Err(FillError::AmbiguousHeader(header.clone()));
        }
        let (name, value) = value(field, vessel_proto::trim_ecmascript(text))?;
        if values.insert(name.clone(), value).is_some() {
            return Err(FillError::Repeated(name));
        }
    }
    let answer = QuestionAnswer::Accept { values };
    question.check(&answer).map_err(FillError::Invalid)?;
    Ok(answer)
}

enum ChoiceMatch<'a> {
    Missing,
    Unique(&'a str),
    Ambiguous,
}
fn choice<'a>(options: &'a [ChoiceOption], text: &str) -> ChoiceMatch<'a> {
    let mut matching = options
        .iter()
        .filter(|option| option.value == text || option.title == text);
    let Some(found) = matching.next() else {
        return ChoiceMatch::Missing;
    };
    if matching.next().is_some() {
        return ChoiceMatch::Ambiguous;
    }
    ChoiceMatch::Unique(found.value.as_str())
}

fn value(field: &QuestionField, text: &str) -> Result<(FieldName, AnswerValue), FillError> {
    let unreadable = || FillError::Unreadable {
        field: field.name.clone(),
        expected: "unambiguous offered labels or canonical field-keyed values",
    };
    if let FieldKind::Choice {
        options, multiple, ..
    } = &field.kind
    {
        match choice(options, text) {
            ChoiceMatch::Ambiguous => return Err(unreadable()),
            ChoiceMatch::Unique(selected) => {
                let value = if *multiple {
                    AnswerValue::Choices(vec![selected.to_owned()])
                } else {
                    AnswerValue::Choice(selected.to_owned())
                };
                return Ok((field.name.clone(), value));
            }
            ChoiceMatch::Missing => {}
        }
        if *multiple && text.contains(", ") {
            if options
                .iter()
                .any(|option| option.title.contains(", ") || option.value.contains(", "))
            {
                return Err(unreadable());
            }
            let mut selected = Vec::new();
            let mut missing = false;
            for part in text.split(", ") {
                match choice(options, part) {
                    ChoiceMatch::Ambiguous => return Err(unreadable()),
                    ChoiceMatch::Unique(value) => selected.push(value.to_owned()),
                    ChoiceMatch::Missing => missing = true,
                }
            }
            if !missing {
                return Ok((field.name.clone(), AnswerValue::Choices(selected)));
            }
            if !selected.is_empty() {
                return Err(unreadable());
            }
        }
        if let Some(companion) = field
            .presentation
            .as_ref()
            .and_then(|shown| shown.custom_answer.as_ref())
        {
            return Ok((companion.clone(), AnswerValue::Text(text.to_owned())));
        }
    }
    let value = match read(field, text)? {
        Given::One(value) => value,
        Given::Several(value) => AnswerValue::Choices(vec![value]),
    };
    Ok((field.name.clone(), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Question {
        serde_json::from_str(r#"{"id":"01938000-0000-7000-8000-000000000001","message":"Choose","fields":[{"name":"route","title":"Route","kind":"choice","required":true,"options":[{"value":"opaque-left","title":"Left"}],"presentation":{"custom_answer":"custom","empty":"require_text"}},{"name":"custom","kind":"text"}]}"#).unwrap()
    }

    #[test]
    fn labels_use_real_values_and_custom_text_uses_its_declared_companion() {
        let question = form();
        for (input, key, expected) in [
            ("Left", "route", AnswerValue::Choice("opaque-left".into())),
            ("New route", "custom", AnswerValue::Text("New route".into())),
        ] {
            let answer =
                fill_headers(&question, &BTreeMap::from([("Route".into(), input.into())])).unwrap();
            let QuestionAnswer::Accept { values } = answer else {
                panic!("accept");
            };
            assert_eq!(values[&key.parse::<FieldName>().unwrap()], expected);
        }
    }

    #[test]
    fn duplicate_headers_and_invalid_values_are_refused_without_guessing() {
        let mut question = form();
        question.fields[1].title = Some("Route".into());
        assert!(matches!(
            fill_headers(
                &question,
                &BTreeMap::from([("Route".into(), "Left".into())])
            ),
            Err(FillError::AmbiguousHeader(_))
        ));
        question.fields[1].title = None;
        question.fields[0].presentation = None;
        assert!(matches!(
            fill_headers(
                &question,
                &BTreeMap::from([("Route".into(), "Invented".into())])
            ),
            Err(FillError::Invalid(_))
        ));
    }
    #[test]
    fn header_values_use_the_same_ecmascript_whitespace_as_generic_validation() {
        let question = form();
        let answer = fill_headers(
            &question,
            &BTreeMap::from([("Route".into(), "\u{feff}Left\u{feff}".into())]),
        )
        .unwrap();
        let QuestionAnswer::Accept { values } = answer else {
            panic!("accept")
        };
        assert_eq!(
            values[&"route".parse::<FieldName>().unwrap()],
            AnswerValue::Choice("opaque-left".into())
        );
        let answer = fill_headers(
            &question,
            &BTreeMap::from([("Route".into(), "\u{85}".into())]),
        )
        .unwrap();
        let QuestionAnswer::Accept { values } = answer else {
            panic!("accept")
        };
        assert_eq!(
            values[&"custom".parse::<FieldName>().unwrap()],
            AnswerValue::Text("\u{85}".into())
        );
        assert!(
            fill_headers(
                &question,
                &BTreeMap::from([("Route".into(), "\u{feff}".into())])
            )
            .is_err()
        );
    }
    #[test]
    fn ambiguous_offered_labels_never_fall_back_to_the_custom_companion() {
        for second in [
            ChoiceOption {
                value: "other".to_owned(),
                title: "Left".to_owned(),
                preview: None,
            },
            ChoiceOption {
                value: "Left".to_owned(),
                title: "Different".to_owned(),
                preview: None,
            },
        ] {
            let mut question = form();
            let FieldKind::Choice { options, .. } = &mut question.fields[0].kind else {
                panic!("choice");
            };
            let exact = second.value.clone();
            options.push(second);
            assert!(matches!(
                fill_headers(
                    &question,
                    &BTreeMap::from([("Route".into(), "Left".into())])
                ),
                Err(FillError::Unreadable { .. })
            ));
            let canonical = QuestionAnswer::Accept {
                values: BTreeMap::from([("route".parse().unwrap(), AnswerValue::Choice(exact))]),
            };
            assert!(question.check(&canonical).is_ok());
            let free = fill_headers(
                &question,
                &BTreeMap::from([("Route".into(), "Unlisted custom".into())]),
            )
            .unwrap();
            let QuestionAnswer::Accept { values } = free else {
                panic!("custom");
            };
            assert_eq!(
                values[&"custom".parse::<FieldName>().unwrap()],
                AnswerValue::Text("Unlisted custom".to_owned())
            );
        }
    }
    #[test]
    fn ambiguous_multi_select_parts_are_not_custom_text() {
        let mut question = form();
        let FieldKind::Choice {
            options, multiple, ..
        } = &mut question.fields[0].kind
        else {
            panic!("choice");
        };
        *multiple = true;
        options.push(ChoiceOption {
            value: "other".to_owned(),
            title: "Left".to_owned(),
            preview: None,
        });
        assert!(matches!(
            fill_headers(
                &question,
                &BTreeMap::from([("Route".into(), "Left, Unlisted".into())])
            ),
            Err(FillError::Unreadable { .. })
        ));
    }
}
