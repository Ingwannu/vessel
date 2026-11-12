//! The registry's reader against a recorded fixture in the registry's v1
//! shape (as GOAT's decoder and the registry's format give it; no
//! reference vendors a copy of the real file), and the fetch against a
//! loopback server. The registry itself is never fetched by a test.

use vessel_proto::HarnessKind;
use vessel_proto::agents::CatalogueRun;

use super::registry::{self, MAX_ENTRIES, NotTheRegistry, Platform};

/// Entries of every kind the reader meets.
const FIXTURE: &str = r#"{
  "version": "1.0.0",
  "agents": [
    {
      "id": "example-agent",
      "name": "Example Agent",
      "version": "1.2.3",
      "description": "An agent run by npx",
      "license": "MIT",
      "website": "https://example.test/agent",
      "repository": "https://example.test/agent.git",
      "distribution": {
        "npx": { "package": "@example/acp@1.2.3", "args": ["--acp"], "env": { "EXAMPLE_MODE": "acp" } }
      }
    },
    {
      "id": "python-agent",
      "name": "Python Agent",
      "version": "0.10.1",
      "description": "An agent run by uvx",
      "distribution": { "uvx": { "package": "fast-agent-acp==0.10.1", "args": ["--acp"] } }
    },
    {
      "id": "binary-agent",
      "name": "Binary Agent",
      "version": "2.0.0",
      "description": "Only archives",
      "distribution": {
        "binary": {
          "linux-x86_64": {
            "archive": "https://example.test/binary-agent-linux.tar.gz",
            "cmd": "./binary-agent",
            "sha256": "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
          }
        }
      }
    },
    {
      "id": "claude-acp",
      "name": "Claude Agent",
      "version": "0.5.0",
      "description": "One of the four, by its adapter's package",
      "distribution": { "npx": { "package": "@agentclientprotocol/claude-agent-acp@0.5.0" } }
    },
    {
      "id": "floating",
      "name": "Floating",
      "version": "1.0.0",
      "description": "Not pinned: skipped",
      "distribution": { "npx": { "package": "@example/floating@latest" } }
    },
    {
      "id": "Bad Id",
      "name": "Bad",
      "version": "1.0.0",
      "description": "An id breaking the rule: skipped",
      "distribution": {}
    },
    {
      "id": "plain-http",
      "name": "Plain",
      "version": "1.0.0",
      "description": "A site that is not https: skipped",
      "website": "http://example.test",
      "distribution": {}
    },
    "not an object"
  ]
}"#;

const KNOWN: &[(&str, HarnessKind)] = &[(
    "@agentclientprotocol/claude-agent-acp",
    HarnessKind::ClaudeCode,
)];

#[test]
fn the_registry_is_read_entry_by_entry_and_pins_are_kept() {
    let linux = Platform("linux-x86_64");
    let read = registry::read(FIXTURE.as_bytes(), Some(linux), KNOWN, |runner| {
        runner == "npx"
    })
    .unwrap();
    assert_eq!(read.version.as_str(), "1.0.0");
    let ids: Vec<&str> = read.entries.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "example-agent",
            "python-agent",
            "binary-agent",
            "claude-acp"
        ]
    );
    assert_eq!(read.skipped, 4);

    let CatalogueRun::Command {
        program,
        args,
        env,
        runner_found,
    } = &read.entries[0].runs
    else {
        panic!("a command: {:?}", read.entries[0]);
    };
    assert_eq!(program.as_str(), "npx");
    assert_eq!(args, &["-y", "@example/acp@1.2.3", "--acp"]);
    assert_eq!(env[0].name.as_str(), "EXAMPLE_MODE");
    assert!(runner_found);
    let CatalogueRun::Command {
        program,
        args,
        runner_found,
        ..
    } = &read.entries[1].runs
    else {
        panic!("a command: {:?}", read.entries[1]);
    };
    assert_eq!(program.as_str(), "uvx");
    assert_eq!(args, &["fast-agent-acp==0.10.1", "--acp"]);
    assert!(!runner_found);
    // An archive is shown, never downloaded; its hash in lower case.
    let CatalogueRun::Download { sha256, .. } = &read.entries[2].runs else {
        panic!("a download: {:?}", read.entries[2]);
    };
    assert!(sha256.as_ref().unwrap().as_str().starts_with("abcdef"));
    assert_eq!(
        read.entries[3].runs,
        CatalogueRun::Known {
            harness: HarnessKind::ClaudeCode
        }
    );

    // On another platform the archive is not there.
    let mac = registry::read(
        FIXTURE.as_bytes(),
        Some(Platform("darwin-aarch64")),
        KNOWN,
        |_| false,
    )
    .unwrap();
    assert_eq!(mac.entries[2].runs, CatalogueRun::Unavailable);
}

#[test]
fn an_envelope_that_is_not_the_registrys_is_refused_and_entries_are_bounded() {
    for refused in ["[]", "{}", r#"{"version":"1.0.0"}"#, "not json"] {
        assert!(
            matches!(
                registry::read(refused.as_bytes(), None, KNOWN, |_| true),
                Err(NotTheRegistry(_))
            ),
            "{refused}"
        );
    }
    let entry = r#"{"id":"a","name":"A","version":"1.0.0","description":"","distribution":{}}"#;
    let many = format!(
        r#"{{"version":"1.0.0","agents":[{}]}}"#,
        vec![entry; MAX_ENTRIES + 2].join(",")
    );
    let read = registry::read(many.as_bytes(), None, KNOWN, |_| true).unwrap();
    assert_eq!(read.entries.len(), MAX_ENTRIES);
    assert_eq!(read.skipped, 2);
}
