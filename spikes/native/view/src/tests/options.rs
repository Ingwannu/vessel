//! `web/src/data/options.test.ts`: the options' layout, against the shapes
//! real agents return (Claude Code's adapter offers a permissions mode, a
//! model, an effort and two on/off model settings; `OpenCode` a model and a
//! mode), and the picks made before a session exists.

use vessel_proto::{
    ChoiceValue, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValue, OptionValueId,
    SessionOption,
};

use crate::{Controls, Lang, Picks, choices_for, controls_of, own_name, with_picks};

fn value(id: &str, name: &str, description: Option<&str>, group: Option<&str>) -> OptionValue {
    OptionValue {
        id: OptionValueId::new(id),
        name: name.to_owned(),
        description: description.map(str::to_owned),
        group: group.map(str::to_owned),
    }
}

fn select(
    id: &str,
    name: &str,
    category: OptionCategory,
    current: &str,
    values: Vec<OptionValue>,
) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: name.to_owned(),
        description: None,
        category,
        kind: OptionKind::Select {
            current: OptionValueId::new(current),
            values,
        },
    }
}

fn toggle(id: &str, name: &str, category: OptionCategory, on: bool) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: name.to_owned(),
        description: None,
        category,
        kind: OptionKind::Toggle { on },
    }
}

fn claude() -> Vec<SessionOption> {
    vec![
        select(
            "mode",
            "Mode",
            OptionCategory::Mode,
            "default",
            vec![
                value(
                    "default",
                    "Default",
                    Some("Standard behavior, prompts for dangerous operations"),
                    None,
                ),
                value(
                    "acceptEdits",
                    "Accept Edits",
                    Some("Auto-accept file edit operations"),
                    None,
                ),
                value("plan", "Plan Mode", Some("Planning mode"), None),
            ],
        ),
        select(
            "model",
            "Model",
            OptionCategory::Model,
            "default",
            vec![
                value("default", "Default (recommended)", None, None),
                value("opus", "Opus", None, None),
                value("haiku", "Haiku", None, None),
            ],
        ),
        select(
            "effort",
            "Effort",
            OptionCategory::ThoughtLevel,
            "high",
            vec![
                value("low", "Low", None, Some("Quick")),
                value("high", "High", None, Some("Deep")),
                value("max", "Max", None, Some("Deep")),
            ],
        ),
        select(
            "fast",
            "Fast mode",
            OptionCategory::ModelConfig,
            "off",
            vec![
                value("off", "Off", None, None),
                value("on", "On", None, None),
            ],
        ),
        toggle("thinking", "Thinking", OptionCategory::ModelConfig, true),
        SessionOption {
            id: OptionId::new("subagents"),
            name: "Agents".to_owned(),
            description: None,
            category: OptionCategory::Other,
            kind: OptionKind::Unknown,
        },
    ]
}

fn opencode() -> Vec<SessionOption> {
    vec![
        select(
            "model",
            "Model",
            OptionCategory::Model,
            "anthropic/claude",
            vec![
                value("anthropic/claude", "Claude", None, None),
                value("openai/gpt", "GPT", None, None),
            ],
        ),
        select(
            "mode",
            "Mode",
            OptionCategory::Mode,
            "build",
            vec![
                value("build", "build", None, None),
                value("plan", "plan", None, None),
            ],
        ),
    ]
}

fn id_of(control: Option<&crate::SelectControl>) -> Option<&str> {
    control.map(|control| control.id.as_str())
}

fn chosen(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

const EN: Lang = Lang::En;

/// "puts each of Claude's options where the page shows it"
#[test]
fn each_of_claudes_options_goes_where_the_page_shows_it() {
    let controls = controls_of(&claude(), "Claude Code", EN);
    assert_eq!(id_of(controls.permissions.as_ref()), Some("mode"));
    assert_eq!(id_of(controls.model.as_ref()), Some("model"));
    assert_eq!(id_of(controls.effort.as_ref()), Some("effort"));
    // The on/off select is listed with the other choices, the toggle is a
    // switch, and the kind this build does not know is skipped.
    let selects: Vec<&str> = controls.selects.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(selects, ["fast"]);
    assert_eq!(controls.toggles.len(), 1);
    let thinking = &controls.toggles[0];
    assert_eq!(
        (thinking.id.as_str(), thinking.name.as_str(), thinking.on),
        ("thinking", "Thinking", true)
    );
    assert_eq!(thinking.description, None);
}

/// "keeps the agent's words, descriptions and groups"
#[test]
fn the_agents_words_descriptions_and_groups_are_kept() {
    let controls = controls_of(&claude(), "Claude Code", EN);
    let permissions = controls.permissions.expect("Claude offers a mode");
    assert_eq!(
        permissions.values[1],
        value(
            "acceptEdits",
            "Accept Edits",
            Some("Auto-accept file edit operations"),
            None
        )
    );
    let effort = controls.effort.expect("Claude offers an effort");
    let groups: Vec<Option<&str>> = effort
        .values
        .iter()
        .map(|value| value.group.as_deref())
        .collect();
    assert_eq!(groups, [Some("Quick"), Some("Deep"), Some("Deep")]);
}

/// "offers no effort for an agent without one"
#[test]
fn no_effort_for_an_agent_without_one() {
    let controls = controls_of(&opencode(), "OpenCode", EN);
    assert_eq!(id_of(controls.model.as_ref()), Some("model"));
    assert_eq!(id_of(controls.permissions.as_ref()), Some("mode"));
    assert_eq!(controls.effort, None);
    assert!(controls.toggles.is_empty());
    assert!(controls.offers_anything());
}

/// "offers nothing for an agent that offers nothing"
#[test]
fn nothing_for_an_agent_that_offers_nothing() {
    assert_eq!(controls_of(&[], "Claude Code", EN), Controls::default());
    assert!(!controls_of(&[], "Claude Code", EN).offers_anything());
}

/// "files a second option of one category with the other choices"
#[test]
fn a_second_option_of_one_category_goes_with_the_other_choices() {
    let mut options = claude();
    options.push(select(
        "small-model",
        "Small model",
        OptionCategory::Model,
        "haiku",
        vec![value("haiku", "Haiku", None, None)],
    ));
    let controls = controls_of(&options, "Claude Code", EN);
    assert_eq!(id_of(controls.model.as_ref()), Some("model"));
    let selects: Vec<&str> = controls.selects.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(selects, ["fast", "small-model"]);
}

/// "name the value in effect, or its id when the agent does not list it"
#[test]
fn the_value_in_effect_is_named_or_its_id_shown() {
    let controls = controls_of(&claude(), "Claude Code", EN);
    assert_eq!(
        controls
            .model
            .as_ref()
            .map(crate::SelectControl::current_name),
        Some("Default (recommended)")
    );
    let odd = controls_of(
        &[select(
            "model",
            "Model",
            OptionCategory::Model,
            "local/llama",
            vec![value("anthropic/claude", "Claude", None, None)],
        )],
        "Claude Code",
        EN,
    );
    assert_eq!(
        odd.model.as_ref().map(crate::SelectControl::current_name),
        Some("local/llama")
    );
}

/// "read the model then the effort, or the agent with no model"
#[test]
fn the_picker_reads_the_model_then_the_effort() {
    let claude = controls_of(&claude(), "Claude Code", EN);
    assert_eq!(
        claude.picker_label("Claude Code"),
        ("Default (recommended)", Some("High"))
    );
    let opencode = controls_of(&opencode(), "OpenCode", EN);
    assert_eq!(opencode.picker_label("OpenCode"), ("Claude", None));
    assert_eq!(Controls::default().picker_label("Codex"), ("Codex", None));
}

/// "show what the session will start with, and send only changes"
#[test]
fn picks_show_what_starts_and_send_only_changes() {
    let options = claude();
    let controls = controls_of(&options, "Claude Code", EN);
    let (Some(permissions), Some(effort), Some(thinking)) = (
        controls.permissions.as_ref(),
        controls.effort.as_ref(),
        controls.toggles.first(),
    ) else {
        panic!("Claude offers all three");
    };
    let mut picks = Picks::default();
    picks.pick(permissions.choose(OptionValueId::new("plan")));
    // The effort the agent already has is no change.
    picks.pick(effort.choose(OptionValueId::new("high")));
    picks.pick(thinking.choose(false));
    let shown = controls_of(&with_picks(&options, &picks), "Claude Code", EN);
    assert_eq!(
        shown.permissions.map(|control| control.current),
        Some(OptionValueId::new("plan"))
    );
    assert_eq!(shown.toggles.first().map(|toggle| toggle.on), Some(false));
    assert_eq!(
        choices_for(&options, &picks),
        [
            chosen("mode", "plan"),
            OptionChoice {
                id: OptionId::new("thinking"),
                value: ChoiceValue::Toggle { on: false },
            },
        ]
    );
}

/// "drop a pick the agent does not offer"
#[test]
fn a_pick_the_agent_does_not_offer_is_dropped() {
    // Made for Claude, then the agent changed to OpenCode.
    let picks = Picks::from_choices(vec![chosen("mode", "acceptEdits"), chosen("effort", "max")]);
    assert!(choices_for(&opencode(), &picks).is_empty());
    assert_eq!(with_picks(&opencode(), &picks), opencode());
}

/// "keep the agent's order, not the order they were made in"
#[test]
fn choices_keep_the_agents_order() {
    let picks = Picks::from_choices(vec![chosen("model", "openai/gpt"), chosen("mode", "plan")]);
    let order: Vec<OptionId> = choices_for(&opencode(), &picks)
        .into_iter()
        .map(|choice| choice.id)
        .collect();
    assert_eq!(order, [OptionId::new("model"), OptionId::new("mode")]);
}

/// "says whose an option is when the page already uses its name"
#[test]
fn an_option_named_like_a_row_says_whose_it_is() {
    assert_eq!(own_name("Agent", "Claude Code", EN), "Claude Code agent");
    assert_eq!(own_name(" agent ", "Claude Code", EN), "Claude Code agent");
    assert_eq!(own_name("Model", "OpenCode", EN), "OpenCode model");
    assert_eq!(own_name("EFFORT", "Codex", EN), "Codex effort");
    assert_eq!(own_name("Permissions", "Codex", EN), "Codex permissions");
    // Not in the page (English only): beside Korean rows an English
    // "Agent" cannot be confused with "에이전트", and is kept.
    assert_eq!(own_name("Agent", "Claude Code", Lang::Ko), "Agent");
    assert_eq!(
        own_name("모델", "Claude Code", Lang::Ko),
        "Claude Code 모델"
    );
}

/// "keeps every other name exactly as the agent gave it"
#[test]
fn every_other_name_is_kept_as_given() {
    for name in ["Fast mode", "Agents", "Sub-agent", ""] {
        assert_eq!(own_name(name, "Claude Code", EN), name);
    }
}

/// "names an agent's other options through it, and leaves the slots alone"
#[test]
fn other_options_are_named_through_it_and_the_slots_left_alone() {
    let mut options = claude();
    options.push(select(
        "agent",
        "Agent",
        OptionCategory::Other,
        "default",
        vec![
            value("default", "Default", None, None),
            value("reviewer", "reviewer", None, None),
        ],
    ));
    options.push(select(
        "small_model",
        "Model",
        OptionCategory::Model,
        "haiku",
        vec![value("haiku", "Haiku", None, None)],
    ));
    options.push(toggle(
        "effort_toggle",
        "Effort",
        OptionCategory::Other,
        true,
    ));
    let controls = controls_of(&options, "Claude Code", EN);
    // The first model is in the composer's slot; its own name is not shown.
    assert_eq!(id_of(controls.model.as_ref()), Some("model"));
    let named: Vec<(&str, &str)> = controls
        .selects
        .iter()
        .filter(|select| select.id.as_str() != "fast")
        .map(|select| (select.id.as_str(), select.name.as_str()))
        .collect();
    assert_eq!(
        named,
        [
            ("agent", "Claude Code agent"),
            ("small_model", "Claude Code model")
        ]
    );
    assert_eq!(
        controls
            .toggles
            .iter()
            .find(|toggle| toggle.id.as_str() == "effort_toggle")
            .map(|toggle| toggle.name.as_str()),
        Some("Claude Code effort")
    );
    // Only the name shown changes: the choice is still the agent's option.
    let shown = controls
        .selects
        .iter()
        .find(|select| select.id.as_str() == "agent")
        .map(|select| select.choose(OptionValueId::new("reviewer")));
    assert_eq!(shown, Some(chosen("agent", "reviewer")));
}
