//! Selection must navigate without bypassing explicit final submission.

use super::*;
use crate::timeline::{answer_of, starting_drafts, steps_of};
use serde_json::json;

fn question(multiple: bool) -> Question {
    serde_json::from_value(json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "message": "Choose a strategy",
        "fields": [
            {"name":"first", "required":true, "kind":"choice", "multiple":multiple,
             "options":[{"value":"a", "title":"A"}, {"value":"b", "title":"B"}]},
            {"name":"last", "required":true, "kind":"text"}
        ]
    }))
    .unwrap()
}

#[test]
fn single_choice_advances_and_deselection_stays() {
    let question = question(false);
    let steps = steps_of(&question);
    let mut drafts = starting_drafts(&question);
    assert_eq!(step_after_pick(&steps, 0, &drafts), None);
    assert_eq!(pick(steps[0], &mut drafts, "a"), Pick::Picked);
    assert_eq!(step_after_pick(&steps, 0, &drafts), Some(1));
    assert_eq!(pick(steps[0], &mut drafts, "a"), Pick::Picked);
    assert_eq!(step_after_pick(&steps, 0, &drafts), None);
}

#[test]
fn checkbox_and_last_question_never_auto_submit() {
    let question = question(true);
    let steps = steps_of(&question);
    let mut drafts = starting_drafts(&question);
    assert_eq!(pick(steps[0], &mut drafts, "b"), Pick::Picked);
    assert_eq!(step_after_pick(&steps, 0, &drafts), None);
    typed(steps[1], &mut drafts, "An explicit answer".to_owned());
    assert_eq!(step_after_pick(&steps, 1, &drafts), None);
    assert_eq!(step_after_pick(&steps, usize::MAX, &drafts), None);
}

#[test]
fn offered_schema_defaults_do_not_preanswer_the_native_card() {
    let question: Question = serde_json::from_value(json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "message": "Choose explicitly",
        "fields": [
            {"name":"choice", "kind":"choice", "required":true,
             "options":[{"value":"a","title":"A"}], "default":["a"]},
            {"name":"text", "kind":"text", "default":"suggested"},
            {"name":"boolean", "kind":"boolean", "default":true}
        ]
    }))
    .unwrap();
    let drafts = empty_drafts(&question);
    assert_eq!(
        drafts.get(&"choice".parse().unwrap()),
        Some(&Draft::Picked(Vec::new()))
    );
    assert_eq!(
        drafts.get(&"text".parse().unwrap()),
        Some(&Draft::Text(String::new()))
    );
    assert_eq!(
        drafts.get(&"boolean".parse().unwrap()),
        Some(&Draft::Boolean(None))
    );
    let steps = steps_of(&question);
    assert!(!step_ready(steps[0], &drafts));
    assert!(!crate::timeline::step_answered(steps[0], &drafts));
}

#[test]
fn whitespace_is_not_an_answer_and_encoding_trims_without_changing_the_draft() {
    let question = question(false);
    let steps = steps_of(&question);
    let mut drafts = empty_drafts(&question);
    for blank in [" \t\r\n", "\u{feff}\u{2003}"] {
        typed(steps[1], &mut drafts, blank.to_owned());
        assert!(!step_ready(steps[1], &drafts));
    }
    let raw = " \t useful answer\n";
    typed(steps[1], &mut drafts, raw.to_owned());
    assert!(step_ready(steps[1], &drafts));
    assert_eq!(
        drafts.get(&"last".parse().unwrap()),
        Some(&Draft::Text(raw.to_owned()))
    );
    let vessel_proto::QuestionAnswer::Accept { values } = answer_of(&question, &drafts) else {
        panic!("accepted answer")
    };
    assert_eq!(
        values.get(&"last".parse().unwrap()),
        Some(&vessel_proto::conversation::AnswerValue::Text(
            "useful answer".to_owned()
        ))
    );
}

#[test]
fn an_unoffered_queued_option_cannot_change_drafts_or_advance() {
    for multiple in [false, true] {
        let question = question(multiple);
        let steps = steps_of(&question);
        let mut drafts = empty_drafts(&question);
        let before = drafts.clone();
        assert_eq!(pick(steps[0], &mut drafts, "retired-option"), Pick::Ignored);
        assert_eq!(drafts, before);
        assert_eq!(step_after_pick(&steps, 0, &drafts), None);
        assert_eq!(pick(steps[0], &mut drafts, "a"), Pick::Picked);
        let before = drafts.clone();
        assert_eq!(pick(steps[0], &mut drafts, "retired-option"), Pick::Ignored);
        assert_eq!(drafts, before);
    }
}

#[test]
fn malformed_boolean_option_is_not_silently_treated_as_false() {
    let question: Question = serde_json::from_value(json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "message": "Choose explicitly",
        "fields": [{"name":"answer", "kind":"boolean", "required":true}]
    }))
    .unwrap();
    let steps = steps_of(&question);
    let mut drafts = empty_drafts(&question);
    assert_eq!(pick(steps[0], &mut drafts, "true"), Pick::Picked);
    let before = drafts.clone();
    for invalid in ["", "TRUE", "retired-option"] {
        assert_eq!(pick(steps[0], &mut drafts, invalid), Pick::Ignored);
        assert_eq!(drafts, before);
    }
    assert_eq!(pick(steps[0], &mut drafts, "false"), Pick::Picked);
    assert_eq!(
        drafts.get(&"answer".parse().unwrap()),
        Some(&Draft::Boolean(Some(false)))
    );
}

#[test]
fn ecmascript_numbers_and_text_keep_exact_editor_drafts() {
    let question: Question = serde_json::from_str(
        r#"{
        "id":"00000000-0000-4000-8000-000000000001", "message":"Values",
        "fields":[
            {"name":"number","kind":"number","required":true},
            {"name":"integer","kind":"integer","required":true},
            {"name":"text","kind":"text","required":true}
        ]
    }"#,
    )
    .unwrap();
    let steps = steps_of(&question);
    let mut drafts = empty_drafts(&question);
    let input = ["\u{feff}1.5\u{feff}", "\u{feff}2\u{feff}", "\u{0085}"];
    for (index, raw) in input.iter().enumerate() {
        typed(steps[index], &mut drafts, (*raw).to_owned());
        assert!(step_ready(steps[index], &drafts));
        assert_eq!(
            drafts.get(&steps[index].field.name),
            Some(&Draft::Text((*raw).to_owned()))
        );
    }
    let answer = answer_of(&question, &drafts);
    question.check(&answer).unwrap();
    let vessel_proto::QuestionAnswer::Accept { values } = answer else {
        panic!("accepted typed values")
    };
    let Some(vessel_proto::conversation::AnswerValue::Number(number)) =
        values.get(&steps[0].field.name)
    else {
        panic!("number")
    };
    assert_eq!(number.get(), 1.5);
    assert_eq!(
        values.get(&steps[1].field.name),
        Some(&vessel_proto::conversation::AnswerValue::Integer(2))
    );
    assert_eq!(
        values.get(&steps[2].field.name),
        Some(&vessel_proto::conversation::AnswerValue::Text(
            "\u{0085}".to_owned()
        ))
    );
    for (index, raw) in ["\u{0085}1.5\u{0085}", "\u{0085}2\u{0085}"]
        .iter()
        .enumerate()
    {
        typed(steps[index], &mut drafts, (*raw).to_owned());
        assert!(!step_ready(steps[index], &drafts));
        assert_eq!(
            drafts.get(&steps[index].field.name),
            Some(&Draft::Text((*raw).to_owned()))
        );
    }
}
