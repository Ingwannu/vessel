//! The page's `data/sync.test.ts`.

mod failures;
mod projection;
mod reports;
mod sentences;

use chrono::{DateTime, Utc};
use vessel_proto::{FleetMember, Generation, Manifest};

use crate::Lang;
use crate::machines::StateTone;
use crate::sync::{MemberState, member_line, secret_names, with_conflicts};

fn now() -> DateTime<Utc> {
    "2026-11-11T07:00:00Z".parse().unwrap()
}

fn member() -> FleetMember {
    serde_json::from_value(serde_json::json!({
        "device": "00000000-0000-4000-8000-000000000001",
        "label": "gpu-box",
        "applied": 3,
        "conflicts": [],
        "reported_at": "2026-11-11T06:59:00Z",
    }))
    .unwrap()
}

#[test]
fn is_up_to_date_behind_by_some_versions_or_not_reported_yet() {
    let words = Lang::En.machines();
    let state = |member: &FleetMember, generation| {
        let state = MemberState::of(member, Generation::new(generation));
        (words.member_state(state), state.tone())
    };
    assert_eq!(
        state(&member(), 3),
        ("Up to date".to_owned(), StateTone::Ok)
    );
    assert_eq!(
        state(&member(), 5),
        ("2 behind".to_owned(), StateTone::Warn)
    );
    let never = FleetMember {
        applied: None,
        ..member()
    };
    assert_eq!(state(&never, 5).0, "Not yet");
}

#[test]
fn says_what_it_applied_its_kept_edits_and_its_last_report() {
    assert_eq!(
        member_line(Lang::En, &member(), now()),
        ["Applied version 3", "no kept edits", "reported 1m ago"]
    );
    let edited = FleetMember {
        conflicts: vec!["AGENTS.md".parse().unwrap()],
        ..member()
    };
    assert_eq!(member_line(Lang::En, &edited, now())[1], "1 kept edit");
    let members = [member(), edited.clone()];
    assert_eq!(with_conflicts(&members), [&edited]);
}

#[test]
fn the_secrets_the_shared_tool_servers_name_are_names_only_each_once_sorted() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "generation": 1,
        "files": {},
        "mcp": {
            "search": {
                "type": "stdio",
                "command": "search-mcp",
                "env": { "KEY": { "secret": "SEARCH_KEY" }, "MODE": "fast" },
            },
            "docs": {
                "type": "http",
                "url": "https://docs.example/mcp",
                "headers": { "Authorization": { "secret": "DOCS_TOKEN" } },
            },
        },
    }))
    .unwrap();
    assert_eq!(secret_names(&manifest), ["DOCS_TOKEN", "SEARCH_KEY"]);
}
