//! Pinned form decimal-prefix semantics, followed by the existing typed node bound.
use super::*;
use crate::localization::{ENGLISH, Locale};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Expected {
    Unlimited,
    Value { value: u32 },
    Number,
    Range,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    input: String,
    #[serde(flatten)]
    expected: Expected,
}
#[test]
fn portable_run_limits_match_the_form_prefix_and_unlimited_rules() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/schedules/editor-fields.json"
    ))
    .unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for case in cases {
        assert!(ids.insert(case.id.clone()));
        let mut fields = Fields::empty();
        fields.set_prompt("Prompt".into());
        fields.set_limit(case.input.clone());
        assert_eq!(fields.limit(), case.input);
        match case.expected {
            Expected::Unlimited => assert!(fields.values().unwrap().max_runs.is_none()),
            Expected::Value { value } => {
                assert_eq!(fields.values().unwrap().max_runs.unwrap().get(), value);
            }
            Expected::Number => assert!(
                matches!(fields.error(), Some(FieldError::Number(_))),
                "{}",
                case.id
            ),
            Expected::Range => assert!(
                matches!(fields.error(), Some(FieldError::Range(_))),
                "{}",
                case.id
            ),
        }
    }
}
#[test]
fn opening_retains_unedited_wire_prompt_and_setting_trims_only_at_its_edge() {
    let definition = crate::schedules::tests::info().definition;
    let mut fields = Fields::open(&definition);
    assert_eq!(fields.values().unwrap().prompt, definition.prompt);
    assert_eq!(fields.prompt(), definition.prompt.as_str());
    fields.set_name(" 새 예약 ".into());
    assert_eq!(fields.values().unwrap().name.unwrap().as_str(), "새 예약");
    assert_eq!(fields.values().unwrap().prompt, definition.prompt);
    fields.set_name("   ".into());
    assert!(fields.values().unwrap().name.is_none());
    fields.set_prompt("  new prompt\n ".into());
    assert_eq!(fields.prompt(), "  new prompt\n ");
    assert_eq!(fields.values().unwrap().prompt.as_str(), "new prompt");
}
#[test]
fn every_field_error_has_whole_english_and_korean_words() {
    let catalogs = [
        Translator::get(ENGLISH).unwrap(),
        Translator::get(Locale::from_tag("ko").unwrap()).unwrap(),
    ];
    let cases = [
        (
            FieldError::Name("a\nb".parse::<ScheduleName>().unwrap_err()),
            "Use a short name on one line, without control characters.",
            "제어 문자 없이 짧은 이름을 한 줄로 입력하세요.",
        ),
        (
            FieldError::Prompt("".parse::<PromptText>().unwrap_err()),
            "Enter a prompt of at most 16384 UTF-8 bytes, without control characters except tabs and line breaks.",
            "프롬프트를 16384 UTF-8 바이트 이내로 입력하세요. 제어 문자는 탭과 줄바꿈만 사용할 수 있습니다.",
        ),
        (
            FieldError::Number("bad".parse::<u32>().unwrap_err()),
            "Enter a whole number from 1 to 1000000, or leave it blank for unlimited runs.",
            "1부터 1000000 사이의 정수를 입력하거나, 실행 횟수를 제한하지 않으려면 비워 두세요.",
        ),
        (
            FieldError::Range(MaxRuns::try_from(0).unwrap_err()),
            "Enter a whole number from 1 to 1000000, or leave it blank for unlimited runs.",
            "1부터 1000000 사이의 정수를 입력하거나, 실행 횟수를 제한하지 않으려면 비워 두세요.",
        ),
    ];
    for (error, en, ko) in cases {
        for (catalog, expected) in catalogs.into_iter().zip([en, ko]) {
            assert_eq!(
                error.label(catalog).replace(['\u{2068}', '\u{2069}'], ""),
                expected
            );
        }
    }
}
#[test]
fn prompt_validation_counts_utf8_bytes_and_refuses_control_characters() {
    let mut fields = Fields::empty();
    fields.set_prompt("x".repeat(AutomationText::PROMPT_MAX));
    assert!(fields.error().is_none());
    fields.set_prompt("가".repeat(AutomationText::PROMPT_MAX));
    assert!(matches!(fields.error(), Some(FieldError::Prompt(_))));
    fields.set_prompt("hello\u{0001}".into());
    assert!(matches!(fields.error(), Some(FieldError::Prompt(_))));
    fields.set_prompt("hello\tworld\r\n안녕".into());
    assert!(fields.error().is_none());
}

#[test]
fn feedback_belongs_to_the_edited_field_without_hiding_save_validation() {
    let mut fields = Fields::empty();
    assert!(matches!(fields.error(), Some(FieldError::Prompt(_))));
    assert!(fields.feedback(Field::Prompt).is_none());
    fields.set_limit("1000001".into());
    assert!(matches!(
        fields.feedback(Field::Limit),
        Some(FieldError::Range(_))
    ));
    assert!(fields.feedback(Field::Prompt).is_none());
    fields.set_prompt(" ".into());
    assert!(matches!(
        fields.feedback(Field::Prompt),
        Some(FieldError::Prompt(_))
    ));
    fields.set_prompt("Check the build".into());
    assert!(fields.feedback(Field::Prompt).is_none());
    assert!(fields.values().is_err());
    fields.set_limit("".into());
    assert!(fields.values().is_ok());
}
