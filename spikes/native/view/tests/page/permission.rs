//! Not the page's: what a permission card's buttons say, against the
//! options the real adapters send (Claude Code's `claude-agent-acp`,
//! `permissions/options`; Codex's `codex-acp`, `permissions/options.ts`),
//! in both languages. The app's words for a kind offered once; the
//! agent's names for a kind offered more than once.

use serde_json::{Value, json};
use vessel_app_view::Asked;
use vessel_proto::PermissionOption;

use crate::support::{EN, KO, uuid};

fn asked(options: Value) -> Asked {
    let options: Vec<PermissionOption> =
        serde_json::from_value(options).expect("options the wire takes");
    Asked {
        id: serde_json::from_value(json!(uuid(1))).expect("an id"),
        what: Some("cargo test".to_owned()),
        options,
    }
}

/// The words of `asked`'s buttons, in `words`' language.
fn said(asked: &Asked, words: vessel_app_view::ConversationWords) -> Vec<String> {
    asked
        .buttons(&words)
        .into_iter()
        .map(|(_, words, _)| words)
        .collect()
}

#[test]
fn claude_codes_tool_request_once_kinds_in_the_apps_words_always_in_the_agents() {
    let asked = asked(json!([
        { "id": "allow-once", "name": "Yes", "kind": "allow_once" },
        { "id": "allow-with-updates", "name": "Yes, and don't ask again for `cargo test` commands", "kind": "allow_always" },
        { "id": "reject", "name": "No", "kind": "reject_once" }
    ]));
    let scope = "Yes, and don't ask again for `cargo test` commands";
    assert_eq!(said(&asked, *EN), ["Accept", scope, "Deny"]);
    assert_eq!(said(&asked, *KO), ["수락", scope, "거부"]);
    let allows: Vec<bool> = asked
        .buttons(KO)
        .into_iter()
        .map(|(_, _, allows)| allows)
        .collect();
    assert_eq!(allows, [true, true, false]);
}

#[test]
fn claude_codes_plan_exit_keeps_the_names_of_its_five_always_options() {
    let asked = asked(json!([
        { "id": "exit-plan-clear-auto", "name": "Yes, clear context (41% used) and use auto mode", "kind": "allow_always" },
        { "id": "exit-plan-clear-bypass", "name": "Yes, clear context (41% used) and bypass permissions", "kind": "allow_always" },
        { "id": "exit-plan-auto", "name": "Yes, and use auto mode", "kind": "allow_always" },
        { "id": "exit-plan-bypass", "name": "Yes, and bypass permissions", "kind": "allow_always" },
        { "id": "exit-plan-accept-edits", "name": "Yes, auto-accept edits", "kind": "allow_always" },
        { "id": "exit-plan-default", "name": "Yes, manually approve edits", "kind": "allow_once" },
        { "id": "reject", "name": "No, keep planning", "kind": "reject_once" }
    ]));
    let always = [
        "Yes, clear context (41% used) and use auto mode",
        "Yes, clear context (41% used) and bypass permissions",
        "Yes, and use auto mode",
        "Yes, and bypass permissions",
        "Yes, auto-accept edits",
    ];
    for (words, (accept, deny)) in [(EN, ("Accept", "Deny")), (KO, ("수락", "거부"))] {
        let mut expected: Vec<&str> = always.to_vec();
        expected.extend([accept, deny]);
        assert_eq!(said(&asked, *words), expected);
    }
}

#[test]
fn codexs_two_and_two_keep_the_agents_names_but_its_lone_once_allow() {
    let asked = asked(json!([
        { "id": "approved", "name": "Yes, proceed", "kind": "allow_once" },
        { "id": "approved-for-session", "name": "Yes, and don't ask again for this command in this session", "kind": "allow_always" },
        { "id": "approved-execpolicy-amendment", "name": "Yes, and don't ask again for commands that start with `git status`", "kind": "allow_always" },
        { "id": "denied", "name": "No, continue without running it", "kind": "reject_once" },
        { "id": "abort", "name": "No, and tell Codex what to do differently", "kind": "reject_once" }
    ]));
    for (words, accept) in [(EN, "Accept"), (KO, "수락")] {
        assert_eq!(
            said(&asked, *words),
            [
                accept,
                "Yes, and don't ask again for this command in this session",
                "Yes, and don't ask again for commands that start with `git status`",
                "No, continue without running it",
                "No, and tell Codex what to do differently",
            ]
        );
    }
}

#[test]
fn every_once_kind_and_an_always_reject() {
    let asked = asked(json!([
        { "id": "a", "name": "Allow", "kind": "allow_once" },
        { "id": "r", "name": "Reject", "kind": "reject_once" },
        { "id": "n", "name": "Never for this project", "kind": "reject_always" }
    ]));
    assert_eq!(
        said(&asked, *EN),
        ["Accept", "Deny", "Never for this project"]
    );
    assert_eq!(
        said(&asked, *KO),
        ["수락", "거부", "Never for this project"]
    );
}

#[test]
fn buttons_that_would_say_the_same_all_keep_the_agents_names() {
    // The app's "Accept" for the lone allow beside an always option the
    // agent also named "Accept".
    let asked = asked(json!([
        { "id": "once", "name": "Allow", "kind": "allow_once" },
        { "id": "always", "name": "Accept", "kind": "allow_always" }
    ]));
    assert_eq!(said(&asked, *EN), ["Allow", "Accept"]);
    assert_eq!(said(&asked, *KO), ["수락", "Accept"]);
}
