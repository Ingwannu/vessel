//! Every agent the node knows is listed (the owner on 0.0.0.0.4: Gemini,
//! not installed on the node, was missing from the selector): `agent.list`
//! merged with the installed harnesses.

use vessel_proto::agents::AgentInfo;
use vessel_proto::{Harness, HarnessKind, HarnessRef, StructuredSupport};

use crate::Agent;

fn harness(kind: HarnessKind, structured: StructuredSupport) -> Harness {
    Harness {
        kind: HarnessRef::from(kind),
        path: "/usr/bin/agent".into(),
        version: None,
        structured,
        terminal: vessel_proto::TerminalSupport::default(),
    }
}

/// The node's answer to `agent.list`, as it is on the wire.
fn known() -> Vec<AgentInfo> {
    serde_json::from_str(
        r#"[
          {"agent":"claude-code","detail":{"kind":"known","installs":[],"sign_in":{}}},
          {"agent":"gemini","detail":{"kind":"known","installs":[
            {"part":"harness","command":"npm install -g @google/gemini-cli"}],"sign_in":{}}},
          {"agent":{"added":"pi"},"detail":{"kind":"added","added":{"label":"Pi","program":"pi-acp","args":[],"source":{"from":"command"}}}}
        ]"#,
    )
    .unwrap()
}

#[test]
fn an_agent_not_installed_is_listed_with_how_to_install_it() {
    let installed = [harness(
        HarnessKind::ClaudeCode,
        StructuredSupport::Available,
    )];
    let agents = Agent::known(&installed, &known());
    let names: Vec<&str> = agents.iter().map(|agent| agent.name.as_str()).collect();
    assert_eq!(names, ["Claude Code", "Gemini", "Pi"]);
    assert_eq!(agents[0].unavailable, None);
    assert_eq!(
        agents[1].unavailable.as_deref(),
        Some("not installed on this machine (install it with `npm install -g @google/gemini-cli`)")
    );
    assert_eq!(
        agents[2].unavailable.as_deref(),
        Some("its program `pi-acp` is not found on this machine")
    );
}

#[test]
fn an_installed_agent_keeps_the_nodes_reason() {
    let installed = [harness(
        HarnessKind::ClaudeCode,
        StructuredSupport::Unavailable {
            reason: "its ACP adapter is not installed".to_owned(),
        },
    )];
    let agents = Agent::known(&installed, &known());
    assert_eq!(
        agents[0].unavailable.as_deref(),
        Some("its ACP adapter is not installed")
    );
}
