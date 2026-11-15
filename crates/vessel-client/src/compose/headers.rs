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
        let (name, value) = value(field, text.trim())?;
        if values.insert(name.clone(), value).is_some() {
            return Err(FillError::Repeated(name));
        }
    }
    let answer = QuestionAnswer::Accept { values };
    question.check(&answer).map_err(FillError::Invalid)?;
    Ok(answer)
}

fn choice<'a>(options: &'a [ChoiceOption], text: &str) -> Option<&'a str> {
    let mut matching = options
        .iter()
        .filter(|option| option.value == text || option.title == text);
    let found = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    Some(found.value.as_str())
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
        if let Some(selected) = choice(options, text) {
            let value = if *multiple {
                AnswerValue::Choices(vec![selected.to_owned()])
            } else {
                AnswerValue::Choice(selected.to_owned())
            };
            return Ok((field.name.clone(), value));
        }
        if *multiple && text.contains(", ") {
            if options
                .iter()
                .any(|option| option.title.contains(", ") || option.value.contains(", "))
            {
                return Err(unreadable());
            }
            let selected: Option<Vec<_>> = text
                .split(", ")
                .map(|part| choice(options, part).map(str::to_owned))
                .collect();
            if let Some(selected) = selected {
                return Ok((field.name.clone(), AnswerValue::Choices(selected)));
            }
            if text.split(", ").any(|part| choice(options, part).is_some()) {
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
}
