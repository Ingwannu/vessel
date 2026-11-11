use vessel_proto::{OptionChoice, OptionValue};

use super::*;

fn select(value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new("mode"),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

fn modes(current: &str) -> acp::SessionModeState {
    acp::SessionModeState::new(
        current.to_owned(),
        vec![
            acp::SessionMode::new("ask", "Ask"),
            acp::SessionMode::new("code", "Code").description("Edits without asking"),
        ],
    )
}

/// `claude-agent-acp` 0.100.0's `configOptions` from a `session/new`
/// (2026-11-11), trimmed to three models and without the option listing
/// the account's own agents.
const RECORDED_CLAUDE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/acp/claude-agent-acp-0.100.0.config-options.json"
));

fn claude() -> Options {
    Options::new(Some(serde_json::from_str(RECORDED_CLAUDE).unwrap()), None)
}

#[test]
fn the_recorded_claude_options_read_as_they_were_offered() {
    let listed = claude().listed();
    let summary: Vec<_> = listed
        .iter()
        .map(|option| (option.id.as_str(), option.category, option.current_name()))
        .collect();
    assert_eq!(
        summary,
        [
            ("mode", OptionCategory::Mode, Some("Manual")),
            ("model", OptionCategory::Model, Some("Opus 5.5")),
            ("effort", OptionCategory::ThoughtLevel, Some("High")),
            // An on/off the agent made a select stays a select: its shape
            // is the agent's.
            ("fast", OptionCategory::ModelConfig, Some("Off")),
        ]
    );
    let Some(OptionKind::Select { values, .. }) = listed.first().map(|option| &option.kind) else {
        panic!("the mode is a select");
    };
    assert_eq!(
        values.first(),
        Some(&OptionValue {
            id: OptionValueId::new("default"),
            name: "Manual".to_owned(),
            description: Some("Always ask before making changes".to_owned()),
            group: None,
        })
    );
}

#[test]
fn a_choice_becomes_the_request_that_makes_it() {
    let options = claude();
    assert_eq!(
        options.change(&select("plan")),
        Ok(Some(Change::Config {
            config: acp::SessionConfigId::new("mode"),
            value: acp::SessionConfigOptionValue::value_id(acp::SessionConfigValueId::new("plan")),
        }))
    );
    assert_eq!(options.change(&select("default")), Ok(None), "current");
    assert!(matches!(
        options.change(&select("yolo")),
        Err(OptionError::NotOffered { .. })
    ));
    let toggle = OptionChoice {
        id: OptionId::new("fast"),
        value: ChoiceValue::Toggle { on: true },
    };
    assert_eq!(
        options.change(&toggle),
        Err(OptionError::WrongKind {
            id: OptionId::new("fast"),
            kind: KindName::Select,
        })
    );
    let unknown = OptionChoice {
        id: OptionId::new("temperature"),
        value: ChoiceValue::Toggle { on: true },
    };
    assert_eq!(
        options.change(&unknown).unwrap_err().to_string(),
        "the agent offers no option `temperature` (offered: mode, model, effort, fast)"
    );
}

#[test]
fn toggles_are_set_with_booleans() {
    let options = Options::new(
        Some(vec![acp::SessionConfigOption::boolean(
            "fast", "Fast", false,
        )]),
        None,
    );
    assert_eq!(
        options.listed().first().map(|option| &option.kind),
        Some(&OptionKind::Toggle { on: false })
    );
    let on = OptionChoice {
        id: OptionId::new("fast"),
        value: ChoiceValue::Toggle { on: true },
    };
    assert_eq!(
        options.change(&on),
        Ok(Some(Change::Config {
            config: acp::SessionConfigId::new("fast"),
            value: acp::SessionConfigOptionValue::boolean(true),
        }))
    );
}

#[test]
fn older_modes_become_a_mode_option_set_with_set_mode() {
    let mut options = Options::new(None, Some(modes("ask")));
    let listed = options.listed();
    let mode = listed.first().unwrap();
    assert_eq!(
        (mode.id.as_str(), mode.category, mode.current_name()),
        ("mode", OptionCategory::Mode, Some("Ask"))
    );
    let change = options.change(&select("code")).unwrap().unwrap();
    assert_eq!(change, Change::Mode(acp::SessionModeId::new("code")));
    options.apply(Applied::Mode(acp::SessionModeId::new("code")));
    assert_eq!(
        options
            .listed()
            .first()
            .and_then(SessionOption::current_name),
        Some("Code")
    );
    // The agent's own report moves it too.
    options.mode_is(acp::SessionModeId::new("ask"));
    assert_eq!(options.change(&select("ask")), Ok(None));
}

#[test]
fn a_mode_option_of_the_agent_wins_over_its_older_modes() {
    let config = vec![
        acp::SessionConfigOption::select(
            "mode",
            "Mode",
            "build",
            vec![
                acp::SessionConfigSelectOption::new("build", "Build"),
                acp::SessionConfigSelectOption::new("plan", "Plan"),
            ],
        )
        .category(acp::SessionConfigOptionCategory::Mode),
    ];
    let options = Options::new(Some(config), Some(modes("ask")));
    assert_eq!(options.listed().len(), 1, "not shown twice");
    // An option named `mode` of another category leaves the synthesized
    // one another id.
    let taken = vec![acp::SessionConfigOption::boolean(
        "mode",
        "Mode flag",
        false,
    )];
    let options = Options::new(Some(taken), Some(modes("ask")));
    let ids: Vec<_> = options
        .listed()
        .into_iter()
        .map(|option| option.id)
        .collect();
    assert_eq!(ids, [OptionId::new("session_mode"), OptionId::new("mode")]);
}

#[test]
fn grouped_values_keep_their_group_names_in_order() {
    let config = vec![
        acp::SessionConfigOption::select(
            "model",
            "Model",
            "b",
            vec![
                acp::SessionConfigSelectGroup::new(
                    "fast",
                    "Fast",
                    vec![acp::SessionConfigSelectOption::new("a", "A")],
                ),
                acp::SessionConfigSelectGroup::new(
                    "deep",
                    "Deep",
                    vec![acp::SessionConfigSelectOption::new("b", "B")],
                ),
            ],
        )
        .category(acp::SessionConfigOptionCategory::Model),
    ];
    let listed = Options::new(Some(config), None).listed();
    let Some(OptionKind::Select { values, current }) = listed.first().map(|option| &option.kind)
    else {
        panic!("a select");
    };
    assert_eq!(current, &OptionValueId::new("b"));
    let groups: Vec<_> = values.iter().map(|value| value.group.as_deref()).collect();
    assert_eq!(groups, [Some("Fast"), Some("Deep")]);
}

#[test]
fn none_offered_is_an_empty_list() {
    assert!(Options::new(None, None).listed().is_empty());
    let mut empty = modes("ask");
    empty.available_modes.clear();
    assert!(
        Options::new(Some(Vec::new()), Some(empty))
            .listed()
            .is_empty()
    );
}
