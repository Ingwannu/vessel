//! `fake-codex-app-server`: stands in for `codex` where Vessel drives it
//! over its own app-server protocol (ADR 0057). A skeleton (slice 0): the
//! Codex driver's slices teach it each method they use, with messages
//! shaped as `codex app-server generate-json-schema` says.
//!
//! Behaviour:
//!
//! - `--version` prints `codex-cli <FAKE_CODEX_VERSION or 0.166.0>`, as
//!   the real one does; Vessel reads it to decide whether its driver may
//!   run this version.
//! - `app-server` speaks JSON-RPC 2.0, one message a line, over stdio:
//!   `initialize` answers `{ "userAgent": ... }`; a notification (no `id`)
//!   is read and ignored; any other request is "method not found"
//!   (`-32601`).
//! - Anything else exits with code 2.

use std::io::{BufRead as _, Write as _};
use std::process::ExitCode;

use serde_json::{Value, json};

/// The version printed when `FAKE_CODEX_VERSION` is unset: the one the
/// driver's schema was generated from.
const VERSION: &str = "0.166.0";

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
        Some("app-server") => serve(&version),
        _ => ExitCode::from(2),
    }
}

/// Answer requests until stdin closes.
fn serve(version: &str) -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return ExitCode::FAILURE;
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let reply = match method {
            "initialize" => json!({
                "id": id,
                "result": { "userAgent": format!("fake-codex-app-server/{version}") },
            }),
            _ => json!({
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") },
            }),
        };
        if writeln!(stdout, "{reply}")
            .and_then(|()| stdout.flush())
            .is_err()
        {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
