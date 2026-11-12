//! `fake-acp`: a scripted ACP agent on stdio, standing in for an ACP
//! harness or adapter (`claude-agent-acp`, `gemini --experimental-acp`).
//!
//! Behaviour:
//!
//! - `--version` prints `fake-acp <version>`; with `FAKE_ACP_EXIT_EARLY`
//!   set it writes `fake-acp: not logged in` to stderr and exits 3 before
//!   speaking ACP (a harness that fails at start). Otherwise it writes
//!   `fake-acp starting` to stderr and serves ACP v1 until stdin closes.
//! - With `FAKE_ACP_HANG` set to `initialize` or `session/new`, that request
//!   is never answered (an adapter stuck at start).
//! - `initialize`: protocol v1, `loadSession` supported, HTTP MCP servers
//!   accepted (not with `FAKE_ACP_NO_HTTP_MCP` set).
//! - `session/new`: session `fake-session-<n>`, with the options
//!   `FAKE_ACP_OPTIONS` picks (`options` module): by default config
//!   options `mode`, `model` (`fake-small`, the default, or `fake-large`),
//!   `effort` and `fast`.
//! - `session/load`: replays the conversation as a user message chunk
//!   `earlier question` and an agent message chunk `replayed <id>`, then
//!   answers with the same options as `session/new`, as they stand.
//! - Both remember the MCP servers they were given, and `session/load` the
//!   session id it loaded.
//! - `session/set_config_option`: sets an option to a value it offers
//!   (`mode` never to `locked`), rejects anything else; `session/set_mode`
//!   the same for the older modes.
//! - `session/prompt`, by prompt text:
//!   - `plain`: one message chunk `echo: plain`, then `end_turn`;
//!   - `wait`: nothing until `session/cancel`, then `cancelled`;
//!   - `flood`: message chunks `flood`, ten a millisecond (faster than a
//!     slow client reads) until `session/cancel`, then `cancelled`; it
//!     gives up with `end_turn` after a minute;
//!   - `entries <n>`: `n` updates that stay `n` entries in the page's
//!     timeline (its reducer merges consecutive chunks of one kind, so
//!     the kinds take turns): a message chunk with a short Markdown
//!     paragraph, a thought chunk, a completed tool call `entry-<i>` (raw
//!     input, one text content), and again; then `end_turn`. A count that
//!     is not a number gives one message chunk `error: entries <n>` (the
//!     page's timeline measurement, `docs/PERFORMANCE.md`);
//!   - `env NAME`: one message chunk `env NAME=<value>` or `env NAME
//!     unset`, from the fake's own environment, then `end_turn`;
//!   - `mcp`: one message chunk `mcp <JSON>`, the MCP servers of the last
//!     `session/new` or `session/load` as ACP sent them (values included);
//!   - `loaded`: one message chunk `loaded <id>` (the id `session/load`
//!     got) or `loaded none`;
//!   - `options`: one message chunk `options mode=<m> model=<m>
//!     effort=<e> fast=<bool>`, the values the fake runs with;
//!   - `switch <mode>`: the agent changes its own mode and reports it
//!     (`config_option_update`, or `current_mode_update` for the older
//!     modes), then one message chunk `switched <mode>`;
//!   - `cwd`: one message chunk `cwd <path>`, the working directory of the
//!     last `session/new` or `session/load`;
//!   - `args`: one message chunk `args <JSON array of its arguments>`;
//!   - `report <path>`: writes its arguments, its whole environment and
//!     the MCP servers of the last `session/new` or `session/load` to that
//!     file as JSON (`vessel_testkit::write_report`), then one message
//!     chunk `reported`;
//!   - `model <text>`: one model call through the node's proxy, as a
//!     routed session's harness would make it (`vessel_testkit::model`),
//!     then one message chunk `model <reply>` or `model error: <why>`;
//!   - `exit`, `garbage`: a message chunk and a permission request, then,
//!     with both still open, the connection fails: `exit` exits with code
//!     9, `garbage` writes a line that is not UTF-8 and stays up;
//!   - anything else: a thought chunk, a message chunk
//!     `echo: <text> (model <model>)`, tool call `call-1` (pending, with raw
//!     input), a plan, an update to `in_progress`, then a permission request
//!     with `allow-once` / `reject-once`. Allow completes the call (content
//!     and raw output), reject fails it, and both end the turn with
//!     `end_turn`; a cancelled answer ends it with `cancelled`.
//! - `session/cancel` cancels the turn in progress.

use std::io::{self, Write as _};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_client_protocol::v1::schema::{
    AgentCapabilities, CancelNotification, Content, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, LoadSessionRequest, LoadSessionResponse,
    McpCapabilities, McpServer, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, PromptRequest,
    PromptResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionId,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    ToolKind,
};
use agent_client_protocol::{Agent, ByteStreams, Error};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;

#[path = "fake_acp/options.rs"]
mod options;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        let written = writeln!(
            io::stdout().lock(),
            "fake-acp {}",
            env!("CARGO_PKG_VERSION")
        );
        drop(written);
        return ExitCode::SUCCESS;
    }
    if std::env::var_os("FAKE_ACP_EXIT_EARLY").is_some() {
        complain("fake-acp: not logged in");
        return ExitCode::from(3);
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            complain(&format!("fake-acp: no runtime: {error}"));
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(serve())
}

fn complain(line: &str) {
    let written = writeln!(io::stderr().lock(), "{line}");
    drop(written);
}

/// What the fake remembers between requests.
#[derive(Debug)]
struct State {
    sessions: u32,
    options: options::Options,
    turn: Option<CancellationToken>,
    /// From the last `session/new` or `session/load`.
    mcp_servers: Vec<McpServer>,
    /// The id the last `session/load` got.
    loaded: Option<String>,
    /// The working directory of the last `session/new` or `session/load`.
    cwd: Option<std::path::PathBuf>,
}

impl State {
    fn new() -> Self {
        Self {
            sessions: 0,
            options: options::Options::from_env(),
            turn: None,
            mcp_servers: Vec::new(),
            loaded: None,
            cwd: None,
        }
    }
}

/// The answer to a prompt that reports what the fake was given, if it is
/// one (`env NAME`, `mcp`, `loaded`, `cwd`, `args`, `report <path>`).
fn report(prompt: &str, state: &State) -> Option<String> {
    if let Some(path) = prompt.strip_prefix("report ") {
        let mcp = serde_json::to_value(&state.mcp_servers).ok();
        return Some(
            match vessel_testkit::write_report(std::path::Path::new(path.trim()), mcp) {
                Ok(()) => "reported".to_owned(),
                Err(error) => format!("error: no report: {error}"),
            },
        );
    }
    if let Some(text) = prompt.strip_prefix("model ") {
        // Blocks this fake's one thread for the call; nothing else of the
        // session runs meanwhile, as in a turn.
        return Some(vessel_testkit::model::call(text));
    }
    if let Some(name) = prompt.strip_prefix("env ") {
        return Some(match std::env::var_os(name) {
            Some(value) => format!("env {name}={}", value.to_string_lossy()),
            None => format!("env {name} unset"),
        });
    }
    match prompt {
        "options" => Some(state.options.describe()),
        "mcp" => Some(format!(
            "mcp {}",
            serde_json::to_string(&state.mcp_servers).unwrap_or_default()
        )),
        "loaded" => Some(format!(
            "loaded {}",
            state.loaded.as_deref().unwrap_or("none")
        )),
        "cwd" => Some(format!(
            "cwd {}",
            state
                .cwd
                .as_deref()
                .map_or_else(|| "none".into(), std::path::Path::to_string_lossy)
        )),
        "args" => {
            let args: Vec<String> = std::env::args_os()
                .skip(1)
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            Some(format!("args {}", serde_json::Value::from(args)))
        }
        _ => None,
    }
}

type Shared = Arc<Mutex<State>>;

fn lock(state: &Shared) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

#[expect(
    clippy::too_many_lines,
    reason = "one handler registration per ACP method, kept in one readable chain"
)]
async fn serve() -> ExitCode {
    complain("fake-acp starting");
    let hang = std::env::var("FAKE_ACP_HANG").unwrap_or_default();
    let hang_at_initialize = hang == "initialize";
    let hang_at_new = hang == "session/new";
    let http_mcp = std::env::var_os("FAKE_ACP_NO_HTTP_MCP").is_none();
    let state: Shared = Arc::new(Mutex::new(State::new()));
    let transport = ByteStreams::new(
        tokio::io::stdout().compat_write(),
        tokio::io::stdin().compat(),
    );
    let served = Agent
        .v1()
        .name("fake-acp")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _cx| {
                if hang_at_initialize {
                    std::future::pending::<()>().await;
                }
                responder.respond(
                    InitializeResponse::new(request.protocol_version)
                        .agent_capabilities(
                            AgentCapabilities::new()
                                .load_session(true)
                                .mcp_capabilities(McpCapabilities::new().http(http_mcp)),
                        )
                        .agent_info(Implementation::new("fake-acp", env!("CARGO_PKG_VERSION"))),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: NewSessionRequest, responder, _cx| {
                    if hang_at_new {
                        std::future::pending::<()>().await;
                    }
                    let response = {
                        let mut state = lock(&state);
                        state.mcp_servers = request.mcp_servers;
                        state.cwd = Some(request.cwd);
                        state.sessions = state.sessions.saturating_add(1);
                        let id = format!("fake-session-{}", state.sessions);
                        let mut response = NewSessionResponse::new(id);
                        response.config_options = state.options.config();
                        response.modes = state.options.modes();
                        response
                    };
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: LoadSessionRequest, responder, cx| {
                    let say = |update| {
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            update,
                        ))
                    };
                    say(SessionUpdate::UserMessageChunk(ContentChunk::new(text(
                        "earlier question",
                    ))))?;
                    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
                        &format!("replayed {}", request.session_id.0),
                    ))))?;
                    let response = {
                        let mut state = lock(&state);
                        state.mcp_servers.clone_from(&request.mcp_servers);
                        state.loaded = Some(request.session_id.0.to_string());
                        state.cwd = Some(request.cwd.clone());
                        let mut response = LoadSessionResponse::new();
                        response.config_options = state.options.config();
                        response.modes = state.options.modes();
                        response
                    };
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: SetSessionConfigOptionRequest, responder, _cx| {
                    let set = lock(&state)
                        .options
                        .set(&request.config_id.0, &request.value);
                    match set {
                        Ok(options) => {
                            responder.respond(SetSessionConfigOptionResponse::new(options))
                        }
                        Err(error) => responder.respond_with_error(error),
                    }
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: SetSessionModeRequest, responder, _cx| {
                    let set = lock(&state).options.set_mode(&request.mode_id.0);
                    match set {
                        Ok(()) => responder.respond(SetSessionModeResponse::new()),
                        Err(error) => responder.respond_with_error(error),
                    }
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: PromptRequest, responder, cx| {
                    let turn = CancellationToken::new();
                    let prompt = prompt_text(&request.prompt);
                    let (model, report) = {
                        let mut state = lock(&state);
                        state.turn = Some(turn.clone());
                        let report = report(&prompt, &state);
                        (state.options.model.clone(), report)
                    };
                    if let Some(mode) = prompt.strip_prefix("switch ") {
                        let update = lock(&state).options.switch(mode.trim());
                        if let Some(update) = update {
                            cx.send_notification(SessionNotification::new(
                                request.session_id.clone(),
                                update,
                            ))?;
                        }
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text(&format!(
                                "switched {}",
                                mode.trim()
                            )))),
                        ))?;
                        return responder.respond(PromptResponse::new(StopReason::EndTurn));
                    }
                    if let Some(report) = report {
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text(&report))),
                        ))?;
                        return responder.respond(PromptResponse::new(StopReason::EndTurn));
                    }
                    // The turn waits for a permission answer, so it runs off
                    // the dispatch loop.
                    cx.spawn({
                        let cx = cx.clone();
                        async move {
                            let reason =
                                run_turn(&cx, &request.session_id, &prompt, &model, &turn).await?;
                            responder.respond(PromptResponse::new(reason))
                        }
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let state = Arc::clone(&state);
                async move |_cancel: CancelNotification, _cx| {
                    if let Some(turn) = lock(&state).turn.take() {
                        turn.cancel();
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(transport)
        .await;
    match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            complain(&format!("fake-acp: {error}"));
            ExitCode::FAILURE
        }
    }
}

async fn run_turn(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    prompt: &str,
    model: &str,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    match prompt {
        "plain" => {
            say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
                "echo: plain",
            ))))?;
            return Ok(StopReason::EndTurn);
        }
        "wait" => {
            turn.cancelled().await;
            return Ok(StopReason::Cancelled);
        }
        "exit" | "garbage" => return failing_turn(cx, session, prompt == "exit", turn).await,
        "flood" => return flood(cx, session, turn).await,
        _ => {}
    }
    if let Some(count) = prompt.strip_prefix("entries ") {
        return entries(cx, session, count, turn).await;
    }
    say(SessionUpdate::AgentThoughtChunk(ContentChunk::new(text(
        "thinking",
    ))))?;
    say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
        &format!("echo: {prompt} (model {model})"),
    ))))?;
    say(SessionUpdate::ToolCall(
        ToolCall::new("call-1", "Run fake tool")
            .kind(ToolKind::Execute)
            .status(ToolCallStatus::Pending)
            .raw_input(serde_json::json!({ "command": "fake", "args": ["--fast"] })),
    ))?;
    say(SessionUpdate::Plan(Plan::new(vec![PlanEntry::new(
        "Run the fake tool",
        PlanEntryPriority::High,
        PlanEntryStatus::InProgress,
    )])))?;
    say(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        "call-1",
        ToolCallUpdateFields::new().status(ToolCallStatus::InProgress),
    )))?;
    let answer = cx
        .send_request(RequestPermissionRequest::new(
            session.clone(),
            ToolCallUpdate::new("call-1", ToolCallUpdateFields::new()),
            vec![
                PermissionOption::new("allow-once", "Allow", PermissionOptionKind::AllowOnce),
                PermissionOption::new("reject-once", "Reject", PermissionOptionKind::RejectOnce),
            ],
        ))
        .block_task()
        .await?;
    let allowed = match answer.outcome {
        RequestPermissionOutcome::Selected(selected) => &*selected.option_id.0 == "allow-once",
        RequestPermissionOutcome::Cancelled => return Ok(StopReason::Cancelled),
        _ => false,
    };
    let fields = if allowed {
        ToolCallUpdateFields::new()
            .status(ToolCallStatus::Completed)
            .content(vec![ToolCallContent::Content(Content::new(text(
                "tool output",
            )))])
            .raw_output(serde_json::json!({ "ok": true }))
    } else {
        ToolCallUpdateFields::new().status(ToolCallStatus::Failed)
    };
    say(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        "call-1", fields,
    )))?;
    if turn.is_cancelled() {
        Ok(StopReason::Cancelled)
    } else {
        Ok(StopReason::EndTurn)
    }
}

/// `flood`: ten message chunks a millisecond until cancelled; gives up
/// with `end_turn` after a minute.
async fn flood(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    for _ in 0..60_000_u32 {
        if turn.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        for _ in 0..10 {
            cx.send_notification(SessionNotification::new(
                session.clone(),
                SessionUpdate::AgentMessageChunk(ContentChunk::new(text("flood"))),
            ))?;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    Ok(StopReason::EndTurn)
}

/// The kinds `entries` takes in turn: never two of a kind in a row, so the
/// page's reducer, which merges consecutive message (or thought) chunks,
/// keeps every update as an entry of its own.
#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Message,
    Thought,
    Tool,
}

/// `entries <n>`: `n` updates of the kinds in turn, then `end_turn`.
async fn entries(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    count: &str,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    let Ok(count) = count.trim().parse::<u32>() else {
        say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
            "error: entries <n>",
        ))))?;
        return Ok(StopReason::EndTurn);
    };
    let kinds = [EntryKind::Message, EntryKind::Thought, EntryKind::Tool];
    for (index, kind) in (0..count).zip(kinds.into_iter().cycle()) {
        // Queued notifications are written by another task of this one
        // thread: yielding lets it drain them as they come (a queue of
        // 20,000 would sit in memory first) and lets a cancel in.
        tokio::task::yield_now().await;
        if turn.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        say(entry(index, kind))?;
    }
    Ok(StopReason::EndTurn)
}

/// Entry `index` of `entries`, of `kind`.
fn entry(index: u32, kind: EntryKind) -> SessionUpdate {
    match kind {
        EntryKind::Message => SessionUpdate::AgentMessageChunk(ContentChunk::new(text(&format!(
            "**Entry {index}.** A short paragraph, as an agent writes between \
             tool calls: it names `src/module_{index}.rs` and what comes next."
        )))),
        EntryKind::Thought => SessionUpdate::AgentThoughtChunk(ContentChunk::new(text(&format!(
            "Entry {index}: deciding which file to read next."
        )))),
        EntryKind::Tool => SessionUpdate::ToolCall(
            ToolCall::new(
                format!("entry-{index}"),
                format!("Read src/module_{index}.rs"),
            )
            .kind(ToolKind::Read)
            .status(ToolCallStatus::Completed)
            .raw_input(serde_json::json!({ "path": format!("src/module_{index}.rs") }))
            .content(vec![ToolCallContent::Content(Content::new(text(
                &format!("{index} lines read"),
            )))]),
        ),
    }
}

/// `exit` and `garbage`: a message chunk and a permission request, then the
/// connection fails with both open.
async fn failing_turn(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    exit: bool,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    cx.send_notification(SessionNotification::new(
        session.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(text("about to fail"))),
    ))?;
    // Sent, never awaited: the request is still open when the connection
    // fails.
    drop(cx.send_request(RequestPermissionRequest::new(
        session.clone(),
        ToolCallUpdate::new("call-9", ToolCallUpdateFields::new()),
        vec![PermissionOption::new(
            "allow-once",
            "Allow",
            PermissionOptionKind::AllowOnce,
        )],
    )));
    // Time for both to reach the client before the failure.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    fail_connection(exit);
    turn.cancelled().await;
    Ok(StopReason::Cancelled)
}

/// Break the connection under a turn: exit at once, or write a line that is
/// not UTF-8 (which ends the client's reading with an error).
#[expect(
    clippy::exit,
    reason = "the scenario is an agent that dies in the middle of a turn"
)]
fn fail_connection(exit: bool) {
    if exit {
        std::process::exit(9);
    }
    let mut stdout = io::stdout().lock();
    let written = stdout
        .write_all(b"\xff\xfe not utf-8\n")
        .and_then(|()| stdout.flush());
    drop(written);
}

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text(TextContent::new(text))
}

/// The prompt's text blocks, joined.
fn prompt_text(prompt: &[ContentBlock]) -> String {
    prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text.as_str()),
            ContentBlock::Image(_)
            | ContentBlock::Audio(_)
            | ContentBlock::ResourceLink(_)
            | ContentBlock::Resource(_)
            | _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
