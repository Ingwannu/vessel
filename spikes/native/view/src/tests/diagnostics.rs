//! The app diagnostic's text (Paseo's `app-diagnostic-report.test.ts`):
//! its shape, and that nothing in it reaches or opens a machine.

use vessel_proto::{HarnessKind, HarnessRef, ProtocolVersion};

use crate::board::{Agent, Machine, NodeVersion, Reach};
use crate::diagnostics::{AppFacts, Renderer, redact, report};

fn app() -> AppFacts {
    AppFacts {
        collected_at: "2026-11-14T12:00:00Z".to_owned(),
        label: "0.0.0.0.16".to_owned(),
        platform: "windows x86_64".to_owned(),
        renderer: Renderer::Gpu {
            device: "NVIDIA GeForce RTX 4070".to_owned(),
            driver: "NVIDIA".to_owned(),
        },
        preferences: "C:\\Users\\me\\AppData\\Roaming\\vessel\\app-prefs.json".to_owned(),
        log: None,
        home: None,
    }
}

fn agent(kind: HarnessKind, name: &str, unavailable: Option<&str>) -> Agent {
    Agent {
        harness: HarnessRef::Known(kind),
        name: name.to_owned(),
        unavailable: unavailable.map(str::to_owned),
        not_installed: false,
    }
}

#[test]
fn lists_the_app_then_each_machine_in_paseos_shape() {
    let laptop = Machine {
        node: None,
        connection_epoch: None,
        name: "laptop".to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: vec![
            agent(HarnessKind::ClaudeCode, "Claude Code", None),
            agent(HarnessKind::Codex, "Codex", Some("not signed in")),
        ],
        protocol: Some(ProtocolVersion::new(18)),
        version: Some(NodeVersion::new("0.1.0")),
    };
    let text = report(&app(), &[laptop]);
    assert_eq!(
        text,
        "Vessel app diagnostics\n  Collected at: 2026-11-14T12:00:00Z\n  App version: 0.0.0.0.16\n  \
         Platform: windows x86_64\n  Renderer: GPU, NVIDIA GeForce RTX 4070 (NVIDIA)\n  \
         Saved machines: 1\n  Preferences: C:\\Users\\me\\AppData\\Roaming\\vessel\\app-prefs.json\n  \
         Log folder: none\n\nMachine: laptop\n  Status: online\n  Version: 0.1.0\n  Protocol: 18\n  Sessions: 0\n  \
         Agent: Claude Code, available\n  Agent: Codex, unavailable: not signed in"
    );
}

#[test]
fn says_why_a_machine_is_unreachable_without_its_address() {
    let away = Machine {
        node: None,
        connection_epoch: None,
        name: "gpu-box".to_owned(),
        reach: Reach::Unreachable(
            "connecting to ws://10.0.0.7:7077 failed (connection refused)".to_owned(),
        ),
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
        version: None,
    };
    let text = report(&app(), &[away]);
    assert!(
        text.contains(
            "  Status: unreachable: connecting to [redacted] failed (connection refused)"
        )
    );
    assert!(text.contains("  Protocol: unknown"));
    assert!(!text.contains("10.0.0.7"));
}

#[test]
fn takes_out_addresses_keys_links_and_secrets() {
    for (raw, kept) in [
        ("via wss://relay.example:443/r/abc", "via [redacted]"),
        ("(http://127.0.0.1:53863)", "([redacted]"),
        ("open vessel://pair?k=abc", "open [redacted]"),
        ("key x25519:txUDLxMXos-Hw", "key x25519:[redacted]"),
        ("token=abcdef1234", "token=[redacted]"),
        ("Authorization: Bearer", "Authorization: [redacted]"),
        ("relay_token: s3cr3t next", "relay_token: [redacted] next"),
        ("Claude Code, available", "Claude Code, available"),
    ] {
        assert_eq!(redact(raw), kept, "{raw}");
    }
}

#[test]
fn shows_the_home_folder_as_a_tilde() {
    let facts = AppFacts {
        preferences: "/home/someone/.config/vessel/app-prefs.json".to_owned(),
        log: Some("/home/someone/.local/state/vessel/logs".to_owned()),
        home: Some("/home/someone/".to_owned()),
        ..app()
    };
    let text = report(&facts, &[]);
    assert!(text.contains("  Preferences: ~/.config/vessel/app-prefs.json"));
    assert!(text.contains("  Log folder: ~/.local/state/vessel/logs"));
    assert!(!text.contains("someone"));
    // A root home changes nothing.
    let root = AppFacts {
        home: Some("/".to_owned()),
        ..facts
    };
    assert!(report(&root, &[]).contains("/home/someone/.config"));
}
