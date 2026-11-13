//! Permission requests in the real adapters' shapes, for the native app's
//! permission card to be checked against (the owner on build 0.0.0.0.7:
//! the card was rebuilt as Paseo's, against real requests):
//!
//! - `permit plan`: Claude Code leaving plan mode (`ExitPlanMode`): the
//!   tool's input carries the plan's Markdown, the request offers five
//!   "always" answers (clear the context or not, auto mode, bypass,
//!   auto-accept edits), one "once" answer and a reject, as
//!   `claude-agent-acp`'s `buildExitPlanModePermissionOptions` sends them.
//! - `permit codex`: Codex asking to run a command: one "once" allow, two
//!   "always" allows, two "once" rejects, as `codex-acp`'s
//!   `commandDecisionOptions` sends them.
//! - `permit two`: two commands asking at once, the second while the
//!   first still waits.
//!
//! After the answers, one message chunk per request: `permitted: <option>`
//! (or `permitted: cancelled`). The turn ends `cancelled` if it was
//! cancelled meanwhile.

use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{
    ContentChunk, PermissionOption, PermissionOptionKind, RequestPermissionOutcome,
    RequestPermissionRequest, SessionId, SessionNotification, SessionUpdate, StopReason, ToolCall,
    ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use tokio_util::sync::CancellationToken;

/// A permission prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Permit {
    /// `permit plan`.
    Plan,
    /// `permit codex`.
    Codex,
    /// `permit two`.
    Two,
}

/// The permission prompt `prompt` is, if it is one.
pub(crate) fn wanted(prompt: &str) -> Option<Permit> {
    match prompt {
        "permit plan" => Some(Permit::Plan),
        "permit codex" => Some(Permit::Codex),
        "permit two" => Some(Permit::Two),
        _ => None,
    }
}

/// The plan Claude Code proposes, in Markdown.
const PLAN: &str = "## Fix the fold\n\n\
    1. Keep the entry a batch ends inside open across batches.\n\
    2. Index tool calls by id across batches, not only within one.\n\
    3. Replay a recorded turn split at every record and compare.\n\n\
    한국어 줄도 하나: 긴 대화에서 스크롤이 끊기지 않는지 함께 확인합니다.";

/// The command Codex asks to run.
const CODEX_COMMAND: &str = "git status --short && git diff --stat";

fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
    PermissionOption::new(id.to_owned(), name.to_owned(), kind)
}

fn plan_options() -> Vec<PermissionOption> {
    use PermissionOptionKind::{AllowAlways, AllowOnce, RejectOnce};
    vec![
        option(
            "exit-plan-clear-auto",
            "Yes, clear context (41% used) and use auto mode",
            AllowAlways,
        ),
        option(
            "exit-plan-clear-bypass",
            "Yes, clear context (41% used) and bypass permissions",
            AllowAlways,
        ),
        option("exit-plan-auto", "Yes, and use auto mode", AllowAlways),
        option(
            "exit-plan-bypass",
            "Yes, and bypass permissions",
            AllowAlways,
        ),
        option(
            "exit-plan-accept-edits",
            "Yes, auto-accept edits",
            AllowAlways,
        ),
        option(
            "exit-plan-default",
            "Yes, manually approve edits",
            AllowOnce,
        ),
        option("reject", "No, keep planning", RejectOnce),
    ]
}

fn codex_options() -> Vec<PermissionOption> {
    use PermissionOptionKind::{AllowAlways, AllowOnce, RejectOnce};
    vec![
        option("approved", "Yes, proceed", AllowOnce),
        option(
            "approved-for-session",
            "Yes, and don't ask again for this command in this session",
            AllowAlways,
        ),
        option(
            "approved-execpolicy-amendment",
            "Yes, and don't ask again for commands that start with `git status`",
            AllowAlways,
        ),
        option("denied", "No, continue without running it", RejectOnce),
        option(
            "abort",
            "No, and tell Codex what to do differently",
            RejectOnce,
        ),
    ]
}

/// Claude Code's three answers for a command.
fn command_options(prefix: &str) -> Vec<PermissionOption> {
    use PermissionOptionKind::{AllowAlways, AllowOnce, RejectOnce};
    vec![
        option("allow-once", "Yes", AllowOnce),
        option(
            "allow-with-updates",
            &format!("Yes, and don't ask again for `{prefix}` commands"),
            AllowAlways,
        ),
        option("reject", "No", RejectOnce),
    ]
}

/// One tool call and its request: what the tool is, what it was given.
struct Asked {
    id: &'static str,
    title: &'static str,
    kind: ToolKind,
    input: serde_json::Value,
    options: Vec<PermissionOption>,
}

fn asked(permit: Permit) -> Vec<Asked> {
    match permit {
        Permit::Plan => vec![Asked {
            id: "permit-plan",
            title: "Ready to code?",
            kind: ToolKind::SwitchMode,
            input: serde_json::json!({ "plan": PLAN }),
            options: plan_options(),
        }],
        Permit::Codex => vec![Asked {
            id: "permit-codex",
            title: CODEX_COMMAND,
            kind: ToolKind::Execute,
            input: serde_json::json!({ "command": ["bash", "-lc", CODEX_COMMAND], "cwd": "/srv/work/vessel" }),
            options: codex_options(),
        }],
        Permit::Two => vec![
            Asked {
                id: "permit-first",
                title: "cargo fmt --all --check",
                kind: ToolKind::Execute,
                input: serde_json::json!({ "command": "cargo fmt --all --check" }),
                options: command_options("cargo fmt"),
            },
            Asked {
                id: "permit-second",
                title: "cargo clippy --workspace --all-targets -- -D warnings",
                kind: ToolKind::Execute,
                input: serde_json::json!({ "command": "cargo clippy --workspace --all-targets -- -D warnings" }),
                options: command_options("cargo clippy"),
            },
        ],
    }
}

/// Run a permission prompt's turn: every request sent before any answer
/// is awaited, so several wait at once.
pub(crate) async fn turn(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    permit: Permit,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    let mut pending = Vec::new();
    for asked in asked(permit) {
        say(SessionUpdate::ToolCall(
            ToolCall::new(asked.id, asked.title)
                .kind(asked.kind)
                .status(ToolCallStatus::Pending)
                .raw_input(asked.input),
        ))?;
        pending.push(cx.send_request(RequestPermissionRequest::new(
            session.clone(),
            ToolCallUpdate::new(asked.id, ToolCallUpdateFields::new()),
            asked.options,
        )));
    }
    for request in pending {
        let line = match request.block_task().await?.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                format!("permitted: {}", selected.option_id.0)
            }
            RequestPermissionOutcome::Cancelled | _ => "permitted: cancelled".to_owned(),
        };
        say(SessionUpdate::AgentMessageChunk(ContentChunk::new(
            super::text(&line),
        )))?;
    }
    Ok(if turn.is_cancelled() {
        StopReason::Cancelled
    } else {
        StopReason::EndTurn
    })
}
