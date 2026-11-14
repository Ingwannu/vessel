//! The page's `data/question-form.test.ts` and
//! `data/question-answers.test.ts` (`backend/m11`, `67bf243`), case for
//! case; the questions written as the wire's JSON.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use vessel_app_view::timeline::{
    AnsweredLine, Draft, Drafts, Pick, Problem, Read, Wrong, answer_of, answered_lines, choices,
    pick, picked, read_field, refused_field, starting_drafts, step_answered, step_header,
    step_problem, step_prompt, step_ready, steps_of, text_target, typed,
};
use vessel_proto::conversation::{AnswerValue, FieldName, FiniteNumber, QuestionField};
use vessel_proto::{Question, QuestionAnswer, QuestionOutcome};

use crate::support::uuid;

fn name(text: &str) -> FieldName {
    text.parse().expect("a field name")
}

fn choice(field: &str, multiple: bool) -> Value {
    json!({
        "name": field, "title": field, "required": false, "kind": "choice",
        "options": [
            { "value": "a", "title": "A", "description": "the first" },
            { "value": "b", "title": "B" }
        ],
        "multiple": multiple
    })
}

fn text(field: &str) -> Value {
    json!({ "name": field, "title": "Other", "required": false, "kind": "text" })
}

fn question(id: u32, message: &str, fields: &[Value]) -> Question {
    serde_json::from_value(json!({ "id": uuid(id), "message": message, "fields": fields }))
        .expect("a question the wire takes")
}

fn field(value: Value) -> QuestionField {
    serde_json::from_value(value).expect("a field the wire takes")
}

/// Claude's `AskUserQuestion` form as the node passes it: by key.
fn claude() -> Question {
    question(
        1,
        "Please answer the following questions.",
        &[
            choice("question_0", false),
            text("question_0_custom"),
            choice("question_10", true),
            text("question_10_custom"),
            choice("question_2", false),
            text("question_2_custom"),
        ],
    )
}

fn names(fields: impl Iterator<Item = Option<String>>) -> Vec<Option<String>> {
    fields.collect()
}

#[test]
fn claudes_pairs_are_one_step_each_in_their_numbers_order() {
    let question = claude();
    let steps = steps_of(&question);
    assert_eq!(
        names(
            steps
                .iter()
                .map(|step| Some(step.field.name.as_str().to_owned()))
        ),
        [Some("question_0"), Some("question_2"), Some("question_10")]
            .map(|name| name.map(str::to_owned))
    );
    assert_eq!(
        names(
            steps
                .iter()
                .map(|step| step.other.map(|other| other.name.as_str().to_owned()))
        ),
        [
            Some("question_0_custom"),
            Some("question_2_custom"),
            Some("question_10_custom")
        ]
        .map(|name| name.map(str::to_owned))
    );
}

#[test]
fn any_other_form_keeps_the_nodes_order_a_step_per_field() {
    let form = question(
        2,
        "Which way?",
        &[text("name"), choice("way", false), text("way_custom")],
    );
    let steps = steps_of(&form);
    assert_eq!(
        steps
            .iter()
            .map(|step| step.field.name.as_str())
            .collect::<Vec<_>>(),
        ["name", "way", "way_custom"]
    );
}

#[test]
fn defaults_start_filled_in() {
    let integer = json!({ "name": "n", "required": true, "kind": "integer", "default": 3 });
    let form = question(1, "", std::slice::from_ref(&integer));
    let drafts = starting_drafts(&form);
    assert_eq!(
        read_field(&field(integer), drafts.get(&name("n"))),
        Read::Value(AnswerValue::Integer(3))
    );
}

#[test]
fn numbers_take_a_sign_and_a_point_integers_no_point() {
    let number = field(json!({ "name": "x", "required": true, "kind": "number", "minimum": -5 }));
    let typed = |text: &str| Draft::Text(text.to_owned());
    assert_eq!(
        read_field(&number, Some(&typed("-2.5"))),
        Read::Value(AnswerValue::Number(
            FiniteNumber::new(-2.5).expect("finite")
        ))
    );
    assert_eq!(
        read_field(&number, Some(&typed("-6"))),
        Read::Wrong(Wrong::AtLeast("-5".to_owned()))
    );
    let integer = field(json!({ "name": "x", "required": true, "kind": "integer", "minimum": -5 }));
    assert_eq!(
        read_field(&integer, Some(&typed("1.5"))),
        Read::Wrong(Wrong::WholeNumber)
    );
}

#[test]
fn a_required_step_with_nothing_and_claudes_other_answering_it() {
    let mut required = choice("question_0", false);
    required["required"] = json!(true);
    let form = question(1, "", &[required, text("question_0_custom")]);
    let steps = steps_of(&form);
    let step = *steps.first().expect("a step");
    assert_eq!(
        step_problem(step, &Drafts::new()),
        Some(Problem::ChooseAnAnswer)
    );
    let drafts = Drafts::from([(name("question_0_custom"), Draft::Text("neither".to_owned()))]);
    assert_eq!(step_problem(step, &drafts), None);
    // Not the page's: the tab's check counts the Other box too (Paseo's).
    assert!(step_answered(step, &drafts));
}

#[test]
fn the_answer_holds_every_field_with_a_value_picks_and_notes_both() {
    let drafts = Drafts::from([
        (name("question_0"), Draft::Picked(vec!["a".to_owned()])),
        (
            name("question_0_custom"),
            Draft::Text("and quickly".to_owned()),
        ),
        (
            name("question_10"),
            Draft::Picked(vec!["a".to_owned(), "b".to_owned()]),
        ),
    ]);
    assert_eq!(
        answer_of(&claude(), &drafts),
        QuestionAnswer::Accept {
            values: BTreeMap::from([
                (name("question_0"), AnswerValue::Choice("a".to_owned())),
                (
                    name("question_0_custom"),
                    AnswerValue::Text("and quickly".to_owned())
                ),
                (
                    name("question_10"),
                    AnswerValue::Choices(vec!["a".to_owned(), "b".to_owned()])
                ),
            ])
        }
    );
}

#[test]
fn a_refusal_names_its_field() {
    let question = claude();
    assert_eq!(
        refused_field(&question, "field `question_2`: a value of another kind")
            .map(FieldName::as_str),
        Some("question_2")
    );
    assert_eq!(refused_field(&question, "something else"), None);
}

const YES_NO: (&str, &str) = ("Yes", "No");

fn line(field: &str, value: &str) -> AnsweredLine {
    AnsweredLine {
        field: field.to_owned(),
        value: value.to_owned(),
    }
}

#[test]
fn answers_by_title_a_choice_by_its_options_title_in_field_order() {
    let question = question(
        1,
        "Which way?",
        &[
            json!({
                "name": "way", "title": "Which way", "required": true, "kind": "choice",
                "options": [{ "value": "left", "title": "Left" }, { "value": "right", "title": "Right" }],
                "multiple": false
            }),
            json!({ "name": "sure", "title": "Sure", "required": false, "kind": "boolean" }),
            json!({ "name": "note", "required": false, "kind": "text" }),
        ],
    );
    let answered = QuestionOutcome::Answered {
        values: BTreeMap::from([
            (name("sure"), AnswerValue::Boolean(true)),
            (name("way"), AnswerValue::Choice("left".to_owned())),
        ]),
    };
    assert_eq!(
        answered_lines(&question, &answered, YES_NO),
        [line("Which way", "Left"), line("Sure", "Yes")]
    );
    assert!(answered_lines(&question, &QuestionOutcome::Declined, YES_NO).is_empty());
}

#[test]
fn claudes_free_text_joins_its_questions_line() {
    let question = question(
        2,
        "Please answer the following questions.",
        &[
            json!({
                "name": "question_1", "title": "Extras", "required": false, "kind": "choice",
                "options": [{ "value": "metrics", "title": "Metrics" }, { "value": "backups", "title": "Backups" }],
                "multiple": true
            }),
            json!({ "name": "question_1_custom", "title": "Other", "required": false, "kind": "text" }),
            json!({ "name": "question_0", "title": "Database", "required": false, "kind": "text" }),
            json!({ "name": "question_0_custom", "required": false, "kind": "text" }),
        ],
    );
    let answered = QuestionOutcome::Answered {
        values: BTreeMap::from([
            (name("question_0"), AnswerValue::Text("SQLite".to_owned())),
            (name("question_0_custom"), AnswerValue::Text(String::new())),
            (
                name("question_1"),
                AnswerValue::Choices(vec!["metrics".to_owned()]),
            ),
            (
                name("question_1_custom"),
                AnswerValue::Text("and alerts".to_owned()),
            ),
        ]),
    };
    assert_eq!(
        answered_lines(&question, &answered, YES_NO),
        [
            line("Database", "SQLite"),
            line("Extras", "Metrics, and alerts")
        ]
    );
}

// Not the page's: Paseo's buttons wait for each of Claude's questions to
// be answered (Next, then Submit), though Claude's form requires nothing;
// another agent's optional field may be left empty.
#[test]
fn claudes_questions_wait_for_an_answer_other_forms_for_what_they_require() {
    let question = claude();
    let steps = steps_of(&question);
    let first = *steps.first().expect("a step");
    assert!(!step_ready(first, &Drafts::new()));
    let picked = Drafts::from([(name("question_0"), Draft::Picked(vec!["b".to_owned()]))]);
    assert!(step_ready(first, &picked));
    let form = question_of_fields(&[text("note")]);
    let steps = steps_of(&form);
    assert!(step_ready(*steps.first().expect("a step"), &Drafts::new()));
}

fn question_of_fields(fields: &[Value]) -> Question {
    question(3, "Anything else?", fields)
}

// The card's step rules (`question_step.rs`): Claude's single choice and
// its "Other" replace each other; a multiple choice toggles; picking the
// same radio again clears it.
#[test]
fn a_pick_and_a_custom_answer_replace_each_other_on_a_single_choice() {
    let form = claude();
    let steps = steps_of(&form);
    let single = steps[0];
    let mut drafts = starting_drafts(&form);
    assert_eq!(
        text_target(single).map(|field| field.name.as_str()),
        Some("question_0_custom")
    );
    assert_eq!(step_header(single), "question_0");
    assert_eq!(step_prompt(&form, single), form.message);
    let (rows, multiple) = choices(single, ("Yes", "No"));
    assert!(!multiple);
    assert_eq!(
        rows.iter()
            .map(|row| row.value.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    typed(single, &mut drafts, "my own".to_owned());
    assert_eq!(
        drafts.get(&name("question_0_custom")),
        Some(&Draft::Text("my own".to_owned()))
    );
    assert_eq!(pick(single, &mut drafts, "a"), Pick::OtherCleared);
    assert!(picked(single, &drafts, "a"));
    assert_eq!(
        drafts.get(&name("question_0_custom")),
        Some(&Draft::Text(String::new()))
    );
    typed(single, &mut drafts, "again".to_owned());
    assert!(!picked(single, &drafts, "a"), "typing clears the pick");
    assert_eq!(pick(single, &mut drafts, "b"), Pick::OtherCleared);
    assert_eq!(
        pick(single, &mut drafts, "b"),
        Pick::Picked,
        "the same radio again clears it"
    );
    assert!(!picked(single, &drafts, "b"));
}

#[test]
fn a_multiple_choice_toggles_and_keeps_its_other() {
    let form = claude();
    let steps = steps_of(&form);
    // Claude's steps go in their numbers' order (0, 2, 10): the multiple
    // choice is question_10, found by name.
    let several = steps
        .iter()
        .copied()
        .find(|step| step.field.name.as_str() == "question_10")
        .expect("Claude's multiple choice");
    let mut drafts = starting_drafts(&form);
    typed(several, &mut drafts, "also this".to_owned());
    assert_eq!(pick(several, &mut drafts, "a"), Pick::Picked);
    assert_eq!(pick(several, &mut drafts, "b"), Pick::Picked);
    assert!(picked(several, &drafts, "a") && picked(several, &drafts, "b"));
    assert_eq!(pick(several, &mut drafts, "a"), Pick::Picked);
    assert!(!picked(several, &drafts, "a"));
    assert_eq!(
        drafts.get(&name("question_10_custom")),
        Some(&Draft::Text("also this".to_owned())),
        "a multiple choice keeps its Other"
    );
}

#[test]
fn a_text_step_has_no_rows_and_ignores_a_pick() {
    let form = question_of_fields(&[text("note")]);
    let steps = steps_of(&form);
    let note = steps[0];
    let mut drafts = starting_drafts(&form);
    assert!(choices(note, ("Yes", "No")).0.is_empty());
    assert_eq!(pick(note, &mut drafts, "a"), Pick::Ignored);
    typed(note, &mut drafts, "hello".to_owned());
    assert_eq!(
        drafts.get(&name("note")),
        Some(&Draft::Text("hello".to_owned()))
    );
}
