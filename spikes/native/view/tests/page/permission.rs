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

// The card's rules (`timeline::permission_card`), against the real
// adapters' requests folded from the wire: Claude Code's command, its plan
// exit with five "always" answers, Codex's two and two, a request with no
// title, a very long command, an edit with its diff, several waiting.

mod card {
    use serde_json::{Value, json};
    use vessel_app_view::timeline::{AnswerMark, CardTitle, Timeline, permission_card};

    use crate::support::{EN, Log, uuid};

    /// A timeline holding `tool` (a `tool_call` event's fields) and one
    /// request per `options` list, each for that tool call.
    fn waiting(tool: &Value, requests: &[Value]) -> Timeline {
        let mut log = Log::default();
        let mut records = vec![log.agent(&json!({
            "event": "turn_started", "content": [{ "type": "text", "text": "go" }]
        }))];
        let mut call = tool.clone();
        call["event"] = json!("tool_call");
        records.push(log.agent(&call));
        for (at, options) in (1_u32..).zip(requests) {
            records.push(log.agent(&json!({
                "event": "permission_requested", "id": uuid(at),
                "tool_call": tool["id"], "options": options
            })));
        }
        let mut timeline = Timeline::default();
        let _change = timeline.apply(&records);
        timeline
    }

    fn shell(id: &str, title: &str, command: &Value) -> Value {
        json!({
            "id": id, "title": title, "kind": "execute", "status": "pending",
            "content": [], "locations": [], "raw_input": { "command": command }
        })
    }

    fn claude_command_options(prefix: &str) -> Value {
        json!([
            { "id": "allow-once", "name": "Yes", "kind": "allow_once" },
            { "id": "allow-with-updates", "name": format!("Yes, and don't ask again for `{prefix}` commands"), "kind": "allow_always" },
            { "id": "reject", "name": "No", "kind": "reject_once" }
        ])
    }

    #[test]
    fn claude_codes_command_is_read_once_in_its_box() {
        let command = "cargo test --workspace";
        let timeline = waiting(
            &shell("call-1", command, &json!(command)),
            &[claude_command_options("cargo test")],
        );
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        // The adapter titles a shell call with its command: the title
        // falls back, the command shows once, in its box.
        assert_eq!(card.title, CardTitle::Required);
        assert_eq!(card.command.as_deref(), Some(command));
        assert_eq!(card.plan, None);
        assert_eq!(card.count, None);
        let marks: Vec<AnswerMark> = card.answers.iter().map(|answer| answer.mark).collect();
        assert_eq!(
            marks,
            [AnswerMark::Allow, AnswerMark::Allow, AnswerMark::Reject]
        );
        let primary: Vec<bool> = card.answers.iter().map(|answer| answer.primary).collect();
        assert_eq!(primary, [true, false, false]);
    }

    #[test]
    fn a_very_long_command_stays_whole_for_its_box() {
        let command = format!(
            "cargo test -- {}",
            "conversation::timeline::a_very_long_name ".repeat(12)
        );
        let timeline = waiting(
            &shell("call-1", command.trim_end(), &json!(command)),
            &[claude_command_options("cargo test")],
        );
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        assert_eq!(card.title, CardTitle::Required);
        assert_eq!(card.command.as_deref(), Some(command.trim_end()));
    }

    #[test]
    fn claude_codes_plan_exit_is_a_plan_with_its_once_answer_primary() {
        let tool = json!({
            "id": "call-plan", "title": "Ready to code?", "kind": "switch_mode", "status": "pending",
            "content": [], "locations": [], "raw_input": { "plan": "## Fix the fold\n\n1. Keep it open." }
        });
        let options = json!([
            { "id": "exit-plan-clear-auto", "name": "Yes, clear context (41% used) and use auto mode", "kind": "allow_always" },
            { "id": "exit-plan-clear-bypass", "name": "Yes, clear context (41% used) and bypass permissions", "kind": "allow_always" },
            { "id": "exit-plan-auto", "name": "Yes, and use auto mode", "kind": "allow_always" },
            { "id": "exit-plan-bypass", "name": "Yes, and bypass permissions", "kind": "allow_always" },
            { "id": "exit-plan-accept-edits", "name": "Yes, auto-accept edits", "kind": "allow_always" },
            { "id": "exit-plan-default", "name": "Yes, manually approve edits", "kind": "allow_once" },
            { "id": "reject", "name": "No, keep planning", "kind": "reject_once" }
        ]);
        let timeline = waiting(&tool, &[options]);
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        assert_eq!(card.title, CardTitle::Plan);
        assert_eq!(
            card.plan.as_deref(),
            Some("## Fix the fold\n\n1. Keep it open.")
        );
        assert_eq!(card.command, None);
        let primary: Vec<&str> = card
            .answers
            .iter()
            .filter(|answer| answer.primary)
            .map(|answer| answer.option.id.as_str())
            .collect();
        assert_eq!(primary, ["exit-plan-default"]);
    }

    #[test]
    fn codexs_shell_call_reads_as_its_script_and_its_once_allow_is_primary() {
        let script = "git status --short && git diff --stat";
        let timeline = waiting(
            &shell("call-codex", script, &json!(["bash", "-lc", script])),
            &[json!([
                { "id": "approved", "name": "Yes, proceed", "kind": "allow_once" },
                { "id": "approved-for-session", "name": "Yes, and don't ask again for this command in this session", "kind": "allow_always" },
                { "id": "approved-execpolicy-amendment", "name": "Yes, and don't ask again for commands that start with `git status`", "kind": "allow_always" },
                { "id": "denied", "name": "No, continue without running it", "kind": "reject_once" },
                { "id": "abort", "name": "No, and tell Codex what to do differently", "kind": "reject_once" }
            ])],
        );
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        assert_eq!(card.command.as_deref(), Some(script));
        assert_eq!(card.title, CardTitle::Required);
        let primary: Vec<&str> = card
            .answers
            .iter()
            .filter(|answer| answer.primary)
            .map(|answer| answer.option.id.as_str())
            .collect();
        assert_eq!(primary, ["approved"]);
        let marks: Vec<AnswerMark> = card.answers.iter().map(|answer| answer.mark).collect();
        assert_eq!(
            marks,
            [
                AnswerMark::Allow,
                AnswerMark::Allow,
                AnswerMark::Allow,
                AnswerMark::Reject,
                AnswerMark::Reject
            ]
        );
    }

    #[test]
    fn a_request_with_no_title_says_permission_required() {
        let tool = json!({
            "id": "call-1", "title": "", "kind": "other", "status": "pending",
            "content": [], "locations": []
        });
        let timeline = waiting(&tool, &[claude_command_options("x")]);
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        assert_eq!(card.title, CardTitle::Required);
        assert_eq!(card.command, None);
    }

    #[test]
    fn an_edit_keeps_its_title_and_shows_its_diff() {
        let tool = json!({
            "id": "call-edit", "title": "Edit view/src/timeline/fold.rs", "kind": "edit", "status": "pending",
            "content": [{ "type": "diff", "path": "/srv/work/view/src/timeline/fold.rs", "old_text": "a\n", "new_text": "b\n" }],
            "locations": [], "raw_input": { "file_path": "/srv/work/view/src/timeline/fold.rs" }
        });
        let timeline = waiting(&tool, &[claude_command_options("x")]);
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, 1, |kind| EN.permission_kind(kind));
        assert_eq!(
            card.title,
            CardTitle::Agent("Edit view/src/timeline/fold.rs")
        );
        assert_eq!(card.command, None);
        assert_eq!(tool.map(|tool| tool.content.len()), Some(1));
    }

    #[test]
    fn several_waiting_count_from_the_first() {
        let command = "cargo fmt --all --check";
        let timeline = waiting(
            &shell("call-1", command, &json!(command)),
            &[
                claude_command_options("cargo fmt"),
                claude_command_options("cargo fmt"),
            ],
        );
        assert_eq!(timeline.waiting_permissions(), 2);
        let (_, permission) = timeline.pending().expect("a request");
        let tool = timeline.tool_call(&permission.request.tool_call);
        let card = permission_card(permission, tool, timeline.waiting_permissions(), |kind| {
            EN.permission_kind(kind)
        });
        assert_eq!(card.count, Some((1, 2)));
        assert_eq!(EN.permission_count(1, 2), "1/2");
    }
}
