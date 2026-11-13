//! The page's `data/agent-status.test.ts`, and `agents.test.ts`'s
//! `agentsOn` case.

use chrono::{DateTime, Utc};
use vessel_proto::agents::AgentInfo;
use vessel_proto::{Harness, SessionOption};

use crate::agent_status::{
    Headline, OfferRow, agent_headline, agent_key, agent_status, agents_on, asked_when, offer_rows,
};
use crate::machines::StateTone;
use crate::{Lang, controls_of};

fn harness(json: serde_json::Value) -> Harness {
    serde_json::from_value(json).unwrap()
}

/// The page's `fixtures/machines.ts`.
fn claude() -> Harness {
    harness(serde_json::json!({
        "kind": "claude-code",
        "path": "/usr/local/bin/claude",
        "version": "2.1.0",
        "structured": { "state": "available" },
    }))
}

fn codex_terminal_only() -> Harness {
    harness(serde_json::json!({
        "kind": "codex",
        "path": "/usr/local/bin/codex",
        "version": "0.50.0",
        "structured": { "state": "unavailable", "reason": "codex-acp is not installed" },
    }))
}

#[test]
fn is_available_needs_attention_with_the_machines_reason_or_not_found() {
    let found = [claude(), codex_terminal_only()];
    let headline = |agent: &str| {
        let agent = serde_json::from_value(serde_json::json!(agent)).unwrap();
        agent_headline(Lang::En, &agent_status(&agent, &found, None))
    };
    assert_eq!(
        headline("claude-code"),
        Headline {
            headline: "Available",
            tone: StateTone::Ok,
            detail: None,
        }
    );
    assert_eq!(
        headline("codex"),
        Headline {
            headline: "Needs attention",
            tone: StateTone::Warn,
            detail: Some("Runs in a terminal only: codex-acp is not installed".to_owned()),
        }
    );
    assert_eq!(headline("gemini").headline, "Not found");
}

#[test]
fn lists_every_agent_the_installed_first() {
    let found = [codex_terminal_only()];
    let keys: Vec<String> = agents_on(&found, &[])
        .iter()
        .map(|entry| agent_key(&entry.agent))
        .collect();
    assert_eq!(keys, ["codex", "claude-code", "gemini", "open-code"]);
}

/// `agents.test.ts`: "is listed in Settings after the four found, a
/// missing one last".
#[test]
fn an_added_agent_is_listed_after_the_four_found_a_missing_one_after_it() {
    let added = harness(serde_json::json!({
        "kind": { "added": "goose" },
        "path": "/usr/local/bin/goose",
        "version": null,
        "structured": { "state": "available" },
        "terminal": { "state": "unavailable", "reason": "an added agent speaks ACP only" },
    }));
    let codex = harness(serde_json::json!({
        "kind": "codex",
        "path": "/bin/codex",
        "version": "0.50.0",
        "structured": { "state": "available" },
    }));
    let listed: Vec<AgentInfo> = serde_json::from_value(serde_json::json!([
        {
            "agent": { "added": "goose" },
            "detail": { "kind": "added", "added": {
                "label": "Goose", "program": "goose", "args": ["acp"], "env": [],
                "secrets": [{ "variable": "GOOSE_KEY", "secret": "GOOSE_KEY" }],
                "inherit": [], "mcp": [], "source": { "from": "command" },
            }},
        },
        {
            "agent": { "added": "gone" },
            "detail": { "kind": "added", "added": {
                "program": "/opt/gone/bin/gone", "args": [], "env": [], "secrets": [],
                "inherit": [], "mcp": [], "source": { "from": "command" },
            }},
        },
    ]))
    .unwrap();
    let found = [codex, added];
    let on = agents_on(&found, &listed);
    let keys: Vec<String> = on.iter().map(|entry| agent_key(&entry.agent)).collect();
    assert_eq!(
        keys,
        [
            "codex",
            "added:goose",
            "added:gone",
            "claude-code",
            "gemini",
            "open-code"
        ]
    );
    assert_eq!(
        agent_headline(Lang::En, &on[1].status),
        Headline {
            headline: "Available",
            tone: StateTone::Ok,
            detail: Some("Runs as a conversation; it has no terminal".to_owned()),
        }
    );
    assert_eq!(
        agent_headline(Lang::En, &on[2].status).detail.as_deref(),
        Some("Its program, /opt/gone/bin/gone, is not found on this machine")
    );
    let before_the_list: Vec<String> = agents_on(&found[1..], &[])
        .iter()
        .map(|entry| agent_key(&entry.agent))
        .collect();
    assert!(before_the_list.contains(&"added:goose".to_owned()));
}

fn row(id: &str, label: &str, values: &[&str], current: &str) -> OfferRow {
    OfferRow {
        id: id.to_owned(),
        label: label.to_owned(),
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        current: current.to_owned(),
    }
}

/// The fake agent's options (`crates/vessel-testkit`, `fake_acp/options.rs`),
/// in the order it gives them.
fn fake_options() -> Vec<SessionOption> {
    serde_json::from_value(serde_json::json!([
        { "id": "mode", "name": "Mode", "category": "mode", "kind": "select", "current": "default",
          "values": [{ "id": "default", "name": "Default" }, { "id": "plan", "name": "Plan" }] },
        { "id": "model", "name": "Model", "category": "model", "kind": "select", "current": "fake-small",
          "values": [{ "id": "fake-small", "name": "Fake small" }, { "id": "fake-large", "name": "Fake large" }] },
        { "id": "effort", "name": "Effort", "category": "thought_level", "kind": "select", "current": "high",
          "values": [{ "id": "low", "name": "Low", "group": "Quick" }, { "id": "high", "name": "High", "group": "Deep" }] },
        { "id": "fast", "name": "Fast", "category": "other", "kind": "toggle", "on": false },
        { "id": "later", "name": "Later", "category": "other", "kind": "unknown" },
    ]))
    .unwrap()
}

#[test]
fn lists_them_as_models_effort_permissions_then_the_rest_in_the_apps_words() {
    assert_eq!(
        offer_rows(Lang::En, &controls_of(&fake_options(), "Fake", Lang::En)),
        [
            row(
                "model",
                "Model",
                &["Fake small", "Fake large"],
                "Fake small"
            ),
            row("effort", "Effort", &["Low", "High"], "High"),
            row("mode", "Permissions", &["Default", "Plan"], "Default"),
            row("fast", "Fast", &["On", "Off"], "Off"),
        ]
    );
}

#[test]
fn names_a_second_option_of_a_category_by_its_own_name() {
    let select = |id: &str, name: &str| {
        serde_json::json!({ "id": id, "name": name, "category": "model", "kind": "select",
            "current": "a", "values": [{ "id": "a", "name": "A" }] })
    };
    let two: Vec<SessionOption> = serde_json::from_value(serde_json::json!([
        select("model", "Model"),
        select("fallback", "Fallback")
    ]))
    .unwrap();
    let labels: Vec<String> = offer_rows(Lang::En, &controls_of(&two, "Fake", Lang::En))
        .into_iter()
        .map(|row| row.label)
        .collect();
    assert_eq!(labels, ["Model", "Fallback"]);
    assert!(offer_rows(Lang::En, &controls_of(&[], "Fake", Lang::En)).is_empty());
}

#[test]
fn says_when_the_machine_asked() {
    let at = |text: &str| text.parse::<DateTime<Utc>>().unwrap();
    let now = at("2026-11-11T07:00:00Z");
    assert_eq!(
        asked_when(Lang::En, at("2026-11-11T06:59:30Z"), now),
        "Asked just now"
    );
    assert_eq!(
        asked_when(Lang::En, at("2026-11-11T06:57:00Z"), now),
        "Asked 3m ago"
    );
}
