//! `fake-claude-stream`: stands in for `claude` where Vessel drives it over
//! its stream-json protocol (ADR 0057). Frames are shaped as the Agent
//! SDK's declared types say (written here from the field names, not
//! copied); each Claude Code driver slice teaches it what it uses.
//!
//! Behaviour:
//!
//! - `--version` prints `<FAKE_CLAUDE_VERSION or 2.1.246> (Claude Code)`,
//!   as the real one does.
//! - `-p` with `--input-format stream-json` reads one JSON frame a line.
//!   The conversation is the one `--session-id=<id>` or `--resume=<id>`
//!   names (`fake-claude-<process id>` without either).
//! - The control protocol: `initialize` is answered with success, or with
//!   an error when `FAKE_CLAUDE_INITIALIZE=refuse`, or never when it is
//!   `silent`; `interrupt` is answered and ends a turn waiting for it.
//! - A `user` frame starts a turn. Before its first, a `system` `init`
//!   frame. The prompt's first word picks what the turn does:
//!   - `think`: reasoning, then a reply, both streamed;
//!   - `tool`: asks to run a command (`can_use_tool`) and waits for the
//!     answer; denied, the command's result is the denial, then a reply;
//!   - `wait`: streams a little, then waits for an interrupt;
//!   - `fail`: ends with an error subtype;
//!   - `limit`: ends `success` with `is_error`, as a usage limit does;
//!   - `exit`: streams a little and exits with code 3;
//!   - `ask`: asks the host something it has no name for (a control
//!     request subtype the driver does not serve), waits for the answer,
//!     then for an interrupt;
//!   - `garbled`: asks to use a tool in a request that does not read
//!     (no tool use id), waits for the answer, then for an interrupt;
//!   - `garbage`: writes a line that is not JSON;
//!   - anything else: a streamed reply, `Hello there`.
//! - With `FAKE_CLAUDE_NO_TRANSCRIPTS` set, `--resume=<id>` finds no
//!   conversation: it says so on stderr and exits with code 1 before
//!   reading anything.
//! - Anything else exits with code 2.

use std::io::BufRead as _;
use std::process::ExitCode;

use serde_json::{Value, json};

/// The version printed when `FAKE_CLAUDE_VERSION` is unset: the one the
/// Agent SDK the driver follows speaks to.
const VERSION: &str = "2.1.246";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        let version =
            std::env::var("FAKE_CLAUDE_VERSION").unwrap_or_else(|_unset| VERSION.to_owned());
        return match said(&mut std::io::stdout(), &format!("{version} (Claude Code)")) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_closed) => ExitCode::FAILURE,
        };
    }
    let streams = args.iter().any(|arg| arg == "-p")
        && args
            .windows(2)
            .any(|pair| matches!(pair, [flag, value] if flag == "--input-format" && value == "stream-json"));
    if !streams {
        return ExitCode::from(2);
    }
    let session = args
        .iter()
        .find_map(|arg| {
            arg.strip_prefix("--session-id=")
                .or_else(|| arg.strip_prefix("--resume="))
        })
        .map_or_else(
            || format!("fake-claude-{}", std::process::id()),
            str::to_owned,
        );
    // `FAKE_CLAUDE_NO_TRANSCRIPTS`: no conversation was ever written, as
    // for a session parked before its first turn; a resume then fails at
    // once, with the CLI's reason on stderr (its words unverified).
    if std::env::var_os("FAKE_CLAUDE_NO_TRANSCRIPTS").is_some()
        && let Some(id) = args.iter().find_map(|arg| arg.strip_prefix("--resume="))
    {
        let _said = said(
            &mut std::io::stderr(),
            &format!("No conversation found with session ID: {id}"),
        );
        return ExitCode::from(1);
    }
    match Fake::new(session).serve() {
        Ok(Some(code)) => ExitCode::from(code),
        Ok(None) => ExitCode::SUCCESS,
        Err(_closed) => ExitCode::FAILURE,
    }
}

/// `line` on `out`.
fn said(out: &mut impl std::io::Write, line: &str) -> std::io::Result<()> {
    writeln!(out, "{line}")?;
    out.flush()
}

/// What a turn waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Waiting {
    Nothing,
    /// The answer to the tool request `request`, for call `call`.
    Permission {
        request: String,
        call: String,
    },
    /// The answer to a request the host should refuse, then an interrupt.
    Refusal {
        request: String,
    },
    /// An interrupt.
    Interrupt,
}

struct Fake {
    session: String,
    out: std::io::Stdout,
    introduced: bool,
    turns: u32,
    waiting: Waiting,
}

impl Fake {
    fn new(session: String) -> Self {
        Self {
            session,
            out: std::io::stdout(),
            introduced: false,
            turns: 0,
            waiting: Waiting::Nothing,
        }
    }

    fn send(&mut self, frame: &Value) -> std::io::Result<()> {
        said(&mut self.out, &frame.to_string())
    }

    /// Answer frames until stdin closes; `Some(code)` to exit with it.
    fn serve(mut self) -> std::io::Result<Option<u8>> {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            let Ok(frame) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let exit = match frame.get("type").and_then(Value::as_str) {
                Some("control_request") => self.control(&frame)?,
                Some("control_response") => self.answered(&frame)?,
                Some("user") => self.turn(&frame)?,
                _ => None,
            };
            if exit.is_some() {
                return Ok(exit);
            }
        }
        Ok(None)
    }

    /// A request of the host's.
    fn control(&mut self, frame: &Value) -> std::io::Result<Option<u8>> {
        let id = frame["request_id"].as_str().unwrap_or_default().to_owned();
        match frame.pointer("/request/subtype").and_then(Value::as_str) {
            Some("initialize") => match std::env::var("FAKE_CLAUDE_INITIALIZE").as_deref() {
                Ok("silent") => {}
                Ok("refuse") => self.send(&json!({"type": "control_response",
                    "response": {"subtype": "error", "request_id": id,
                                 "error": "initialize is not supported"}}))?,
                _ => self.send(&json!({"type": "control_response",
                    "response": {"subtype": "success", "request_id": id,
                                 "response": {"commands": [], "agents": [], "models": [],
                                              "output_style": "default",
                                              "available_output_styles": ["default"],
                                              "account": {}}}}))?,
            },
            Some("interrupt") => {
                self.send(&json!({"type": "control_response",
                    "response": {"subtype": "success", "request_id": id,
                                 "response": {"still_queued": []}}}))?;
                if self.waiting == Waiting::Interrupt {
                    self.waiting = Waiting::Nothing;
                    self.send(
                        &json!({"type": "result", "subtype": "error_during_execution",
                        "is_error": true, "errors": ["the turn was interrupted"],
                        "session_id": self.session}),
                    )?;
                }
            }
            _ => self.send(&json!({"type": "control_response",
                "response": {"subtype": "error", "request_id": id,
                             "error": "unknown request"}}))?,
        }
        Ok(None)
    }

    /// The host's answer to a request of the fake's.
    fn answered(&mut self, frame: &Value) -> std::io::Result<Option<u8>> {
        let response = &frame["response"];
        let id = response["request_id"].as_str().unwrap_or_default();
        match self.waiting.clone() {
            Waiting::Permission { request, call } if request == id => {
                self.waiting = Waiting::Nothing;
                let answer = &response["response"];
                let denied = response["subtype"] != "success" || answer["behavior"] != "allow";
                let text = if denied {
                    answer["message"].as_str().unwrap_or("denied").to_owned()
                } else {
                    "a\nb".to_owned()
                };
                self.send(
                    &json!({"type": "user", "parent_tool_use_id": null, "isSynthetic": true,
                    "session_id": self.session,
                    "message": {"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": call, "content": text,
                         "is_error": denied}]}}),
                )?;
                let message = format!("m{}b", self.turns);
                self.reply(&message, "", "Done.")?;
                self.result(&json!({"input_tokens": 4, "output_tokens": 2}))?;
            }
            Waiting::Refusal { request } if request == id => {
                // Refused, as the host must; the turn waits to be stopped.
                self.waiting = Waiting::Interrupt;
            }
            Waiting::Nothing
            | Waiting::Interrupt
            | Waiting::Permission { .. }
            | Waiting::Refusal { .. } => {}
        }
        Ok(None)
    }

    /// A prompt: a turn.
    fn turn(&mut self, frame: &Value) -> std::io::Result<Option<u8>> {
        self.turns = self.turns.saturating_add(1);
        if !self.introduced {
            self.introduced = true;
            self.send(
                &json!({"type": "system", "subtype": "init", "session_id": self.session,
                "claude_code_version": VERSION, "model": "claude-opus-5-5",
                "permissionMode": "default", "slash_commands": [], "capabilities": [],
                "tools": ["Bash"], "mcp_servers": [], "cwd": "/"}),
            )?;
        }
        let text = frame
            .pointer("/message/content")
            .and_then(Value::as_str)
            .or_else(|| {
                frame
                    .pointer("/message/content/0/text")
                    .and_then(Value::as_str)
            })
            .unwrap_or_default();
        let message = format!("m{}", self.turns);
        let call = format!("toolu_{}", self.turns);
        match text.split_whitespace().next().unwrap_or_default() {
            "think" => {
                self.reply(&message, "Look first.", "Thought about it.")?;
                self.result(&json!({"input_tokens": 3, "output_tokens": 5,
                                   "cache_read_input_tokens": 100,
                                   "cache_creation_input_tokens": 2}))?;
            }
            "tool" => {
                self.send(&json!({"type": "assistant", "parent_tool_use_id": null,
                    "session_id": self.session,
                    "message": {"id": message, "content": [
                        {"type": "tool_use", "id": call, "name": "Bash",
                         "input": {"command": "ls", "description": "List"}}]}}))?;
                let request = format!("c{}", self.turns);
                self.send(&json!({"type": "control_request", "request_id": request,
                    "request": {"subtype": "can_use_tool", "tool_name": "Bash",
                                "input": {"command": "ls"}, "tool_use_id": call,
                                "permission_suggestions": []}}))?;
                self.waiting = Waiting::Permission { request, call };
            }
            "wait" => {
                self.delta(&message, "Working")?;
                self.waiting = Waiting::Interrupt;
            }
            "fail" => self.send(
                &json!({"type": "result", "subtype": "error_during_execution",
                "is_error": true, "errors": ["boom"], "session_id": self.session}),
            )?,
            "limit" => self.send(&json!({"type": "result", "subtype": "success",
                "is_error": true, "result": "You've hit your usage limit",
                "session_id": self.session}))?,
            "exit" => {
                self.delta(&message, "Bye")?;
                return Ok(Some(3));
            }
            "ask" => {
                let request = format!("q{}", self.turns);
                self.send(&json!({"type": "control_request", "request_id": request,
                    "request": {"subtype": "elicitation", "mcp_server_name": "docs",
                                "message": "Which file?"}}))?;
                self.waiting = Waiting::Refusal { request };
            }
            "garbled" => {
                let request = format!("g{}", self.turns);
                self.send(&json!({"type": "control_request", "request_id": request,
                    "request": {"subtype": "can_use_tool", "tool_name": "Bash"}}))?;
                self.waiting = Waiting::Refusal { request };
            }
            "garbage" => said(&mut self.out, "this is not a frame")?,
            _ => {
                self.reply(&message, "", "Hello there")?;
                self.result(&json!({"input_tokens": 3, "output_tokens": 2}))?;
            }
        }
        Ok(None)
    }

    /// One streamed text delta of message `id`.
    fn delta(&mut self, id: &str, text: &str) -> std::io::Result<()> {
        self.send(&json!({"type": "stream_event", "parent_tool_use_id": null,
            "session_id": self.session,
            "event": {"type": "message_start", "message": {"id": id, "content": []}}}))?;
        self.send(&json!({"type": "stream_event", "parent_tool_use_id": null,
            "session_id": self.session,
            "event": {"type": "content_block_delta", "index": 0,
                      "delta": {"type": "text_delta", "text": text}}}))
    }

    /// Message `id` streamed (`thinking`, when not empty, then `text` in
    /// two pieces), then whole.
    fn reply(&mut self, id: &str, thinking: &str, text: &str) -> std::io::Result<()> {
        let stream = |event: Value| {
            json!({"type": "stream_event", "parent_tool_use_id": null,
                   "session_id": self.session, "event": event})
        };
        let mut frames = vec![stream(
            json!({"type": "message_start", "message": {"id": id, "content": []}}),
        )];
        let mut blocks = Vec::new();
        if !thinking.is_empty() {
            frames.push(stream(json!({"type": "content_block_delta", "index": 0,
                "delta": {"type": "thinking_delta", "thinking": thinking}})));
            blocks.push(json!({"type": "thinking", "thinking": thinking, "signature": "s"}));
        }
        let split = text.find(' ').map_or(text.len(), |at| at.saturating_add(1));
        let (first, rest) = text.split_at(split);
        for piece in [first, rest].into_iter().filter(|piece| !piece.is_empty()) {
            frames.push(stream(json!({"type": "content_block_delta", "index": 1,
                "delta": {"type": "text_delta", "text": piece}})));
        }
        frames.push(stream(json!({"type": "message_stop"})));
        blocks.push(json!({"type": "text", "text": text}));
        frames.push(json!({"type": "assistant", "parent_tool_use_id": null,
            "session_id": self.session, "message": {"id": id, "content": blocks}}));
        for frame in frames {
            self.send(&frame)?;
        }
        Ok(())
    }

    fn result(&mut self, usage: &Value) -> std::io::Result<()> {
        self.send(
            &json!({"type": "result", "subtype": "success", "is_error": false,
            "result": "", "stop_reason": "end_turn", "usage": usage,
            "session_id": self.session}),
        )
    }
}
