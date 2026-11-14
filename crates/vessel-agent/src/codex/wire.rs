//! Codex's app-server wire: JSON-RPC 2.0 without the `jsonrpc` member, one
//! message a line, over the agent's stdio (`codex app-server`'s default
//! transport).
//!
//! [Decision Log]
//! - Purpose: the driver's loop must see the agent's messages in the order
//!   they were written (a turn's last delta before its `turn/completed`),
//!   and never wait on a write while the agent waits on a read.
//! - Choice: one task reads lines into a bounded channel, one task writes
//!   what the loop queues, and the loop matches responses to its requests
//!   by id itself, so it never awaits a response in place (as the ACP
//!   driver's loop never does).
//! - Bounds: a line longer than [`MAX_LINE`] ends the connection (a
//!   protocol error), never a buffer grown without end. A resumed thread's
//!   history is never asked for in one line (`excludeTurns`), so what
//!   remains is a single item or delta; Paseo measured a whole long
//!   thread at 19.75 MB.
//! - Alternatives: the ACP SDK's JSON-RPC layer (it requires the
//!   `jsonrpc` member Codex leaves out); a JSON-RPC crate (one more
//!   dependency for what is a line and four shapes).

use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;

/// The longest line read: 64 MiB.
pub(super) const MAX_LINE: usize = 64 * 1024 * 1024;

/// Lines queued for the agent before the loop waits.
const OUTGOING: usize = 64;

/// What the agent wrote.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Incoming {
    /// The answer to one of the driver's requests.
    Response { id: u64, result: Value },
    /// The failure of one of the driver's requests.
    Failure { id: u64, error: RpcError },
    /// A notification.
    Notification { method: String, params: Value },
    /// The agent asks the driver something (an approval, a question).
    Request {
        id: Value,
        method: String,
        params: Value,
    },
}

/// A JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(super) struct RpcError {
    pub(super) code: i64,
    pub(super) message: String,
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
/// what is not a message, or the loop is gone; the last item says why.
pub(super) async fn read(stdout: ChildStdout, into: mpsc::Sender<Result<Incoming, Ended>>) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    // What one line's read may take: the bound and its newline.
    let limit = u64::try_from(MAX_LINE).map_or(u64::MAX, |max| max.saturating_add(1));
    let ended = loop {
        line.clear();
        // One byte past the bound, so a line of exactly the bound is told
        // from a longer one.
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
            Ok(message) => {
                if into.send(Ok(message)).await.is_err() {
                    return;
                }
            }
            Err(error) => break Ended::Protocol(error),
        }
    };
    let _gone = into.send(Err(ended)).await;
}

/// One line as a message.
fn parse(text: &[u8]) -> Result<Incoming, String> {
    let value: Value =
        serde_json::from_slice(text).map_err(|error| format!("not JSON: {error}"))?;
    let Value::Object(mut fields) = value else {
        return Err("a message that is not an object".to_owned());
    };
    let id = fields.remove("id");
    let method = fields.remove("method");
    let params = fields.remove("params").unwrap_or(Value::Null);
    match (id, method) {
        (Some(id), Some(Value::String(method))) => Ok(Incoming::Request { id, method, params }),
        (None, Some(Value::String(method))) => Ok(Incoming::Notification { method, params }),
        (Some(id), None) => {
            // The driver numbers its requests; another id answers nothing
            // it asked.
            let id = id
                .as_u64()
                .ok_or_else(|| format!("an answer to a request it was not asked: {id}"))?;
            if let Some(error) = fields.remove("error") {
                let error = serde_json::from_value(error)
                    .map_err(|error| format!("an error without its code: {error}"))?;
                Ok(Incoming::Failure { id, error })
            } else {
                Ok(Incoming::Response {
                    id,
                    result: fields.remove("result").unwrap_or(Value::Null),
                })
            }
        }
        (_, _) => Err("a message without a method or an id".to_owned()),
    }
}

/// The loop's side of the agent's input: lines are written in order by a
/// task of their own.
#[derive(Debug, Clone)]
pub(super) struct Outgoing(mpsc::Sender<Vec<u8>>);

impl Outgoing {
    /// Start the writer for `stdin`.
    pub(super) fn start(stdin: ChildStdin) -> Self {
        let (sender, receiver) = mpsc::channel(OUTGOING);
        drop(tokio::spawn(write(stdin, receiver)));
        Self(sender)
    }

    /// A writer whose lines the caller reads, for the loop's tests.
    #[cfg(test)]
    pub(super) fn detached() -> (Self, mpsc::Receiver<Vec<u8>>) {
        let (sender, receiver) = mpsc::channel(OUTGOING);
        (Self(sender), receiver)
    }

    /// Send request `id`.
    pub(super) async fn request(&self, id: u64, method: &str, params: &impl Serialize) -> bool {
        self.line(&Request { id, method, params }).await
    }

    /// Send a notification.
    pub(super) async fn notify(&self, method: &str, params: &impl Serialize) -> bool {
        self.line(&Notification { method, params }).await
    }

    /// Answer the agent's request `id`.
    pub(super) async fn answer(&self, id: &Value, result: &impl Serialize) -> bool {
        self.line(&Answer { id, result }).await
    }

    /// Refuse the agent's request `id`.
    pub(super) async fn refuse(&self, id: &Value, code: i64, message: &str) -> bool {
        let error = Refusal { code, message };
        self.line(&Refused { id, error }).await
    }

    /// Queue one message; false once the writer is gone (the agent closed
    /// its input), or when it did not serialize.
    async fn line(&self, message: &impl Serialize) -> bool {
        let mut bytes = match serde_json::to_vec(message) {
            Ok(bytes) => bytes,
            Err(error) => {
                // Every message is built from serializable parts; one that
                // is not is not sent, and its caller answers as for a
                // closed input.
                tracing::warn!(%error, "a message for Codex did not serialize");
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

/// The bytes request `id` takes on the wire, its newline left out: what a
/// bounded prompt (a side question's context) is measured on before it is
/// sent, written by the same code that sends it.
///
/// # Errors
/// The message does not serialize.
pub(super) fn request_len(
    id: u64,
    method: &str,
    params: &impl Serialize,
) -> serde_json::Result<usize> {
    serde_json::to_vec(&Request { id, method, params }).map(|bytes| bytes.len())
}

#[derive(Serialize)]
struct Request<'a, P> {
    id: u64,
    method: &'a str,
    params: &'a P,
}

#[derive(Serialize)]
struct Notification<'a, P> {
    method: &'a str,
    params: &'a P,
}

#[derive(Serialize)]
struct Answer<'a, R> {
    id: &'a Value,
    result: &'a R,
}

#[derive(Serialize)]
struct Refused<'a> {
    id: &'a Value,
    error: Refusal<'a>,
}

#[derive(Serialize)]
struct Refusal<'a> {
    code: i64,
    message: &'a str,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_line_is_one_of_four_shapes() {
        assert_eq!(
            parse(br#"{"id":3,"result":{"turn":{"id":"t"}}}"#),
            Ok(Incoming::Response {
                id: 3,
                result: json!({"turn": {"id": "t"}})
            })
        );
        assert_eq!(
            parse(br#"{"id":4,"error":{"code":-32600,"message":"no"}}"#),
            Ok(Incoming::Failure {
                id: 4,
                error: RpcError {
                    code: -32600,
                    message: "no".to_owned()
                }
            })
        );
        assert_eq!(
            parse(br#"{"method":"turn/started","params":{"threadId":"a"}}"#),
            Ok(Incoming::Notification {
                method: "turn/started".to_owned(),
                params: json!({"threadId": "a"})
            })
        );
        assert_eq!(
            parse(br#"{"id":"x","method":"item/fileChange/requestApproval","params":{}}"#),
            Ok(Incoming::Request {
                id: json!("x"),
                method: "item/fileChange/requestApproval".to_owned(),
                params: json!({})
            })
        );
        assert!(parse(b"[1]").is_err());
        assert!(parse(b"{}").is_err());
        assert!(
            parse(br#"{"id":"x","result":1}"#).is_err(),
            "not an id it gave"
        );
    }
}
