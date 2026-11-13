//! `shapes`: one turn with the shapes a real agent's conversation has,
//! for the native app's and the page's screens to be checked against
//! (the owner, on the app's build 0.0.0.0.5: screens had only been seen
//! with the fakes' short texts, and broke on real ones).
//!
//! The turn, in order: a long thought; a message with a heading, a table
//! with a long cell, a code block wider than any window, a list, a link
//! and Korean paragraphs; a read of a file with a long path; an edit whose
//! diff has wide lines; a command whose title and input are a long shell
//! line, asking for permission with four answers, and once allowed,
//! completing with thousands of lines of output; a closing message in
//! Korean. A refusal fails the command and the turn still ends.
//!
//! The numbers a check counts on are constants here ([`OUTPUT_LINES`],
//! [`WIDE`], [`STARTUP_LINES`]), so a fleet that makes this session can
//! say it still holds them.
//!
//! The start-up log a real adapter writes before any prompt
//! ([`startup_log`]) is a shape too, but of the session, not of a turn:
//! `session/new` writes it when `FAKE_ACP_STARTUP_LOG` is set.

use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{
    Content, ContentChunk, Diff, PermissionOption, PermissionOptionKind, RequestPermissionOutcome,
    RequestPermissionRequest, SessionId, SessionNotification, SessionUpdate, StopReason, ToolCall,
    ToolCallContent, ToolCallLocation, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    ToolKind,
};
use tokio_util::sync::CancellationToken;

use super::text;

/// Lines of the command's output.
pub(crate) const OUTPUT_LINES: usize = 2400;

/// Lines of the start-up log ([`startup_log`]).
pub(crate) const STARTUP_LINES: usize = 10;

/// What a real Claude Code's ACP adapter writes to its error output while
/// it creates session `session`, before the first prompt (the owner's
/// session on build 0.0.0.0.7, its timings kept): the node keeps each
/// line as a `HarnessLog` record, so a conversation opens with them.
/// `session/new` writes these when `FAKE_ACP_STARTUP_LOG` is set.
pub(crate) fn startup_log(session: &str) -> [String; STARTUP_LINES] {
    let create = |phase: &str, took: u32, total: u32| {
        format!(
            "[session/create] sessionId={session} phase={phase} durationMs={took} totalMs={total}"
        )
    };
    [
        create("settings", 40, 41),
        format!("[session/query] sessionId={session} resume=none apiType=native baseUrl=native"),
        create("prepare-query", 20, 61),
        create("sdk-initialize", 1179, 1240),
        format!(
            "[session/models] sessionId={session} phase=set-model durationMs=73 model=opus outcome=success"
        ),
        create("models", 81, 1321),
        create("modes", 0, 1321),
        create("agents", 0, 1321),
        create("ultracode", 131, 1452),
        create("register", 7, 1459),
    ]
}

/// The width of the widest lines (the code block's, the diff's), in
/// characters.
pub(crate) const WIDE: usize = 240;

/// The command the agent asks to run: one long shell line.
const COMMAND: &str = "cargo test --workspace --all-features --locked -- --test-threads 1 \
--nocapture conversation::timeline::a_very_long_test_name_that_keeps_going_past_any_row \
2>&1 | tee /tmp/vessel-shapes/test-run-with-a-long-name.log";

/// `shapes`: the turn above, then `end_turn` (`cancelled` when cancelled
/// while asking).
pub(crate) async fn shapes(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    say(SessionUpdate::AgentThoughtChunk(ContentChunk::new(text(
        &thought(),
    ))))?;
    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
        &message(),
    ))))?;
    say(SessionUpdate::ToolCall(read()))?;
    say(SessionUpdate::ToolCall(edit()))?;
    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
        "Now the tests, which take a while.",
    ))))?;
    say(SessionUpdate::ToolCall(
        ToolCall::new("shapes-run", COMMAND)
            .kind(ToolKind::Execute)
            .status(ToolCallStatus::Pending)
            .raw_input(serde_json::json!({
                "command": COMMAND,
                "cwd": "/srv/work/vessel/crates/vessel-web/frontend/src/features/session",
                "timeout_ms": 600_000,
            })),
    ))?;
    let answer = cx
        .send_request(RequestPermissionRequest::new(
            session.clone(),
            ToolCallUpdate::new("shapes-run", ToolCallUpdateFields::new()),
            vec![
                PermissionOption::new("allow-once", "Allow", PermissionOptionKind::AllowOnce),
                PermissionOption::new(
                    "allow-always",
                    "Always allow `cargo test` in this project",
                    PermissionOptionKind::AllowAlways,
                ),
                PermissionOption::new("reject-once", "Reject", PermissionOptionKind::RejectOnce),
                PermissionOption::new(
                    "reject-always",
                    "Never allow commands that write outside the project",
                    PermissionOptionKind::RejectAlways,
                ),
            ],
        ))
        .block_task()
        .await?;
    let allowed = match answer.outcome {
        RequestPermissionOutcome::Selected(selected) => selected.option_id.0.starts_with("allow"),
        RequestPermissionOutcome::Cancelled => return Ok(StopReason::Cancelled),
        _ => false,
    };
    let fields = if allowed {
        ToolCallUpdateFields::new()
            .status(ToolCallStatus::Completed)
            .content(vec![ToolCallContent::Content(Content::new(
                text(&output()),
            ))])
            .raw_output(serde_json::json!({ "exit_code": 0, "lines": OUTPUT_LINES }))
    } else {
        ToolCallUpdateFields::new().status(ToolCallStatus::Failed)
    };
    say(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        "shapes-run",
        fields,
    )))?;
    if turn.is_cancelled() {
        return Ok(StopReason::Cancelled);
    }
    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
        "테스트를 모두 돌렸습니다. 실패한 것은 없고, 가장 오래 걸린 것은 \
         `conversation::timeline` 묶음이었습니다. 다음으로 긴 대화에서 스크롤이 \
         끊기지 않는지 확인하겠습니다.",
    ))))?;
    Ok(StopReason::EndTurn)
}

/// A thought of several paragraphs.
fn thought() -> String {
    [
        "The failure is in the timeline's fold, not in the transport: the records arrive \
         in order, and the entry that goes missing is the second chunk of a message that \
         follows a tool call.",
        "Two places could drop it: the batch boundary, where the last entry is extended, \
         or the index from a tool call's id, which is only updated on a new call.",
        "I will read the fold first, then the batching in the shell, then run the tests \
         that replay a recorded turn.",
    ]
    .join("\n\n")
}

/// A message with every kind of Markdown a reply has.
fn message() -> String {
    let wide = format!(
        "let {} = vec![{}];",
        "a_name_long_enough_to_need_scrolling",
        (0..40)
            .map(|n| format!("\"value-{n:02}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let wide: String = wide.chars().take(WIDE).collect();
    format!(
        "## What I found\n\n\
         The fold drops a chunk when a batch ends **inside** a message. Here is what \
         each part does, with the file that holds it:\n\n\
         | Part | File | Lines | What it does | Status |\n\
         | --- | --- | ---: | --- | --- |\n\
         | Fold | `view/src/timeline/fold.rs` | 412 | Folds records into entries, changing the last one in place when a chunk continues it | changed |\n\
         | Rows | `view/src/timeline/rows.rs` | 88 | Which entries are drawn once runs of tool calls fold | unchanged |\n\
         | Window | `view/src/timeline/window.rs` | 190 | What is near the view | unchanged |\n\
         | Find | `view/src/timeline/find.rs` | 380 | Matches in prompts and replies | unchanged |\n\n\
         The line that mattered:\n\n\
         ```rust\n{wide}\nlet next = record.seq.next().ok_or(Full)?;\n```\n\n\
         - It now extends the entry the batch began with.\n\
         - The index by tool call id is kept across batches.\n\
         - See [the page's reducer](https://github.com/Team-WICKED/vessel) for the same rule.\n\n\
         긴 대화에서도 같은 문제가 생기는지 확인했습니다. 한 묶음이 메시지 중간에서 끝나면 \
         다음 묶음의 첫 조각이 새 항목으로 들어가던 것이 원인이었고, 이제는 마지막 항목을 \
         그대로 이어 붙입니다.\n\n\
         한국어 문단이 여러 줄로 이어질 때도 줄바꿈과 자간이 페이지와 같은지 함께 봐 주세요."
    )
}

/// A read of a file deep in the tree.
fn read() -> ToolCall {
    let path =
        "/srv/work/vessel/spikes/native/app/src/areas/conversation/timeline/rows/folded_runs.rs";
    ToolCall::new("shapes-read", format!("Read {path}"))
        .kind(ToolKind::Read)
        .status(ToolCallStatus::Completed)
        .locations(vec![ToolCallLocation::new(path).line(Some(42))])
        .raw_input(serde_json::json!({ "path": path, "offset": 1, "limit": 400 }))
        .content(vec![ToolCallContent::Content(Content::new(text(
            "400 lines read",
        )))])
}

/// An edit whose diff has lines wider than any window.
fn edit() -> ToolCall {
    let path = "/srv/work/vessel/spikes/native/view/src/timeline/fold.rs";
    let line = |n: usize, word: &str| {
        let body = format!(
            "    let {word}_{n} = timeline.entries().iter().filter(|entry| entry.seq > since).map(|entry| entry.key.clone()).collect::<Vec<_>>(); // {}",
            "x".repeat(WIDE)
        );
        body.chars().take(WIDE).collect::<String>()
    };
    let old: Vec<String> = (0..30).map(|n| line(n, "old")).collect();
    let new: Vec<String> = (0..45).map(|n| line(n, "new")).collect();
    ToolCall::new("shapes-edit", format!("Edit {path}"))
        .kind(ToolKind::Edit)
        .status(ToolCallStatus::Completed)
        .locations(vec![ToolCallLocation::new(path)])
        .content(vec![ToolCallContent::Diff(
            Diff::new(path, new.join("\n")).old_text(Some(old.join("\n"))),
        )])
}

/// The command's output: thousands of numbered lines, as a test run
/// prints them.
fn output() -> String {
    (1..=OUTPUT_LINES)
        .map(|n| {
            format!(
                "test conversation::timeline::case_{n:04}_folds_a_batch_that_ends_inside_a_message ... ok ({} ms)",
                n.checked_rem(97).unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::v1::schema::{ToolCallContent, ToolKind};

    use super::{OUTPUT_LINES, WIDE, edit, message, output, read};

    /// The longest line of `text`, in characters.
    fn widest(text: &str) -> usize {
        text.lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
    }

    // What the screens are checked against must not quietly become short
    // again: the fleet's check (`spikes/native/scripts/fleet.sh`) sees only
    // what `vessel log` prints, which leaves out a tool's output, so the
    // numbers are held here.
    #[test]
    fn the_output_has_its_thousands_of_lines() {
        assert_eq!(output().lines().count(), OUTPUT_LINES);
    }

    #[test]
    fn the_message_holds_a_table_wide_code_and_korean() {
        let message = message();
        assert_eq!(widest(&message), WIDE);
        assert!(message.lines().filter(|line| line.starts_with('|')).count() >= 4);
        assert!(message.contains("```rust"));
        assert!(
            message
                .chars()
                .any(|c| ('\u{ac00}'..='\u{d7a3}').contains(&c))
        );
    }

    #[test]
    fn the_startup_log_has_its_lines_and_names_the_session() {
        let lines = super::startup_log("fake-session-1");
        assert_eq!(lines.len(), super::STARTUP_LINES);
        assert!(
            lines
                .iter()
                .all(|line| line.contains("sessionId=fake-session-1"))
        );
        assert!(lines.iter().all(|line| line.starts_with("[session/")));
    }

    #[test]
    fn the_diff_is_wide_and_long() {
        let call = edit();
        let Some(ToolCallContent::Diff(diff)) = call.content.first() else {
            panic!("the edit carries a diff");
        };
        assert_eq!(widest(&diff.new_text), WIDE);
        assert_eq!(diff.new_text.lines().count(), 45);
        assert_eq!(
            diff.old_text.as_deref().map(|old| old.lines().count()),
            Some(30)
        );
        assert_eq!(read().kind, ToolKind::Read);
    }
}
