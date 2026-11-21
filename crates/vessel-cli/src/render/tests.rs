//! Tests of the plain-text renderings.

use chrono::DateTime;
use vessel_proto::{HarnessKind, SessionId, SessionSpec, TerminalSize};

use super::*;

#[test]
fn a_model_call_shows_its_counts_never_content() {
    let call: ModelCall = serde_json::from_str(
        r#"{"route":"qwen-coder","model":"qwen/qwen3-coder","api":"openai_chat_completions",
            "usage":{"input":72,"cache_read":128,"output":50},"stop":"tool_use","duration_ms":1234}"#,
    )
    .unwrap();
    assert_eq!(
        model_call(&call),
        "[model qwen-coder qwen/qwen3-coder: 72 in (+128 cached, +0 to cache), 50 out, tool use, 1.2s]"
    );
    let failed: ModelCall = serde_json::from_str(
        r#"{"route":"r","model":"m","api":"anthropic_messages","stop":"error",
            "failure":"context_too_long","duration_ms":40}"#,
    )
    .unwrap();
    assert_eq!(
        model_call(&failed),
        "[model r m: 0 in, 0 out, failed: context_too_long, 40.0ms]"
    );
}

#[test]
fn options_show_each_value_by_its_name() {
    let list: Vec<SessionOption> = serde_json::from_str(
        r#"[{"id":"model","name":"Model","category":"model","kind":"select","current":"haiku",
             "values":[{"id":"haiku","name":"Haiku"}]},
            {"id":"mode","name":"Mode","category":"mode","kind":"select","current":"plan",
             "values":[]},
            {"id":"fast","name":"Fast","category":"other","kind":"toggle","on":true}]"#,
    )
    .unwrap();
    assert_eq!(options(&list), "Model Haiku, Mode plan, Fast on");
    assert_eq!(options(&[]), "none");
}

#[test]
fn profiles_show_variable_names_and_labels() {
    let profiles = [
        ProfileInfo {
            name: "claude".parse().unwrap(),
            label: None,
            harness: HarnessKind::ClaudeCode.into(),
            sets: Vec::new(),
            model: None,
        },
        ProfileInfo {
            name: "claude-alt".parse().unwrap(),
            label: Some("second account".to_owned()),
            harness: HarnessKind::ClaudeCode.into(),
            sets: vec![
                "CLAUDE_CONFIG_DIR".parse().unwrap(),
                "ANTHROPIC_BASE_URL".parse().unwrap(),
            ],
            model: None,
        },
        ProfileInfo {
            name: "claude-qwen".parse().unwrap(),
            label: None,
            harness: HarnessKind::ClaudeCode.into(),
            sets: Vec::new(),
            model: Some("qwen-coder".parse().unwrap()),
        },
    ];
    let text = profiles_text(&profiles);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(
        lines[2].starts_with("claude-qwen ") && lines[2].contains(" qwen-coder "),
        "{text}"
    );
    assert!(
        lines[0].starts_with("claude ") && lines[0].contains(" - "),
        "{text}"
    );
    assert!(
        lines[1].contains("CLAUDE_CONFIG_DIR,ANTHROPIC_BASE_URL")
            && lines[1].ends_with("second account"),
        "{text}"
    );
    assert_eq!(profiles_text(&[]), "no profiles\n");
}

fn profiles_text(profiles: &[ProfileInfo]) -> String {
    super::profiles(profiles)
}

#[test]
fn model_routes_show_provider_api_and_models() {
    let routes = [
        ModelRoute {
            name: "qwen-coder".parse().unwrap(),
            provider: "openrouter".parse().unwrap(),
            api: ModelApi::OpenAiChatCompletions,
            model: vessel_proto::ModelId::new("qwen/qwen3-coder"),
            small_model: Some(vessel_proto::ModelId::new("qwen/qwen3-coder-flash")),
        },
        ModelRoute {
            name: "sonnet".parse().unwrap(),
            provider: "anthropic".parse().unwrap(),
            api: ModelApi::AnthropicMessages,
            model: vessel_proto::ModelId::new("claude-sonnet-4-5"),
            small_model: None,
        },
    ];
    let text = models(&routes);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with("qwen-coder "), "{text}");
    assert!(lines[0].contains(" openrouter "), "{text}");
    assert!(lines[0].contains(" openai-chat-completions "), "{text}");
    assert!(
        lines[0].ends_with("qwen/qwen3-coder (small: qwen/qwen3-coder-flash)"),
        "{text}"
    );
    assert!(lines[1].ends_with(" claude-sonnet-4-5"), "{text}");
    assert!(models(&[]).starts_with("no model routes"));
}

#[test]
fn sessions_render_one_line_each() {
    let session = {
        let mut info = SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(
                HarnessKind::Codex,
                SessionMode::Terminal {
                    args: vec!["--full-auto".to_owned()],
                    size: TerminalSize::default(),
                },
            ),
            DateTime::UNIX_EPOCH,
        );
        info.status = SessionStatus::Exited { code: None };
        info
    };
    let text = sessions(std::slice::from_ref(&session));
    assert!(text.starts_with(&session.id.to_string()));
    assert!(text.contains("codex"));
    assert!(text.contains("exited (killed)"));
    assert!(text.contains("terminal --full-auto"));
    assert!(text.ends_with("1970-01-01 00:00:00\n"));
    assert_eq!(sessions(&[]), "no sessions\n");

    let mut marked = session;
    marked.marks.pinned = true;
    marked.marks.unread = true;
    assert!(
        sessions(std::slice::from_ref(&marked)).ends_with("00:00:00  [pinned, unread]\n"),
        "{}",
        sessions(&[marked])
    );
}

/// A kept worktree says why in words, never as a debug dump, and what
/// removes it; a deleted session without a worktree says nothing more.
#[test]
fn a_kept_worktree_says_why_and_how_to_remove_it() {
    use vessel_proto::WorktreeId;
    use vessel_proto::conversation::{SessionDeleted, TidiedWorktree};
    use vessel_proto::workspace::{GitOperation, KeptReason, RemoveWorktreeOutcome};

    let kept = RemoveWorktreeOutcome::Kept {
        diff_stat: None,
        reasons: vec![
            KeptReason::Uncommitted { files: 3 },
            KeptReason::UniqueCommits { commits: 1 },
            KeptReason::Operation {
                operation: GitOperation::Rebase,
            },
        ],
    };
    let text = removal(&kept);
    assert!(
        text.starts_with(
            "kept: 3 files not committed; 1 commit no other branch or tag has; a rebase in \
             progress\n"
        ),
        "{text}"
    );
    assert!(text.contains("--snapshot"), "{text}");
    let worktree = WorktreeId::random();
    let text = deleted(&SessionDeleted {
        worktree: Some(TidiedWorktree {
            worktree,
            outcome: kept,
        }),
    });
    assert!(
        text.starts_with(&format!("its worktree {worktree} was kept: 3 files")),
        "{text}"
    );
    assert!(
        text.contains(&format!("vessel worktree remove {worktree} --snapshot")),
        "{text}"
    );
    let removed = SessionDeleted {
        worktree: Some(TidiedWorktree {
            worktree,
            outcome: RemoveWorktreeOutcome::Removed { snapshot: None },
        }),
    };
    assert_eq!(
        deleted(&removed),
        format!("its worktree {worktree} was removed\n")
    );
    assert_eq!(deleted(&SessionDeleted { worktree: None }), "");
}

#[test]
fn refusals_failures_and_stages_are_told_in_words() {
    use vessel_proto::workspace::{CloneStage, SetupFailure, WorktreeRefusal};

    assert_eq!(
        refusal(&WorktreeRefusal::Limit),
        "as many worktrees as allowed already (on the node, or for this agent)"
    );
    assert_eq!(
        setup_failure(SetupFailure::Exit {
            command: 0,
            code: Some(2)
        }),
        "command 1 exited with 2"
    );
    assert_eq!(clone_stage(CloneStage::CheckingOut), "checking out files");
}
