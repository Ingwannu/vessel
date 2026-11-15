use super::*;
use vessel_proto::{ChoiceValue, OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId};

fn choice(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}
fn model(values: &[&str]) -> SessionOption {
    SessionOption {
        id: OptionId::new("model"),
        name: "Model".into(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new("default"),
            values: values
                .iter()
                .map(|value| OptionValue {
                    id: OptionValueId::new(*value),
                    name: (*value).into(),
                    description: None,
                    group: None,
                })
                .collect(),
        },
    }
}

#[test]
fn an_offered_id_past_the_shared_bound_is_refused_without_replacing_previous_defaults() {
    let long = "m".repeat(257);
    let options = [model(&["default", "old", &long])];
    let previous = [choice("model", "old")];
    let before = previous.clone();
    let error = prepare(&options, &previous, choice("model", &long)).unwrap_err();
    assert_eq!(error, SaveProblem::Storage(prefs::TooMany { max: 8 }));
    assert_eq!(previous, before);
    assert_eq!(
        prepare(&options, &previous, choice("model", "old"))
            .unwrap()
            .as_slice(),
        &previous
    );
    assert!(
        prepare(&options, &previous, choice("model", "default"))
            .unwrap()
            .as_slice()
            .is_empty()
    );
}

#[test]
fn saved_choice_byte_and_count_boundaries_match_the_existing_contract() {
    assert_eq!(prefs::MAX_CHOICE_BYTES, 256);
    for (value, accepted) in [
        ("v".repeat(67), true),
        ("v".repeat(256), true),
        ("v".repeat(257), false),
        (format!("{}a", "한".repeat(85)), true),
        (format!("{}ab", "한".repeat(85)), false),
    ] {
        let proposed = vec![choice("model", &value)];
        assert_eq!(
            checked(proposed).is_ok(),
            accepted,
            "UTF-8 bytes {}",
            value.len()
        );
    }
    assert!(checked(vec![choice(&"i".repeat(256), "v")]).is_ok());
    assert!(matches!(
        checked(vec![choice(&"i".repeat(257), "v")]),
        Err(SaveProblem::Storage(_))
    ));
    let choices: Vec<_> = (0..8)
        .map(|i| choice(&format!("option-{i}"), "v"))
        .collect();
    assert_eq!(checked(choices.clone()).unwrap().as_slice(), choices);
    let mut too_many = choices;
    too_many.push(choice("option-nine", "v"));
    assert_eq!(
        checked(too_many),
        Err(SaveProblem::Storage(prefs::TooMany { max: 8 }))
    );
}

#[test]
fn stale_unknown_mismatched_and_conflicting_options_are_not_silently_dropped() {
    let options = [model(&["default", "new"])];
    for proposed in [
        choice("missing", "new"),
        choice("model", "absent"),
        OptionChoice {
            id: OptionId::new("model"),
            value: ChoiceValue::Toggle { on: true },
        },
    ] {
        assert_eq!(
            prepare(&options, &[], proposed),
            Err(SaveProblem::NotOffered)
        );
    }
    let repeated = [options[0].clone(), options[0].clone()];
    assert_eq!(
        prepare(&repeated, &[], choice("model", "new")),
        Err(SaveProblem::Conflicting)
    );
    assert_eq!(
        checked(vec![choice("model", "one"), choice("model", "two")]),
        Err(SaveProblem::Conflicting)
    );
}

#[test]
fn every_default_refusal_has_a_whole_sentence_in_each_language() {
    use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
    let en = Translator::get(ENGLISH).unwrap().agent_defaults();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_defaults();
    for (problem, english, korean) in [
        (
            SaveProblem::Storage(prefs::TooMany { max: 8 }),
            "This device cannot save the selected defaults. Choose another option.",
            "이 기기에는 선택한 기본값을 저장할 수 없습니다. 다른 옵션을 선택하세요.",
        ),
        (
            SaveProblem::NotOffered,
            "The agent no longer offers this option. The previous defaults are kept.",
            "에이전트가 더 이상 이 옵션을 제공하지 않습니다. 이전 기본값을 유지합니다.",
        ),
        (
            SaveProblem::Conflicting,
            "The agent reported conflicting options. The previous defaults are kept.",
            "에이전트가 서로 충돌하는 옵션을 보냈습니다. 이전 기본값을 유지합니다.",
        ),
    ] {
        assert_eq!(en.problem(problem), english);
        assert_eq!(ko.problem(problem), korean);
        assert_eq!(problem.variant(), crate::alert::Variant::Error);
    }
}
