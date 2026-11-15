//! `fake-acp`: a scripted ACP agent on stdio, standing in for an ACP
//! harness or adapter (`claude-agent-acp`, `gemini --experimental-acp`).
//!
//! Behaviour:
//!
//! - `--version` prints `fake-acp <version>`; with `FAKE_ACP_EXIT_EARLY`
//!   set it writes `fake-acp: not logged in` to stderr and exits 3 before
//!   speaking ACP (a harness that fails at start). Otherwise it writes
//!   `fake-acp starting` to stderr and serves ACP v1 until stdin closes.
//! - With `FAKE_ACP_ENV_FILE` set, it first writes the names (never the
//!   values) of its environment's variables there, sorted, one a line: what
//!   a test of an added agent's clean environment reads (ADR 0032).
//! - With `FAKE_ACP_SIGN_IN` set, `initialize` lists one sign-in method
//!   (`fake-login`, "Fake login") and `session/new` answers ACP's
//!   `auth_required` (what `agent.status` reads, ADR 0032). `initialize`
//!   always names it: `fake-acp` and the crate's version.
//! - With `--slow-start <ms>` among its arguments, `initialize` is answered
//!   that many milliseconds late: an agent that takes a while to start
//!   (the page's new chat is timed against one, `web/e2e/new-chat.spec.ts`;
//!   an added agent's arguments are how a test gives it).
//! - With `FAKE_ACP_HANG` set to `initialize` or `session/new`, that request
//!   is never answered (an adapter stuck at start); stuck at `session/new`,
//!   it says so on stderr first (`fake-acp: stuck at session/new`), so a
//!   test knows `initialize` went through.
//! - `initialize`: protocol v1, `loadSession` supported (not with
//!   `FAKE_ACP_NO_LOAD` set: an agent that cannot resume), HTTP MCP servers
//!   accepted (not with `FAKE_ACP_NO_HTTP_MCP` set), images in prompts
//!   (audio too with `FAKE_ACP_AUDIO` set), and the steering extension (`_meta.steering.supported`; not with
//!   `FAKE_ACP_NO_STEER` set). It remembers whether the client advertised
//!   form elicitation, for `ask`, and when it did, writes `fake-acp: the
//!   client takes questions` to stderr.
//! - `_session/steering` (the steering extension, not with
//!   `FAKE_ACP_NO_STEER` set: "method not found" then): during a `wait`
//!   turn, `injected` and a message chunk `steered: <text>` (a steer
//!   `finish` also ends the turn); with no such turn, `promptRequired`.
//!   Any other extension method is "method not found".
//! - Right after answering `session/new` or `session/load`, an
//!   `available_commands_update`: `compact` (with a hint) and `review`.
//! - `session/new`: session `fake-session-<n>-<tag>` (`<tag>` this
//!   process's own, so no two fakes share an id, as no two real agents
//!   do), with the options `FAKE_ACP_OPTIONS` picks (`options` module): by default config
//!   options `mode`, `model` (`fake-small`, the default, or `fake-large`),
//!   `effort` and `fast`. With `FAKE_ACP_STARTUP_LOG` set it first writes
//!   a real adapter's start-up log to stderr (`shapes::startup_log`).
//! - `session/fork` (protocol 14; offered unless `FAKE_ACP_NO_FORK` is
//!   set): session `fork~<source>~<at>`, `<at>` the anchor's message id
//!   (`_meta.jetbrains.air.fork.messageId`, as the real adapters read it)
//!   or `head`; with `FAKE_ACP_FORK_IGNORES_POINT` set always `head`, an
//!   agent that ignores the anchor. Options as `session/new`.
//! - `session/delete` (offered): says `fake-acp: deleted <id>` on stderr,
//!   so a test sees a refused fork deleted.
//! - `session/list` (protocol 17; offered unless `FAKE_ACP_NO_LIST` is
//!   set, "method not found" then): three earlier conversations over two
//!   pages (`history` module); no session is started by it.
//! - `session/load`: replays the conversation as a user message chunk
//!   `earlier question` (then, with `FAKE_ACP_REPLAY` set to a number,
//!   that many agent message chunks `replayed line <n>`, a long
//!   conversation) and an agent message chunk `replayed <id>` whose
//!   message id is where a fork was made (`<at>`, `head-last` for `head`,
//!   `replayed-last` for any other session), then
//!   answers with the same options as `session/new`, as they stand. With
//!   `FAKE_ACP_LOAD_FAILS` set it replays nothing and answers an error
//!   (`no such session: <id>`), as an agent whose transcript is gone does.
//! - Both remember the MCP servers they were given, and `session/load` the
//!   session id it loaded.
//! - `session/set_config_option`: sets an option to a value it offers
//!   (`mode` never to `locked`), rejects anything else; `session/set_mode`
//!   the same for the older modes.
//! - `session/prompt`, by prompt text:
//!   - `plain`: one message chunk `echo: plain` with a message id
//!     (`msg-<n>`, new each time), then `end_turn`;
//!   - `wait`: nothing until `session/cancel`, then `cancelled`, or until
//!     a steer `finish`, then `end_turn`;
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
//!   - `shapes`: a turn with a real conversation's shapes (`shapes` module);
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
//!   - `describe` (with whatever else the prompt holds): one message chunk
//!     saying each block of the prompt, `; `-separated (`describe`
//!     module): `text <len>`, `image <type> <n> bytes` and `audio <type>
//!     <n> bytes` (the base64 decoded), `link <name> <type> <size> read
//!     <n>` (the bytes of the file its `file:` URI names, or
//!     `unreadable`), then `end_turn`;
//!   - `usage-burst`: 101 cumulative cost updates, for sparse read-only pages;
//!     `FAKE_ACP_LOAD_USAGE` replays .5 USD while loading, before Ready;
//!     its `higher` mode replays 10 USD. `usage-after-load` reports 12 USD
//!     and `usage-later` 14 USD, with the same vendor response shape.
//!   - `usage-reasoning`: as `usage`, with reported zero reasoning/write and 12 cached reads;
//!   - `usage`: a `usage_update` (1,200 of 200,000 tokens, 0.5 USD), one
//!     message chunk `used`, then `end_turn` with the turn's tokens (30:
//!     20 in, 10 out); `usage <tokens>` uses that many of the 200,000 tokens;
//!   - `report <path>`: writes its arguments, its whole environment and
//!     the MCP servers of the last `session/new` or `session/load` to that
//!     file as JSON (`vessel_testkit::write_report`), then one message
//!     chunk `reported`;
//!   - `model <text>`: one model call through the node's proxy, as a
//!     routed session's harness would make it (`vessel_testkit::model`),
//!     then one message chunk `model <reply>` or `model error: <why>`;
//!   - `ask`, `ask withdraw`: a question (protocol 10's `elicitation/create`
//!     in form mode, `ask` module) when the client advertised form
//!     elicitation, then one message chunk `answered: <action>` (with the
//!     accepted content as JSON); `ask withdraw` cancels its own request
//!     first (`$/cancel_request`). Without the capability, one message
//!     chunk `no questions`. `ask link`: a URL-mode elicitation, sent
//!     whatever the client advertised, reported the same way; `ask
//!     claude`: Claude's `AskUserQuestion` form (two questions, each with
//!     its free-text "Other"), reported the same way; `ask questions` and
//!     `ask one`: the same question built as the adapter builds it, three
//!     questions (long option descriptions, one choosing several, one in
//!     Korean) or the first alone (`ask` module);
//!   - `permit plan`, `permit codex`, `permit two`: permission requests in
//!     the real adapters' shapes (Claude Code leaving plan mode, Codex's
//!     five answers, two at once; `permits` module), then one message
//!     chunk `permitted: <option>` per request;
//!   - `plans`: a changing plan in Claude Code shape (`plans` module),
//!     followed by `plans done`;
//!   - `fail`: one message chunk `failing`, then the prompt's request
//!     answers an error (`fake-acp: the turn failed`) and the agent stays
//!     up: a turn that fails with the agent alive;
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
    AgentCapabilities, AuthMethod, AuthMethodAgent, AvailableCommand, AvailableCommandInput,
    AvailableCommandsUpdate, CancelNotification, Content, ContentBlock, ContentChunk, Cost,
    DeleteSessionRequest, DeleteSessionResponse, ElicitationCapabilities, ForkSessionRequest,
    ForkSessionResponse, Implementation, InitializeRequest, InitializeResponse,
    ListSessionsRequest, LoadSessionRequest, LoadSessionResponse, McpCapabilities, McpServer,
    MessageId, NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind, Plan,
    PlanEntry, PlanEntryPriority, PlanEntryStatus, PromptCapabilities, PromptRequest,
    PromptResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionCapabilities,
    SessionDeleteCapabilities, SessionForkCapabilities, SessionId, SessionListCapabilities,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    ToolKind, UnstructuredCommandInput, Usage, UsageUpdate,
};
use agent_client_protocol::{Agent, ByteStreams, Error, UntypedMessage};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;

#[path = "fake_acp/ask.rs"]
mod ask;
#[path = "fake_acp/describe.rs"]
mod describe;
#[path = "fake_acp/history.rs"]
mod history;
#[path = "fake_acp/options.rs"]
mod options;
#[path = "fake_acp/permits.rs"]
mod permits;
#[path = "fake_acp/plans.rs"]
mod plans;
#[path = "fake_acp/real.rs"]
mod real;
#[path = "fake_acp/shapes.rs"]
mod shapes;

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
    if let Some(file) = std::env::var_os("FAKE_ACP_ENV_FILE") {
        let mut names: Vec<String> = std::env::vars_os()
            .map(|(name, _value)| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        let mut text = names.join("\n");
        text.push('\n');
        if let Err(error) = std::fs::write(&file, text) {
            complain(&format!(
                "fake-acp: cannot write the environment's names: {error}"
            ));
            return ExitCode::FAILURE;
        }
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
    /// Ends the running `wait` turn with `end_turn`: a `wait` turn is the
    /// one a steer reaches.
    steerable: Option<CancellationToken>,
    /// From the last `session/new` or `session/load`.
    mcp_servers: Vec<McpServer>,
    /// The id the last `session/load` got.
    loaded: Option<String>,
    /// The working directory of the last `session/new` or `session/load`.
    cwd: Option<std::path::PathBuf>,
    /// Whether the client advertised form elicitation (protocol 10).
    questions: bool,
}

impl State {
    fn new() -> Self {
        Self {
            sessions: 0,
            options: options::Options::from_env(),
            turn: None,
            steerable: None,
            mcp_servers: Vec::new(),
            loaded: None,
            cwd: None,
            questions: false,
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

/// What the fake offers as commands, as Claude's adapter sends them.
fn commands(session: SessionId) -> SessionNotification {
    SessionNotification::new(
        session,
        SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(vec![
            AvailableCommand::new("compact", "Clear history but keep a summary").input(
                AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(
                    "<optional instructions>",
                )),
            ),
            AvailableCommand::new("review", "Review the changes"),
        ])),
    )
}

type Shared = Arc<Mutex<State>>;

fn lock(state: &Shared) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How late `initialize` is answered: `--slow-start <ms>` among the
/// arguments.
fn slow_start() -> Option<std::time::Duration> {
    let millis = std::env::args()
        .skip_while(|arg| arg != "--slow-start")
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    Some(std::time::Duration::from_millis(millis))
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
    let slow_start = slow_start();
    let http_mcp = std::env::var_os("FAKE_ACP_NO_HTTP_MCP").is_none();
    let steer = std::env::var_os("FAKE_ACP_NO_STEER").is_none();
    let loads = std::env::var_os("FAKE_ACP_NO_LOAD").is_none();
    let hears = std::env::var_os("FAKE_ACP_AUDIO").is_some();
    let startup_log = std::env::var_os("FAKE_ACP_STARTUP_LOG").is_some();
    let load_usage = std::env::var_os("FAKE_ACP_LOAD_USAGE")
        .map(|mode| if mode == "higher" { 10.0 } else { 0.5 });
    let load_fails = std::env::var_os("FAKE_ACP_LOAD_FAILS").is_some();
    let signs_in = std::env::var_os("FAKE_ACP_SIGN_IN").is_some();
    let forks = std::env::var_os("FAKE_ACP_NO_FORK").is_none();
    let ignores_point = std::env::var_os("FAKE_ACP_FORK_IGNORES_POINT").is_some();
    let lists = std::env::var_os("FAKE_ACP_NO_LIST").is_none();
    let replay: u32 = std::env::var("FAKE_ACP_REPLAY")
        .ok()
        .and_then(|lines| lines.parse().ok())
        .unwrap_or(0);
    let state: Shared = Arc::new(Mutex::new(State::new()));
    let transport = ByteStreams::new(
        tokio::io::stdout().compat_write(),
        tokio::io::stdin().compat(),
    );
    let served = Agent
        .v1()
        .name("fake-acp")
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: InitializeRequest, responder, _cx| {
                    if hang_at_initialize {
                        std::future::pending::<()>().await;
                    }
                    if let Some(wait) = slow_start {
                        tokio::time::sleep(wait).await;
                    }
                    let questions = request
                        .client_capabilities
                        .elicitation
                        .as_ref()
                        .is_some_and(ElicitationCapabilities::supports_form);
                    lock(&state).questions = questions;
                    if questions {
                        complain("fake-acp: the client takes questions");
                    }
                    let mut answer = InitializeResponse::new(request.protocol_version)
                        .agent_capabilities(
                            AgentCapabilities::new()
                                .load_session(loads)
                                .prompt_capabilities(
                                    PromptCapabilities::new().image(true).audio(hears),
                                )
                                .mcp_capabilities(McpCapabilities::new().http(http_mcp))
                                .session_capabilities(
                                    SessionCapabilities::new()
                                        .fork(forks.then(SessionForkCapabilities::new))
                                        .delete(SessionDeleteCapabilities::new())
                                        .list(lists.then(SessionListCapabilities::new)),
                                ),
                        )
                        .agent_info(Implementation::new("fake-acp", env!("CARGO_PKG_VERSION")));
                    if signs_in {
                        answer = answer.auth_methods(vec![AuthMethod::Agent(
                            AuthMethodAgent::new("fake-login", "Fake login"),
                        )]);
                    }
                    if steer {
                        answer.meta = serde_json::json!({ "steering": { "supported": true } })
                            .as_object()
                            .cloned();
                    }
                    responder.respond(answer)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: NewSessionRequest, responder, cx| {
                    if hang_at_new {
                        complain("fake-acp: stuck at session/new");
                        std::future::pending::<()>().await;
                    }
                    if signs_in {
                        return responder.respond_with_error(Error::auth_required());
                    }
                    let response = {
                        let mut state = lock(&state);
                        state.mcp_servers = request.mcp_servers;
                        state.cwd = Some(request.cwd);
                        state.sessions = state.sessions.saturating_add(1);
                        let id = format!("fake-session-{}-{}", state.sessions, process_tag());
                        let mut response = NewSessionResponse::new(id);
                        response.config_options = state.options.config();
                        response.modes = state.options.modes();
                        response
                    };
                    let session = response.session_id.clone();
                    if startup_log {
                        for line in shapes::startup_log(&session.0) {
                            complain(&line);
                        }
                    }
                    responder.respond(response)?;
                    cx.send_notification(commands(session))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: DeleteSessionRequest, responder, _cx| {
                // Said on stderr, so a test sees it in the session's log.
                complain(&format!("fake-acp: deleted {}", request.session_id.0));
                responder.respond(DeleteSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ListSessionsRequest, responder, _cx| {
                if !lists {
                    return responder.respond_with_error(Error::method_not_found());
                }
                complain("fake-acp: listing its conversations");
                match history::page(request.cwd, request.cursor.as_deref()) {
                    Ok(page) => responder.respond(page),
                    Err(error) => responder.respond_with_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: ForkSessionRequest, responder, cx| {
                    let point = fork_point(request.meta.as_ref());
                    let at = match &point {
                        Some(point) if !ignores_point => point.as_str(),
                        _ => "head",
                    };
                    let id = format!("fork~{}~{at}", request.session_id.0);
                    let response = {
                        let mut state = lock(&state);
                        state.mcp_servers = request.mcp_servers;
                        state.cwd = Some(request.cwd);
                        let mut response = ForkSessionResponse::new(id);
                        response.config_options = state.options.config();
                        response.modes = state.options.modes();
                        response
                    };
                    let session = response.session_id.clone();
                    responder.respond(response)?;
                    cx.send_notification(commands(session))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: LoadSessionRequest, responder, cx| {
                    if load_fails {
                        return responder.respond_with_error(
                            Error::invalid_params()
                                .data(format!("no such session: {}", request.session_id.0)),
                        );
                    }
                    let say = |update| {
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            update,
                        ))
                    };
                    // Real adapters can replay their cumulative session
                    // cost before load's response and the driver's Ready.
                    if let Some(amount) = load_usage {
                        say(SessionUpdate::UsageUpdate(
                            UsageUpdate::new(1200, 200_000).cost(Cost::new(amount, "USD")),
                        ))?;
                    }
                    say(SessionUpdate::UserMessageChunk(ContentChunk::new(text(
                        "earlier question",
                    ))))?;
                    // A long conversation (`FAKE_ACP_REPLAY`): what an
                    // import of one costs.
                    for line in 1..=replay {
                        say(SessionUpdate::AgentMessageChunk(ContentChunk::new(text(
                            &format!("replayed line {line}\n"),
                        ))))?;
                    }
                    // A fork's replay ends where it was forked (protocol
                    // 14): its id says so.
                    let mut replayed =
                        ContentChunk::new(text(&format!("replayed {}", request.session_id.0)));
                    replayed.message_id =
                        Some(MessageId::new(replayed_message(&request.session_id.0)));
                    say(SessionUpdate::AgentMessageChunk(replayed))?;
                    if let Ok(replay) = std::env::var("FAKE_ACP_REPLAY_USER") {
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::UserMessageChunk(ContentChunk::new(text(&replay))),
                        ))?;
                    }
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
                    responder.respond(response)?;
                    cx.send_notification(commands(request.session_id.clone()))
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
                    let finish = CancellationToken::new();
                    let prompt = prompt_text(&request.prompt);
                    let (model, report) = {
                        let mut state = lock(&state);
                        state.turn = Some(turn.clone());
                        state.steerable = (prompt == "wait").then(|| finish.clone());
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
                    if prompt == "usage-burst" {
                        for amount in 1_u32..=101 {
                            cx.send_notification(SessionNotification::new(
                                request.session_id.clone(),
                                SessionUpdate::UsageUpdate(
                                    UsageUpdate::new(1200, 200_000)
                                        .cost(Cost::new(f64::from(amount), "USD")),
                                ),
                            ))?;
                        }
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text("used"))),
                        ))?;
                        return responder.respond(PromptResponse::new(StopReason::EndTurn));
                    }
                    let used = prompt
                        .strip_prefix("usage")
                        .and_then(|tokens| tokens.trim().parse::<u64>().ok());
                    if used.is_some()
                        || matches!(
                            prompt.as_str(),
                            "usage" | "usage-reasoning" | "usage-after-load" | "usage-later"
                        )
                    {
                        let amount = match prompt.as_str() {
                            "usage-after-load" => 12.0,
                            "usage-later" => 14.0,
                            _ => 0.5,
                        };
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::UsageUpdate(
                                UsageUpdate::new(used.unwrap_or(1200), 200_000)
                                    .cost(Cost::new(amount, "USD")),
                            ),
                        ))?;
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text("used"))),
                        ))?;
                        let usage = if prompt == "usage-reasoning" {
                            Usage::new(30, 20, 10)
                                .thought_tokens(0)
                                .cached_read_tokens(12)
                                .cached_write_tokens(0)
                        } else {
                            Usage::new(30, 20, 10)
                        };
                        return responder
                            .respond(PromptResponse::new(StopReason::EndTurn).usage(usage));
                    }
                    if prompt == "limit" {
                        return responder.respond(PromptResponse::new(StopReason::MaxTokens));
                    }
                    if prompt == "fail" {
                        // The agent stays up: only this turn's request
                        // answers an error, as an agent's failed model
                        // call does.
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text("failing"))),
                        ))?;
                        return responder.respond_with_error(
                            Error::internal_error().data("fake-acp: the turn failed"),
                        );
                    }
                    if prompt == "describe" {
                        let described = describe::blocks(&request.prompt);
                        cx.send_notification(SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(text(&described))),
                        ))?;
                        return responder.respond(PromptResponse::new(StopReason::EndTurn));
                    }
                    if let Some(asked) = ask::wanted(&prompt) {
                        let form = lock(&state).questions;
                        // The turn waits for the client's answer, so it
                        // runs off the dispatch loop.
                        return cx.spawn({
                            let cx = cx.clone();
                            async move {
                                let reason =
                                    ask::turn(&cx, &request.session_id, form, asked, &turn).await?;
                                responder.respond(PromptResponse::new(reason))
                            }
                        });
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
                        let state = Arc::clone(&state);
                        async move {
                            let reason = run_turn(
                                &cx,
                                &request.session_id,
                                &prompt,
                                &model,
                                (&turn, &finish),
                            )
                            .await?;
                            lock(&state).steerable = None;
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
        .on_receive_request(
            {
                let state = Arc::clone(&state);
                async move |request: UntypedMessage,
                            responder: agent_client_protocol::v1::Responder<serde_json::Value>,
                            cx: agent_client_protocol::v1::ConnectionTo<
                    agent_client_protocol::Client,
                >| {
                    if !steer || request.method != "_session/steering" {
                        return responder.respond_with_error(Error::method_not_found());
                    }
                    let said = request
                        .params
                        .get("prompt")
                        .and_then(|prompt| prompt.get(0))
                        .and_then(|block| block.get("text"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let session = request
                        .params
                        .get("sessionId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let steerable = lock(&state).steerable.clone();
                    let Some(finish) = steerable else {
                        return responder
                            .respond(serde_json::json!({ "outcome": "promptRequired" }));
                    };
                    cx.send_notification(SessionNotification::new(
                        SessionId::new(session),
                        SessionUpdate::AgentMessageChunk(ContentChunk::new(text(&format!(
                            "steered: {said}"
                        )))),
                    ))?;
                    responder.respond(serde_json::json!({ "outcome": "injected" }))?;
                    if said == "finish" {
                        finish.cancel();
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
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

/// What makes this process's session ids its own: real agents mint ids
/// no other process repeats (UUIDs), and a node refuses two live sessions
/// on one conversation (protocol 17), so two fakes must not both say
/// `fake-session-1`. Its process id and start time, in hex.
fn process_tag() -> &'static str {
    static TAG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TAG.get_or_init(|| {
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        format!("{:x}{started:x}", std::process::id())
    })
}

/// A message id, new each time: `msg-<n>` (protocol 14's points).
fn next_message() -> String {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    format!(
        "msg-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// The anchor of a `session/fork`, as both real adapters read it:
/// `_meta.jetbrains.air.fork { version: 1, messageId }`.
fn fork_point(meta: Option<&serde_json::Map<String, serde_json::Value>>) -> Option<String> {
    let fork = meta?.get("jetbrains")?.get("air")?.get("fork")?;
    if fork.get("version")?.as_u64()? != 1 {
        return None;
    }
    fork.get("messageId")?.as_str().map(str::to_owned)
}

/// The id of a replay's last agent message: for a fork (`fork~<source>~<at>`)
/// the message it was forked at, `head-last` for a fork of the whole
/// conversation; `replayed-last` otherwise.
fn replayed_message(session: &str) -> String {
    match session.rsplit_once('~') {
        Some((_, "head")) => "head-last".to_owned(),
        Some((_, at)) if session.starts_with("fork~") => at.to_owned(),
        _ => "replayed-last".to_owned(),
    }
}

async fn run_turn(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Client>,
    session: &SessionId,
    prompt: &str,
    model: &str,
    (turn, finish): (&CancellationToken, &CancellationToken),
) -> Result<StopReason, Error> {
    let say = |update| cx.send_notification(SessionNotification::new(session.clone(), update));
    match prompt {
        "plain" => {
            // An id, as Claude's and Codex's adapters give theirs: the
            // turn's point (protocol 14).
            let mut chunk = ContentChunk::new(text("echo: plain"));
            chunk.message_id = Some(MessageId::new(next_message()));
            say(SessionUpdate::AgentMessageChunk(chunk))?;
            return Ok(StopReason::EndTurn);
        }
        "wait" => {
            return Ok(tokio::select! {
                () = turn.cancelled() => StopReason::Cancelled,
                () = finish.cancelled() => StopReason::EndTurn,
            });
        }
        "exit" | "garbage" => return failing_turn(cx, session, prompt == "exit", turn).await,
        "flood" => return flood(cx, session, turn).await,
        "plans" => return plans::turn(cx, session, turn),
        "shapes" => return shapes::shapes(cx, session, turn).await,
        _ => {}
    }
    if let Some(permit) = permits::wanted(prompt) {
        return permits::turn(cx, session, permit, turn).await;
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
