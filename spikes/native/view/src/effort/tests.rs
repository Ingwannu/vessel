//! The effort control over the shapes agents send: Claude Code as its
//! adapter 0.100.0 sends (the fixture `claude-agent-acp-0.100.0`: `effort`
//! with "default" and five levels, `fast` a select of on and off) with the
//! `ultracode` select the preview node's Claude Code adds, and its older
//! `fast_mode` toggle; Codex with levels up to its own Max and Ultra and a
//! `fast` select; an agent the table does not know.

use vessel_proto::{
    ChoiceValue, HarnessKind, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValue,
    OptionValueId, SessionOption,
};

use super::{Effort, Step, Switch};

fn select(
    id: &str,
    category: OptionCategory,
    current: &str,
    values: &[(&str, &str)],
) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category,
        kind: OptionKind::Select {
            current: OptionValueId::new(current),
            values: values
                .iter()
                .map(|(value, name)| OptionValue {
                    id: OptionValueId::new(*value),
                    name: (*name).to_owned(),
                    description: None,
                    group: None,
                })
                .collect(),
        },
    }
}

fn claude(level: &str, ultracode: &str, fast: &str) -> Vec<SessionOption> {
    vec![
        select(
            "mode",
            OptionCategory::Mode,
            "default",
            &[("default", "Default")],
        ),
        select("model", OptionCategory::Model, "opus", &[("opus", "Opus")]),
        select(
            "effort",
            OptionCategory::ThoughtLevel,
            level,
            &[
                ("default", "Default"),
                ("low", "Low"),
                ("medium", "Medium"),
                ("high", "High"),
                ("xhigh", "Extra high"),
                ("max", "Max"),
            ],
        ),
        select(
            "fast",
            OptionCategory::ModelConfig,
            fast,
            &[("on", "On"), ("off", "Off")],
        ),
        select(
            "ultracode",
            OptionCategory::Other,
            ultracode,
            &[("off", "Off"), ("on", "On")],
        ),
        select(
            "agent",
            OptionCategory::Other,
            "default",
            &[("default", "Default")],
        ),
    ]
}

fn names(effort: &Effort) -> Vec<&str> {
    effort
        .steps
        .iter()
        .map(|step| match step {
            Step::Level { name, .. } | Step::Top { name } => name.as_str(),
        })
        .collect()
}

fn level(value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new("effort"),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

fn set(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

#[test]
fn claude_codes_levels_end_with_ultracode_and_leave_out_default() {
    let effort = Effort::of(Some(HarnessKind::ClaudeCode), &claude("high", "off", "off"));
    let effort = effort.expect("Claude Code sends levels");
    assert_eq!(
        names(&effort),
        ["Low", "Medium", "High", "Extra high", "Max", "ultracode"]
    );
    assert_eq!(effort.current, Some(2));
    assert!(effort.fast.as_ref().is_some_and(|fast| !fast.on()));
    assert!(effort.drives(&OptionId::new("ultracode")));
    assert!(effort.drives(&OptionId::new("fast")));
    assert!(!effort.drives(&OptionId::new("agent")));
}

#[test]
fn the_top_step_turns_ultracode_on_at_the_highest_level_and_a_lower_one_off() {
    let effort =
        Effort::of(Some(HarnessKind::ClaudeCode), &claude("high", "off", "off")).expect("levels");
    assert_eq!(effort.choose(5), [level("max"), set("ultracode", "on")]);
    let on =
        Effort::of(Some(HarnessKind::ClaudeCode), &claude("max", "on", "off")).expect("levels");
    assert_eq!(on.current, Some(5));
    assert_eq!(on.choose(3), [level("xhigh"), set("ultracode", "off")]);
    // Ultracode already off: only the level.
    assert_eq!(effort.choose(1), [level("medium")]);
}

#[test]
fn reset_is_the_agents_default_and_ultracode_off() {
    let on = Effort::of(Some(HarnessKind::ClaudeCode), &claude("max", "on", "on")).expect("levels");
    assert_eq!(on.reset(), [level("default"), set("ultracode", "off")]);
    let default = Effort::of(
        Some(HarnessKind::ClaudeCode),
        &claude("default", "off", "off"),
    )
    .expect("levels");
    assert_eq!(default.current, None);
}

#[test]
fn fast_is_a_select_or_the_older_toggle() {
    let effort =
        Effort::of(Some(HarnessKind::ClaudeCode), &claude("high", "off", "on")).expect("levels");
    let fast = effort.fast.expect("a fast switch");
    assert!(fast.on());
    assert_eq!(fast.choose(false), set("fast", "off"));
    let mut older = claude("high", "off", "off");
    older.retain(|option| option.id.as_str() != "fast");
    older.push(SessionOption {
        id: OptionId::new("fast_mode"),
        name: String::new(),
        description: None,
        category: OptionCategory::Other,
        kind: OptionKind::Toggle { on: false },
    });
    let effort = Effort::of(Some(HarnessKind::ClaudeCode), &older).expect("levels");
    assert_eq!(
        effort.fast,
        Some(Switch::Toggle {
            id: OptionId::new("fast_mode"),
            on: false
        })
    );
}

/// Codex as the preview node's sends it (read off the app's log of its
/// probe, 2026-11-13).
#[test]
fn codexs_top_levels_are_its_own_steps_and_fast_mode_is_its_switch() {
    let options = vec![
        select(
            "mode",
            OptionCategory::Mode,
            "agent",
            &[("read-only", "Read only"), ("agent", "Agent")],
        ),
        select(
            "collaboration_mode",
            OptionCategory::Other,
            "default",
            &[("default", "Default"), ("plan", "Plan")],
        ),
        select(
            "model",
            OptionCategory::Model,
            "gpt-6.1-sol",
            &[("gpt-6.1-sol", "6.1 Sol")],
        ),
        select(
            "reasoning_effort",
            OptionCategory::ThoughtLevel,
            "ultra",
            &[
                ("low", "Low"),
                ("medium", "Medium"),
                ("high", "High"),
                ("xhigh", "Extra high"),
                ("max", "Max"),
                ("ultra", "Ultra"),
            ],
        ),
        select(
            "fast-mode",
            OptionCategory::ModelConfig,
            "off",
            &[("off", "Off"), ("on", "On")],
        ),
    ];
    let effort = Effort::of(Some(HarnessKind::Codex), &options).expect("levels");
    assert_eq!(
        names(&effort),
        ["Low", "Medium", "High", "Extra high", "Max", "Ultra"]
    );
    assert_eq!(effort.current, Some(5));
    assert_eq!(effort.maximum(), Some(5), "Ultra is Codex's maximum");
    assert!(effort.top.is_none());
    let fast = effort.fast.as_ref().expect("a fast switch");
    assert_eq!(fast.choose(true), set("fast-mode", "on"));
    assert!(effort.drives(&OptionId::new("fast-mode")));
    assert!(!effort.drives(&OptionId::new("collaboration_mode")));
}

#[test]
fn an_agent_the_table_does_not_know_gets_its_levels_only() {
    let effort =
        Effort::of(Some(HarnessKind::Gemini), &claude("high", "off", "off")).expect("levels");
    assert_eq!(
        names(&effort),
        ["Low", "Medium", "High", "Extra high", "Max"]
    );
    assert!(effort.top.is_none() && effort.fast.is_none());
    assert_eq!(
        effort.maximum(),
        Some(4),
        "a last level named max is the maximum"
    );
    let none = Effort::of(Some(HarnessKind::OpenCode), &[]);
    assert!(none.is_none());
}

/// The maximum is the folded top switch when there is one, else a last
/// level the table names; an agent whose last level is neither has none.
#[test]
fn the_maximum_is_a_role_not_a_word() {
    let claude =
        Effort::of(Some(HarnessKind::ClaudeCode), &claude("high", "off", "off")).expect("levels");
    assert_eq!(claude.maximum(), Some(5), "Ultracode, after Max");
    let plain = Effort::of(
        Some(HarnessKind::OpenCode),
        &[select(
            "effort",
            OptionCategory::ThoughtLevel,
            "high",
            &[("low", "Low"), ("high", "High")],
        )],
    )
    .expect("levels");
    assert_eq!(plain.maximum(), None);
    for known in super::ROLES {
        assert!(!known.ids.is_empty());
    }
}
