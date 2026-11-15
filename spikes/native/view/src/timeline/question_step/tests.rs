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
