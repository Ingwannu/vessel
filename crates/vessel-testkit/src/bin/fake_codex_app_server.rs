//! `fake-codex-app-server`: stands in for `codex` where Vessel drives it
//! over its own app-server protocol (ADR 0057). Its messages are shaped as
//! `codex app-server generate-json-schema` (Codex 0.166.0) says; each
//! driver slice teaches it what it uses.
//!
//! Behaviour:
//!
//! - `--version` prints `codex-cli <FAKE_CODEX_VERSION or 0.166.0>`.
//! - `app-server` speaks JSON-RPC 2.0 without the `jsonrpc` member, one
//!   message a line, over stdio:
//!   - `initialize` answers `{ userAgent }`; notifications are read and
//!     ignored.
//!   - `thread/start` opens `fake-thread-<process id>`; `thread/resume`
//!     opens the id asked for, or fails for an id starting `missing`.
//!     Either keeps the approval policy, reviewer and sandbox it was given.
//!   - `turn/start` answers the turn (`inProgress`), says `turn/started`,
//!     and plays the scenario its first text input names:
//!     - `plain` (or anything else): a message streamed in two deltas, a
//!       delta of another thread's (which is not this session's), token
//!       usage, then `completed`;
//!     - `think`: a reasoning summary delta, then as `plain`;
//!     - `quiet`: a message that arrives whole, without deltas;
//!     - `tool`: a command run (`echo hi`), then as `plain`;
//!     - `plan`: a plan of two steps, then as `plain`;
//!     - `approve`: asks for approval to run a command (`cmd-1`, before
//!       its item starts), offering a decision with a rule among its
//!       `availableDecisions`, and waits; the item then starts and ends
//!       (`declined` for `decline` or `cancel`), and the message is
//!       `decision <the decision as JSON>`; a `cancel` interrupts the turn;
//!     - `edit`: starts a file change, asks to apply it and waits; as
//!       `approve`;
//!     - `permit`: asks for network access (request id 7, a number); the
//!       message is `granted <scope> <permissions as JSON>`;
//!     - `ask`: asks two questions (`color`, a choice of `red` or `blue`,
//!       and `name`, free text); the message is `answers <answers as
//!       JSON>`;
//!     - `form`: a tool server asks for a form (`name`, a string); the
//!       message is `action <action> <content as JSON>`;
//!     - `secret`: asks a secret question; as `ask`;
//!     - `resolved`: asks to run a command, says the request is resolved
//!       (`serverRequest/resolved`), then ends the turn;
//!     - `policy`: the message is `turn <approval policy> <reviewer>
//!       <sandbox policy's type> thread <approval policy> <reviewer>
//!       <sandbox>`: what the turn and the thread were given;
//!     - `wait`: runs until `turn/interrupt` (then `interrupted`); a
//!       `turn/steer` meanwhile is taken and said back as a message;
//!     - `fail`: ends `failed` with the error `fake failure`;
//!     - `garbled`: ends with a `turn/completed` that does not read (its
//!       turn has no status);
//!     - `other`: first completes another turn of the same thread (which
//!       is not this session's), then as `plain`;
//!     - `stream`: streams up to [`STREAM`] deltas without a pause, from a
//!       thread of its own, until `turn/interrupt` stops it (then
//!       `interrupted`; otherwise `completed`);
//!     - `badstart`: answers `turn/start` with a turn that does not read
//!       (no status) and says nothing more of it;
//!     - `exit`: exits mid-turn.
//!   - Any other request is "method not found" (`-32601`). An error
//!     answering a request of the fake's ends the turn with the message
//!     `refused <message>`.
//! - Anything else exits with code 2.

use std::io::{BufRead as _, Write as _};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

#[path = "fake_codex_app_server/catalogue.rs"]
mod catalogue;
#[path = "fake_codex_app_server/launch_audit.rs"]
mod launch_audit;

/// The version printed when `FAKE_CODEX_VERSION` is unset: the one the
/// driver's schema was generated from.
const VERSION: &str = "0.166.0";

/// What a turn asked, waiting for the client's answer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asked {
    Command,
    FileChange,
    Permissions,
    UserInput,
    Elicitation,
}

/// The most deltas the `stream` scenario writes: far more than a cancel
/// needs to arrive in, so a turn that ends `completed` was not cancelled
/// in time.
const STREAM: u32 = 200_000;

/// The agent's output, shared with a streaming turn's thread.
type Out = Arc<Mutex<std::io::Stdout>>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let version = std::env::var("FAKE_CODEX_VERSION").unwrap_or_else(|_unset| VERSION.to_owned());
    match args.first().map(String::as_str) {
        Some("--version") => {
            let mut stdout = std::io::stdout();
            match writeln!(stdout, "codex-cli {version}").and_then(|()| stdout.flush()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_closed) => ExitCode::FAILURE,
            }
        }
        Some("app-server") => Server::new(version).serve(),
        _ => ExitCode::from(2),
    }
}

/// A turn that waits on the client.
enum Waiting {
    /// For the answer to a request of the fake's.
    Answer {
        id: Value,
        turn: String,
        asked: Asked,
    },
    /// For an interrupt.
    Interrupt { turn: String },
    /// For an interrupt, while a thread streams; set to stop it.
    Streaming { stop: Arc<AtomicBool> },
}

struct Server {
    version: String,
    thread: Option<String>,
    turns: u32,
    waiting: Option<Waiting>,
    /// The thread's approval policy, reviewer and sandbox, as given.
    started: String,
    /// The last `turn/start`'s parameters.
    last_turn: Value,
    out: Out,
}

impl Server {
    fn new(version: String) -> Self {
        Self {
            version,
            thread: None,
            turns: 0,
            waiting: None,
            started: String::new(),
            last_turn: Value::Null,
            out: Arc::new(Mutex::new(std::io::stdout())),
        }
    }

    fn serve(mut self) -> ExitCode {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                return ExitCode::FAILURE;
            };
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let id = message.get("id").cloned();
            let method = message.get("method").and_then(Value::as_str);
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            let done = match (id, method) {
                (Some(id), Some(method)) => self.request(&id, method, &params),
                (Some(id), None) => self.answered(&id, &message),
                (None, _) => Ok(()),
            };
            match done {
                Ok(()) => {}
                Err(Exit::Now) => return ExitCode::SUCCESS,
                Err(Exit::Closed) => return ExitCode::FAILURE,
            }
        }
        ExitCode::SUCCESS
    }

    fn request(&mut self, id: &Value, method: &str, params: &Value) -> Result<(), Exit> {
        catalogue::record(method).map_err(|_failed| Exit::Closed)?;
        match method {
            "model/list" | "config/read" | "getUserSavedConfig" => {
                self.send(&json!({"id": id, "result": catalogue::response(method, params)}))
            }
            "initialize" => {
                let agent = format!("codex_cli_rs/{} (fake; x86_64) vessel", self.version);
                self.send(&json!({"id": id, "result": {
                    "userAgent": agent, "codexHome": "/fake/codex", "platformFamily": "unix",
                    "platformOs": "linux"
                }}))
            }
            "thread/start" => {
                let thread = format!("fake-thread-{}", std::process::id());
                self.opened(id, &thread, params)
            }
            "skills/list" => self.send(&json!({"id": id, "result": {"data": [{
                "cwd": params.get("cwds").and_then(|cwds| cwds.get(0)).cloned().unwrap_or(Value::Null),
                "errors": [],
                "skills": [
                    {"name":"review","description":"Review changes","path":"/fake/skills/review/SKILL.md","enabled":true,"scope":"user"},
                    {"name":"hidden","description":"Disabled","path":"/fake/hidden","enabled":false,"scope":"user"}
                ]
            }]}})),
            "thread/resume" => {
                let thread = text(params, "threadId");
                if thread.starts_with("missing") {
                    return self.refuse(id, -32600, &format!("no thread {thread}"));
                }
                self.opened(id, &thread, params)
            }
            "turn/start" => self.turn(id, params),
            "turn/interrupt" => {
                self.send(&json!({"id": id, "result": {}}))?;
                match self.waiting.take() {
                    Some(Waiting::Interrupt { turn }) => {
                        self.completed(&turn, "interrupted", None)?;
                    }
                    // Its thread ends the turn.
                    Some(Waiting::Streaming { stop }) => stop.store(true, Ordering::SeqCst),
                    other @ (Some(Waiting::Answer { .. }) | None) => self.waiting = other,
                }
                Ok(())
            }
            "turn/steer" => {
                let expected = text(params, "expectedTurnId");
                match &self.waiting {
                    Some(Waiting::Interrupt { turn }) if *turn == expected => {
                        let turn = turn.clone();
                        self.send(&json!({"id": id, "result": {"turnId": turn}}))?;
                        let said = format!("steered: {}", first_text(params));
                        self.message(&turn, &format!("steer-{}", self.turns), &said, true)
                    }
                    Some(_) | None => self.refuse(id, -32600, "no turn to steer"),
                }
            }
            _ => self.refuse(id, -32601, &format!("method not found: {method}")),
        }
    }

    /// The client's answer to a request of the fake's.
    fn answered(&mut self, id: &Value, message: &Value) -> Result<(), Exit> {
        let Some(Waiting::Answer {
            id: asked_id,
            turn,
            asked,
        }) = self.waiting.take()
        else {
            return Ok(());
        };
        if asked_id != *id {
            self.waiting = Some(Waiting::Answer {
                id: asked_id,
                turn,
                asked,
            });
            return Ok(());
        }
        if let Some(error) = message.get("error") {
            let said = format!("refused {}", error["message"].as_str().unwrap_or(""));
            self.message(&turn, "msg-answer", &said, true)?;
            return self.completed(&turn, "completed", None);
        }
        let result = message.get("result").cloned().unwrap_or(Value::Null);
        let mut end = "completed";
        let said = match asked {
            Asked::Command | Asked::FileChange => {
                let decision = result.get("decision").cloned().unwrap_or(Value::Null);
                let refused = decision == "decline" || decision == "cancel";
                if decision == "cancel" {
                    end = "interrupted";
                }
                let status = if refused { "declined" } else { "completed" };
                let item = if asked == Asked::Command {
                    self.notify(
                        "item/started",
                        &json!({"threadId": self.thread(), "turnId": turn, "startedAtMs": 1,
                            "item": command("cmd-1", "inProgress", None)}),
                    )?;
                    command("cmd-1", status, None)
                } else {
                    file_change(status)
                };
                self.notify(
                    "item/completed",
                    &json!({"threadId": self.thread(), "turnId": turn, "completedAtMs": 2,
                        "item": item}),
                )?;
                format!("decision {decision}")
            }
            Asked::Permissions => {
                let scope = field(&result, "scope").as_str().unwrap_or("turn");
                format!("granted {scope} {}", field(&result, "permissions"))
            }
            Asked::UserInput => format!("answers {}", field(&result, "answers")),
            Asked::Elicitation => format!(
                "action {} {}",
                field(&result, "action").as_str().unwrap_or(""),
                result.get("content").cloned().unwrap_or(Value::Null)
            ),
        };
        self.message(&turn, "msg-answer", &said, true)?;
        self.completed(&turn, end, None)
    }

    /// Ask the client something on `turn`, and wait for its answer.
    fn ask(
        &mut self,
        id: &Value,
        turn: &str,
        asked: Asked,
        method: &str,
        params: &Value,
    ) -> Result<(), Exit> {
        self.waiting = Some(Waiting::Answer {
            id: id.clone(),
            turn: turn.to_owned(),
            asked,
        });
        self.send(&json!({"id": id, "method": method, "params": params}))
    }

    fn opened(&mut self, id: &Value, thread: &str, params: &Value) -> Result<(), Exit> {
        launch_audit::record(params).map_err(|_failed| Exit::Closed)?;
        self.thread = Some(thread.to_owned());
        self.started = format!(
            "{} {} {}",
            text(params, "approvalPolicy"),
            text(params, "approvalsReviewer"),
            text(params, "sandbox")
        );
        let cwd = params.get("cwd").cloned().unwrap_or(Value::Null);
        self.send(&json!({"id": id, "result": {
            "approvalPolicy": "on-request", "approvalsReviewer": "user", "cwd": cwd,
            "model": "fake-model", "modelProvider": "openai",
            "sandbox": {"type": "workspaceWrite"},
            "thread": {
                "id": thread, "cliVersion": self.version, "createdAt": 0, "cwd": cwd,
                "ephemeral": false, "modelProvider": "openai", "preview": "",
                "projectId": null, "sessionId": thread, "source": "vscode",
                "status": {"type": "idle"}, "turns": [], "updatedAt": 0
            }
        }}))
    }

    fn turn(&mut self, id: &Value, params: &Value) -> Result<(), Exit> {
        launch_audit::turn(params).map_err(|_failed| Exit::Closed)?;
        self.turns = self.turns.saturating_add(1);
        let turn = format!("turn-{}", self.turns);
        let prompt = first_text(params);
        self.last_turn = params.clone();
        if prompt.starts_with("badstart") {
            // A turn without its status: the answer does not read, and
            // nothing more is said of the turn.
            return self.send(&json!({"id": id, "result": {"turn": {"id": turn}}}));
        }
        self.send(&json!({"id": id, "result": {"turn": in_progress(&turn)}}))?;
        self.notify(
            "turn/started",
            &json!({"threadId": self.thread(), "turn": in_progress(&turn)}),
        )?;
        let scenario = prompt.split_whitespace().next().unwrap_or("").to_owned();
        match scenario.as_str() {
            "$review" => {
                let skill = params["input"]
                    .as_array()
                    .and_then(|input| input.iter().find(|item| item["type"] == "skill"));
                let description = skill.map_or_else(
                    || "missing native skill".to_owned(),
                    |skill| {
                        format!(
                            "native skill {} {} {prompt}",
                            text(skill, "name"),
                            text(skill, "path")
                        )
                    },
                );
                self.message(&turn, "msg-skill", &description, false)?;
                self.completed(&turn, "completed", None)
            }
            "think" => {
                self.notify(
                    "item/reasoning/summaryTextDelta",
                    &json!({"threadId": self.thread(), "turnId": turn, "itemId": "think-1",
                        "summaryIndex": 0, "delta": "thinking it over"}),
                )?;
                self.plain(&turn, &prompt)
            }
            "quiet" => {
                self.message(&turn, "msg-quiet", "said all at once", false)?;
                self.completed(&turn, "completed", None)
            }
            "tool" => {
                let started = command("cmd-1", "inProgress", None);
                self.notify(
                    "item/started",
                    &json!({"threadId": self.thread(), "turnId": turn, "startedAtMs": 1,
                        "item": started}),
                )?;
                let done = command("cmd-1", "completed", Some("hi\n"));
                self.notify(
                    "item/completed",
                    &json!({"threadId": self.thread(), "turnId": turn, "completedAtMs": 2,
                        "item": done}),
                )?;
                self.plain(&turn, &prompt)
            }
            "plan" => {
                self.notify(
                    "turn/plan/updated",
                    &json!({"threadId": self.thread(), "turnId": turn, "plan": [
                        {"step": "read", "status": "completed"},
                        {"step": "write", "status": "inProgress"}
                    ]}),
                )?;
                self.plain(&turn, &prompt)
            }
            "approve" | "resolved" | "edit" | "permit" | "ask" | "secret" | "form" | "policy" => {
                self.asking(&scenario, &turn)
            }
            "wait" => {
                self.waiting = Some(Waiting::Interrupt { turn });
                Ok(())
            }
            "fail" => self.completed(&turn, "failed", Some("fake failure")),
            "garbled" => self.notify(
                "turn/completed",
                &json!({"threadId": self.thread(), "turn": {"id": turn, "items": []}}),
            ),
            "other" => {
                self.completed("turn-of-another-client", "completed", None)?;
                self.plain(&turn, &prompt)
            }
            "stream" => {
                let stop = Arc::new(AtomicBool::new(false));
                let streaming = Streaming {
                    out: Arc::clone(&self.out),
                    thread: self.thread(),
                    turn: turn.clone(),
                    stop: Arc::clone(&stop),
                };
                self.waiting = Some(Waiting::Streaming { stop });
                drop(std::thread::spawn(move || streaming.run()));
                Ok(())
            }
            "exit" => Err(Exit::Now),
            _ => self.plain(&turn, &prompt),
        }
    }

    /// The scenarios that ask the client something (or say what the turn
    /// and the thread were given).
    fn asking(&mut self, scenario: &str, turn: &str) -> Result<(), Exit> {
        let thread = self.thread();
        match scenario {
            "approve" | "resolved" => {
                let rule = json!({"acceptWithExecpolicyAmendment":
                    {"execpolicy_amendment": ["echo"]}});
                let params = json!({"threadId": thread, "turnId": turn,
                    "itemId": "cmd-1", "command": "echo hi", "cwd": "/fake",
                    "startedAtMs": 1, "availableDecisions": ["accept", "acceptForSession",
                        rule, "decline", "cancel"]});
                let id = json!(format!("approval-{}", self.turns));
                let method = "item/commandExecution/requestApproval";
                self.ask(&id, turn, Asked::Command, method, &params)?;
                if scenario == "resolved" {
                    self.waiting = None;
                    self.notify(
                        "serverRequest/resolved",
                        &json!({"threadId": thread, "requestId": id}),
                    )?;
                    return self.completed(turn, "completed", None);
                }
                Ok(())
            }
            "edit" => {
                self.notify(
                    "item/started",
                    &json!({"threadId": thread, "turnId": turn, "startedAtMs": 1,
                        "item": file_change("inProgress")}),
                )?;
                let params = json!({"threadId": thread, "turnId": turn,
                    "itemId": "edit-1", "reason": "Write notes.md", "startedAtMs": 1});
                let method = "item/fileChange/requestApproval";
                self.ask(&json!("edit-ask"), turn, Asked::FileChange, method, &params)
            }
            "permit" => {
                let params = json!({"threadId": thread, "turnId": turn,
                    "itemId": "perm-1", "cwd": "/fake", "startedAtMs": 1,
                    "reason": "Fetch the docs", "permissions": {"network": {"enabled": true}}});
                let method = "item/permissions/requestApproval";
                self.ask(&json!(7), turn, Asked::Permissions, method, &params)
            }
            "ask" | "secret" => {
                let questions = if scenario == "secret" {
                    json!([{"id": "token", "header": "Token", "question": "Your token?",
                        "isOther": false, "isSecret": true, "options": null}])
                } else {
                    json!([
                        {"id": "color", "header": "Color", "question": "Which color?",
                            "isOther": false, "isSecret": false, "options": [
                                {"label": "red", "description": "Warm"},
                                {"label": "blue", "description": "Cool"}]},
                        {"id": "name", "header": "Name", "question": "Your name?",
                            "isOther": false, "isSecret": false, "options": null}
                    ])
                };
                let params = json!({"threadId": thread, "turnId": turn,
                    "itemId": "ask-1", "isBlocking": true, "questions": questions});
                let method = "item/tool/requestUserInput";
                self.ask(&json!("ask-1"), turn, Asked::UserInput, method, &params)
            }
            "form" => {
                let params = json!({"threadId": thread, "turnId": turn,
                    "serverName": "fake-server", "mode": "form", "message": "Who?",
                    "requestedSchema": {"type": "object", "properties": {
                        "name": {"type": "string", "title": "Name"}}, "required": ["name"]}});
                let method = "mcpServer/elicitation/request";
                self.ask(&json!("form-1"), turn, Asked::Elicitation, method, &params)
            }
            "policy" => {
                let given = &self.last_turn;
                let said = format!(
                    "turn {} {} {} thread {}",
                    text(given, "approvalPolicy"),
                    text(given, "approvalsReviewer"),
                    field(field(given, "sandboxPolicy"), "type")
                        .as_str()
                        .unwrap_or(""),
                    self.started
                );
                self.message(turn, "msg-policy", &said, true)?;
                self.completed(turn, "completed", None)
            }
            _ => self.plain(turn, scenario),
        }
    }

    /// A message in two deltas, another thread's delta, usage, the end.
    fn plain(&mut self, turn: &str, prompt: &str) -> Result<(), Exit> {
        self.notify(
            "item/agentMessage/delta",
            &json!({"threadId": "other-thread", "turnId": "other-turn", "itemId": "other",
                "delta": "not this session's"}),
        )?;
        let said = format!("fake reply: {prompt}");
        self.message(turn, &format!("msg-{}", self.turns), &said, true)?;
        self.notify(
            "thread/tokenUsage/updated",
            &json!({"threadId": self.thread(), "turnId": turn, "tokenUsage": {
                "total": usage(self.turns.saturating_mul(100)), "last": usage(100),
                "modelContextWindow": 1000
            }}),
        )?;
        self.completed(turn, "completed", None)
    }

    /// An agent message, streamed in two deltas or whole.
    fn message(&mut self, turn: &str, item: &str, text: &str, stream: bool) -> Result<(), Exit> {
        let message = json!({"type": "agentMessage", "id": item, "text": text});
        self.notify(
            "item/started",
            &json!({"threadId": self.thread(), "turnId": turn, "startedAtMs": 1,
                "item": {"type": "agentMessage", "id": item, "text": ""}}),
        )?;
        if stream {
            let middle = text.char_indices().nth(text.chars().count().div_euclid(2));
            let (head, tail) = text.split_at(middle.map_or(text.len(), |(at, _)| at));
            for delta in [head, tail] {
                self.notify(
                    "item/agentMessage/delta",
                    &json!({"threadId": self.thread(), "turnId": turn, "itemId": item,
                        "delta": delta}),
                )?;
            }
        }
        self.notify(
            "item/completed",
            &json!({"threadId": self.thread(), "turnId": turn, "completedAtMs": 2,
                "item": message}),
        )
    }

    fn completed(&mut self, turn: &str, status: &str, error: Option<&str>) -> Result<(), Exit> {
        let error = error.map(|message| json!({"message": message}));
        self.notify(
            "turn/completed",
            &json!({"threadId": self.thread(), "turn": {
                "id": turn, "items": [], "status": status, "error": error
            }}),
        )
    }

    fn thread(&self) -> String {
        self.thread.clone().unwrap_or_default()
    }

    fn notify(&mut self, method: &str, params: &Value) -> Result<(), Exit> {
        self.send(&json!({"method": method, "params": params}))
    }

    fn refuse(&mut self, id: &Value, code: i64, message: &str) -> Result<(), Exit> {
        self.send(&json!({"id": id, "error": {"code": code, "message": message}}))
    }

    fn send(&mut self, message: &Value) -> Result<(), Exit> {
        write_line(&self.out, message)
    }
}

/// One line on the shared output.
fn write_line(out: &Out, message: &Value) -> Result<(), Exit> {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    writeln!(out, "{message}")
        .and_then(|()| out.flush())
        .map_err(|_closed| Exit::Closed)
}

/// A turn streaming from a thread of its own (`stream`).
struct Streaming {
    out: Out,
    thread: String,
    turn: String,
    stop: Arc<AtomicBool>,
}

impl Streaming {
    fn run(self) {
        let mut sent = 0;
        while sent < STREAM && !self.stop.load(Ordering::SeqCst) {
            let delta = json!({"method": "item/agentMessage/delta", "params": {
                "threadId": self.thread, "turnId": self.turn, "itemId": "stream-1",
                "delta": "x"}});
            if write_line(&self.out, &delta).is_err() {
                return;
            }
            sent = sent.saturating_add(1);
        }
        let status = if self.stop.load(Ordering::SeqCst) {
            "interrupted"
        } else {
            "completed"
        };
        let _closed = write_line(
            &self.out,
            &json!({"method": "turn/completed", "params": {"threadId": self.thread,
                "turn": {"id": self.turn, "items": [], "status": status, "error": null}}}),
        );
    }
}

/// Why serving stopped early.
enum Exit {
    /// The scenario says so.
    Now,
    /// Stdout closed.
    Closed,
}

fn in_progress(turn: &str) -> Value {
    json!({"id": turn, "items": [], "status": "inProgress"})
}

fn command(id: &str, status: &str, output: Option<&str>) -> Value {
    json!({"type": "commandExecution", "id": id, "command": "echo hi", "cwd": "/fake",
        "commandActions": [], "status": status, "aggregatedOutput": output,
        "exitCode": output.map(|_| 0)})
}

fn file_change(status: &str) -> Value {
    json!({"type": "fileChange", "id": "edit-1", "status": status, "changes": [
        {"path": "/fake/notes.md", "kind": {"type": "add"}, "diff": "+notes\n"}]})
}

fn usage(total: u32) -> Value {
    json!({"inputTokens": total, "cachedInputTokens": 0, "outputTokens": 0,
        "reasoningOutputTokens": 0, "totalTokens": total})
}

/// `value`'s `key`, or null.
fn field<'v>(value: &'v Value, key: &str) -> &'v Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn text(params: &Value, key: &str) -> String {
    params[key].as_str().unwrap_or("").to_owned()
}

/// The first text input of a turn or a steer.
fn first_text(params: &Value) -> String {
    params["input"]
        .as_array()
        .and_then(|input| input.iter().find_map(|item| item["text"].as_str()))
        .unwrap_or("")
        .to_owned()
}
