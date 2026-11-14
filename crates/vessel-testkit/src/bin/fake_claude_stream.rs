//! `fake-claude-stream`: stands in for `claude` where Vessel drives it over
//! its stream-json protocol (ADR 0057). A skeleton (slice 0): the Claude
//! Code driver's slices teach it each frame they use, with frames shaped
//! as the Agent SDK's declared types say.
//!
//! Behaviour:
//!
//! - `--version` prints `<FAKE_CLAUDE_VERSION or 2.1.246> (Claude Code)`,
//!   as the real one does.
//! - `-p` with `--input-format stream-json` reads one JSON frame a line.
//!   Before its first answer it writes a `system` `init` frame naming its
//!   session (`fake-claude-<process id>`); each `user` frame is answered
//!   with a `result` frame of subtype `success` and an empty result. Other
//!   frames are read and ignored.
//! - Anything else exits with code 2.

use std::io::{BufRead as _, Write as _};
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
        return said(&format!("{version} (Claude Code)"));
    }
    let streams = args.iter().any(|arg| arg == "-p")
        && args
            .windows(2)
            .any(|pair| matches!(pair, [flag, value] if flag == "--input-format" && value == "stream-json"));
    if !streams {
        return ExitCode::from(2);
    }
    serve()
}

/// `line` on stdout.
fn said(line: &str) -> ExitCode {
    let mut stdout = std::io::stdout();
    match writeln!(stdout, "{line}").and_then(|()| stdout.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_closed) => ExitCode::FAILURE,
    }
}

/// Answer `user` frames until stdin closes.
fn serve() -> ExitCode {
    let session = format!("fake-claude-{}", std::process::id());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut introduced = false;
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return ExitCode::FAILURE;
        };
        let Ok(frame) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if frame.get("type").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let mut frames = Vec::new();
        if !introduced {
            introduced = true;
            frames.push(json!({ "type": "system", "subtype": "init", "session_id": session }));
        }
        frames.push(json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "result": "",
            "session_id": session,
        }));
        for frame in frames {
            if writeln!(stdout, "{frame}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
