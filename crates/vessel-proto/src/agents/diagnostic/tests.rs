//! The diagnostic's wire form and its text, against the fixtures the page's
//! renderer is held to as well (`crate::wire_fixtures`).

use super::*;
use crate::wire_fixtures;

#[test]
fn typescript_fixture_of_agent_diagnostics_reads_and_renders_as_paseos_text() {
    let diagnostics: Vec<AgentDiagnostic> =
        serde_json::from_str(&wire_fixtures::read("agent-diagnostic.json")).unwrap();
    assert_eq!(diagnostics.len(), 3);
    let texts: Vec<String> = diagnostics.iter().map(AgentDiagnostic::text).collect();
    let text = texts.join("\n");
    assert!(
        wire_fixtures::current("agent-diagnostic.txt", &text),
        "agent-diagnostic.txt is not what the renderer makes; rerun with VESSEL_WRITE_TS=1:\n{text}"
    );
    // Paseo's shape: the title, `  Label: value` rows, then Models and
    // Status.
    let first = texts.first().unwrap();
    assert!(first.starts_with("Claude Code\n  Command source: built in\n"));
    assert!(first.ends_with("  Models: 5\n  Status: Ready\n"));
    assert!(
        first.contains("  Search path matches: ~/.local/bin/claude\n    /usr/local/bin/claude\n")
    );
    // Each reads back as it was written.
    for diagnostic in &diagnostics {
        let again: AgentDiagnostic =
            serde_json::from_str(&serde_json::to_string(diagnostic).unwrap()).unwrap();
        assert_eq!(&again, diagnostic);
    }
}

#[test]
fn a_request_names_its_agent_and_refuses_unknown_fields() {
    let request: GetAgentDiagnostic = serde_json::from_str(r#"{"agent":"codex"}"#).unwrap();
    assert_eq!(request.profile, None);
    assert!(serde_json::from_str::<GetAgentDiagnostic>(r#"{"agent":"codex","x":1}"#).is_err());
}

#[test]
fn bounded_text_is_cut_on_a_character_boundary() {
    assert_eq!(bounded_text("abc", 8), "abc");
    assert_eq!(bounded_text("abc", 2), "ab");
    // "é" is two bytes: a cut inside it drops it.
    assert_eq!(bounded_text("aé", 2), "a");
}
