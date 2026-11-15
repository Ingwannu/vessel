//! Required choice/custom groups accept either route and preserve legacy forms.

use super::*;
use crate::timeline::{EmptyAnswerPolicy, QuestionFieldPresentation};

fn form() -> Question {
    serde_json::from_str(r#"{"id":"01938000-0000-7000-8000-000000000001","message":"Choose","fields":[{"name":"choice","required":true,"kind":"choice","options":[{"value":"first","title":"First","preview":"Preview"}],"presentation":{"custom_answer":"custom","empty":"require_text"}},{"name":"custom","kind":"text"}]}"#).unwrap()
}

fn answer(values: &[(&str, AnswerValue)]) -> QuestionAnswer {
    QuestionAnswer::Accept {
        values: values
            .iter()
            .map(|(key, value)| (key.parse().unwrap(), value.clone()))
            .collect(),
    }
}

#[test]
fn a_custom_companion_satisfies_required_choice_without_replacing_its_options() {
    let form = form();
    assert!(form.validate_presentation().is_ok());
    assert!(
        form.check(&answer(&[("choice", AnswerValue::Choice("first".into()))]))
            .is_ok()
    );
    assert!(
        form.check(&answer(&[(
            "custom",
            AnswerValue::Text("Another path".into())
        )]))
        .is_ok()
    );
    assert!(
        form.check(&answer(&[("custom", AnswerValue::Text("  ".into()))]))
            .is_err()
    );
    assert!(
        form.check(&answer(&[(
            "choice",
            AnswerValue::Choice("invented".into())
        )]))
        .is_err()
    );
    assert_eq!(form.fields.len(), 2);
}

#[test]
fn empty_policy_is_explicit_and_legacy_text_keeps_its_existing_semantics() {
    let mut form = form();
    form.fields[0].presentation.as_mut().unwrap().empty = EmptyAnswerPolicy::AllowEmpty;
    assert!(
        form.check(&answer(&[("custom", AnswerValue::Text(String::new()))]))
            .is_ok()
    );
    let mut text = form.fields.remove(1);
    text.required = true;
    form.fields = vec![text];
    assert!(
        form.check(&answer(&[("custom", AnswerValue::Text(String::new()))]))
            .is_ok()
    );
    form.fields[0].presentation = Some(QuestionFieldPresentation {
        custom_answer: None,
        placeholder: None,
        empty: EmptyAnswerPolicy::RequireText,
        dismiss_label: None,
    });
    assert!(
        form.check(&answer(&[("custom", AnswerValue::Text(" ".into()))]))
            .is_err()
    );
    form.fields[0].presentation.as_mut().unwrap().empty = EmptyAnswerPolicy::AllowEmpty;
    assert!(
        form.check(&answer(&[("custom", AnswerValue::Text(String::new()))]))
            .is_ok()
    );
    assert!(form.check(&QuestionAnswer::Decline).is_ok());
}

#[test]
fn invalid_companions_are_refused_before_answers_can_have_effects() {
    let mut form = form();
    form.fields[1].required = true;
    assert_eq!(
        form.validate_presentation().unwrap_err().reason,
        QuestionPresentationError::NotOptionalText
    );
    assert!(matches!(
        form.check(&answer(&[("choice", AnswerValue::Choice("first".into()))])),
        Err(InvalidAnswer::Presentation(_))
    ));
    form.fields[1].required = false;
    form.fields[0].presentation.as_mut().unwrap().custom_answer = Some("missing".parse().unwrap());
    assert_eq!(
        form.validate_presentation().unwrap_err().reason,
        QuestionPresentationError::MissingCompanion
    );
    form.fields[0].presentation.as_mut().unwrap().custom_answer = Some("choice".parse().unwrap());
    assert_eq!(
        form.validate_presentation().unwrap_err().reason,
        QuestionPresentationError::MissingCompanion
    );
    let companion = form.fields[1].clone();
    form.fields.push(companion);
    form.fields[0].presentation = None;
    assert_eq!(
        form.validate_presentation().unwrap_err().reason,
        QuestionPresentationError::DuplicateField
    );
}

#[test]
fn generic_custom_and_text_answers_share_ecmascript_blank_semantics() {
    let mut question = form();
    assert!(
        question
            .check(&answer(&[("custom", AnswerValue::Text("\u{feff}".into()))]))
            .is_err()
    );
    assert!(
        question
            .check(&answer(&[("custom", AnswerValue::Text("\u{0085}".into()))]))
            .is_ok()
    );
    let _choice = question.fields.remove(0);
    question.fields[0].required = true;
    question.fields[0].presentation = Some(QuestionFieldPresentation {
        custom_answer: None,
        placeholder: None,
        empty: EmptyAnswerPolicy::RequireText,
        dismiss_label: None,
    });
    assert!(
        question
            .check(&answer(&[("custom", AnswerValue::Text("\u{feff}".into()))]))
            .is_err()
    );
    assert!(
        question
            .check(&answer(&[("custom", AnswerValue::Text("\u{0085}".into()))]))
            .is_ok()
    );
}
