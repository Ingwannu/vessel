//! Claude Code's stdio as frames: one task reads the agent's stdout a line
//! at a time into a bounded channel, one writes what the loop queues.
//!
//! [Decision Log]
//! - Purpose: the session loop sees the agent's frames in the order they
//!   were written (a turn's last delta before its `result`), never waits
//!   on a write while the agent waits on a read, and never holds a line
//!   without bound.
//! - Bounds: a line longer than [`MAX_LINE`] ends the connection (a
//!   protocol error) instead of growing a buffer without end. The CLI
//!   writes one message or one streamed event a line; a tool's whole
//!   output fits well within it.
//! - Reading what does not decode: a line that is not a JSON object is
//!   not this protocol and ends the connection. A frame of a type the
//!   driver knows whose fields do not read is kept apart when the session
//!   must act on it (a `control_request`, which is answered with a refusal
//!   whatever it asked; a `result`, without which a turn would never end)
//!   and logged and skipped otherwise (a message the timeline would only
//!   show). This is the floor of ADR 0057: what is not understood is
//!   denied, and nothing waits on it forever.

use serde::Deserialize as _;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;

use super::wire::{Frame, Outgoing};

/// The longest line read: 64 MiB.
pub(super) const MAX_LINE: usize = 64 * 1024 * 1024;

/// Lines queued for the agent before the loop waits.
const OUTGOING: usize = 64;

/// What one line of the agent's was.
#[derive(Debug)]
pub(super) enum Read {
    /// A frame.
    Frame(Box<Frame>),
    /// A control request whose fields do not read: refused, with its id
    /// when it has one.
    UnreadableRequest {
        request_id: Option<String>,
        error: String,
    },
    /// A turn's `result` whose fields do not read.
    UnreadableResult(String),
}

/// Why reading ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Ended {
    /// The agent closed its output.
    Closed,
    /// It wrote what is not this protocol.
    Protocol(String),
}

/// Read the agent's lines into `into` until it closes its output, writes
/// what is not a frame, or the loop is gone; the last item says why.
pub(super) async fn read(stdout: ChildStdout, into: mpsc::Sender<Result<Read, Ended>>) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    // One byte past the bound, so a line of exactly the bound is told from
    // a longer one.
    let limit = u64::try_from(MAX_LINE).map_or(u64::MAX, |max| max.saturating_add(1));
    let ended = loop {
        line.clear();
        let mut limited = (&mut reader).take(limit);
        match limited.read_until(b'\n', &mut line).await {
            Ok(0) => break Ended::Closed,
            Ok(_) if line.len() > MAX_LINE => {
                break Ended::Protocol(format!("a line longer than {MAX_LINE} bytes"));
            }
            Ok(_) => {}
            Err(error) => break Ended::Protocol(format!("reading the agent's output: {error}")),
        }
        let text = line.strip_suffix(b"\n").unwrap_or(&line);
        if text.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match parse(text) {
            Ok(Some(read)) => {
                if into.send(Ok(read)).await.is_err() {
                    return;
                }
            }
            Ok(None) => {}
            Err(error) => break Ended::Protocol(error),
        }
    };
    let _gone = into.send(Err(ended)).await;
}

/// One line: a frame, one the session must refuse or end a turn on, or
/// `None` for a frame skipped.
pub(super) fn parse(text: &[u8]) -> Result<Option<Read>, String> {
    let value: Value =
        serde_json::from_slice(text).map_err(|error| format!("not JSON: {error}"))?;
    if !value.is_object() {
        return Err("a line that is not a JSON object".to_owned());
    }
    match Frame::deserialize(&value) {
        Ok(frame) => Ok(Some(Read::Frame(Box::new(frame)))),
        Err(error) => match value.get("type").and_then(Value::as_str) {
            Some("control_request") => Ok(Some(Read::UnreadableRequest {
                request_id: value
                    .get("request_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                error: error.to_string(),
            })),
            Some("result") => Ok(Some(Read::UnreadableResult(error.to_string()))),
            kind => {
                tracing::warn!(?kind, %error, "a Claude Code frame that does not read, skipped");
                Ok(None)
            }
        },
    }
}

/// The loop's side of the agent's input: frames are written in order by a
/// task of their own.
#[derive(Debug, Clone)]
pub(super) struct Writer(mpsc::Sender<Vec<u8>>);

impl Writer {
    /// Start the writer for `stdin`.
    pub(super) fn start(stdin: ChildStdin) -> Self {
        let (sender, receiver) = mpsc::channel(OUTGOING);
        drop(tokio::spawn(write(stdin, receiver)));
        Self(sender)
    }

    /// Queue `frame`; false once the agent's input is closed (or the frame
    /// could not be encoded, which its types rule out: nothing is sent
    /// then, and the caller treats it as closed rather than wait on an
    /// answer that cannot come).
    pub(super) async fn send(&self, frame: &Outgoing) -> bool {
        let mut bytes = match serde_json::to_vec(frame) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(%error, "a frame for Claude Code could not be encoded");
                return false;
            }
        };
        bytes.push(b'\n');
        self.0.send(bytes).await.is_ok()
    }
}

async fn write(mut stdin: ChildStdin, mut lines: mpsc::Receiver<Vec<u8>>) {
    while let Some(line) = lines.recv().await {
        if stdin.write_all(&line).await.is_err() || stdin.flush().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Read, parse};

    #[test]
    fn a_line_is_a_frame_a_refusal_to_make_or_the_end() {
        assert!(matches!(
            parse(br#"{"type":"keep_alive"}"#),
            Ok(Some(Read::Frame(_)))
        ));
        assert!(matches!(
            parse(br#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool"}}"#),
            Ok(Some(Read::UnreadableRequest { request_id: Some(ref id), .. })) if id == "r1"
        ));
        assert!(matches!(
            parse(br#"{"type":"control_request","request":{"subtype":"can_use_tool"}}"#),
            Ok(Some(Read::UnreadableRequest {
                request_id: None,
                ..
            }))
        ));
        assert!(matches!(
            parse(br#"{"type":"result","subtype":7}"#),
            Ok(Some(Read::UnreadableResult(_)))
        ));
        assert!(matches!(
            parse(br#"{"type":"assistant","message":"not a message"}"#),
            Ok(None)
        ));
        assert!(parse(b"not json").is_err());
        assert!(parse(b"[1]").is_err());
    }
}
