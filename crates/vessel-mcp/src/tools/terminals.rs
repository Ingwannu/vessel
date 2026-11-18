//! The terminal tools (protocol 11, ADR 0031): open a shell beside a
//! session, read it, type into it, close it, and see the ports its tree
//! listens on. The node allows them only where a person turned agent
//! terminals on in the session's profile, and only on terminals this agent
//! opened.
//!
//! [Decision Log]
//! - `read_terminal` returns text: the records' bytes, escape sequences
//!   removed, at most [`READ_BYTES`] a call, and the cursor (the next
//!   record's number) to read on from. The node keeps no terminal
//!   emulator, so this is the output, not the picture a full-screen
//!   program draws; an escape sequence split across two records may leave
//!   a stray character.
//! - `write_terminal` is marked destructive and open-world: typing into a
//!   shell runs programs, and a harness that asks before such tools asks.
//! - Keys by name (`enter`, `ctrl_c`, …) so an agent never has to write a
//!   control character into JSON.

use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use serde_json::{Value, json};
use vessel_proto::ports::ListPorts;
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, ListTerminals, ReadTerminal, SendTerminal, TerminalPlace,
};
use vessel_proto::{EventSeq, SessionEvent, SessionId, TerminalBytes, TerminalId, TerminalInput};

use super::{Target, ToolError, VesselTools, number, required, text, tool};

/// The most text one `read_terminal` returns.
pub(super) const READ_BYTES: usize = 32 * 1024;
/// The most text one `write_terminal` sends.
pub(super) const WRITE_BYTES: usize = 16 * 1024;
/// The longest `read_terminal` waits for output.
const MAX_WAIT_SECS: u64 = 30;

impl VesselTools {
    pub(super) async fn terminals(
        &self,
        name: &str,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        match name {
            "create_terminal" => self.create_terminal(node, args).await,
            "list_terminals" => Ok(json!(self.on(node, ListTerminals { place: None }).await?)),
            "read_terminal" => self.read_terminal(node, args).await,
            "write_terminal" => self.write_terminal(node, args).await,
            "close_terminal" => {
                let terminal = terminal(args)?;
                let _ack = self.on(node, CloseTerminal { terminal }).await?;
                Ok(json!({ "closed": terminal }))
            }
            "list_ports" => Ok(json!(
                self.on(node, ListPorts { changed_from: None }).await?
            )),
            _ => Err(ToolError::Arguments("unknown terminal tool".into())),
        }
    }

    async fn create_terminal(&self, node: Target, args: &JsonObject) -> Result<Value, ToolError> {
        let session = match text(args, "session")? {
            Some(session) => session
                .parse::<SessionId>()
                .map_err(|_invalid| ToolError::Arguments("`session` is not a session id".into()))?,
            None => self.own_session().ok_or_else(|| {
                ToolError::Arguments("give `session`: this agent's own is not known".into())
            })?,
        };
        let title = text(args, "title")?
            .map(|title| {
                title
                    .parse()
                    .map_err(|_invalid| ToolError::Arguments("`title` is one line".into()))
            })
            .transpose()?;
        let info = self
            .on(
                node,
                CreateTerminal {
                    place: TerminalPlace::Session { session },
                    size: None,
                    title,
                },
            )
            .await?;
        Ok(json!({ "terminal": info }))
    }

    async fn read_terminal(&self, node: Target, args: &JsonObject) -> Result<Value, ToolError> {
        let terminal = terminal(args)?;
        let from = EventSeq::new(number(args, "from")?.unwrap_or(0));
        let wait = number(args, "wait_secs")?
            .map(|seconds| u32::try_from(seconds.min(MAX_WAIT_SECS)).unwrap_or(0));
        let page = self
            .on(
                node,
                ReadTerminal {
                    terminal,
                    from,
                    limit: ReadTerminal::MAX_LIMIT,
                    wait,
                },
            )
            .await?;
        let mut output = Vec::new();
        let mut next = page.records.first().map_or(page.next, |record| record.seq);
        let mut ended = None;
        for record in &page.records {
            match &record.event {
                SessionEvent::Terminal(chunk) => {
                    if output.len().saturating_add(chunk.data.bytes().len()) > READ_BYTES
                        && !output.is_empty()
                    {
                        break;
                    }
                    output.extend_from_slice(chunk.data.bytes());
                }
                SessionEvent::Status { status } => {
                    if !status.is_live() {
                        ended = Some(status.clone());
                    }
                }
                // Vessel's own lines (a service's restarts), as lines.
                SessionEvent::Agent {
                    event: vessel_proto::AgentEvent::HarnessLog { line },
                } => {
                    output.extend_from_slice(b"\n[");
                    output.extend_from_slice(line.as_bytes());
                    output.extend_from_slice(b"]\n");
                }
                SessionEvent::Agent { .. }
                | SessionEvent::ProfileChanged { .. }
                | SessionEvent::ModelCall(_)
                | SessionEvent::OptionsChanged { .. }
                | SessionEvent::Renamed { .. }
                | SessionEvent::QueueChanged { .. }
                | SessionEvent::CommandsChanged { .. }
                | SessionEvent::UsageChanged { .. }
                | SessionEvent::Parked { .. }
                | SessionEvent::Woken
                | SessionEvent::Forked { .. }
                | SessionEvent::Rewound { .. }
                | SessionEvent::Reloaded { .. } => {}
            }
            next = record.seq.next().unwrap_or(record.seq);
        }
        let skipped = page.records.first().is_some_and(|record| record.seq > from);
        Ok(json!({
            "text": plain_text(&output),
            "next": next.get(),
            "at_end": next == page.next && page.at_end,
            "ended": ended,
            "dropped_before": skipped.then(|| page.records.first().map(|record| record.seq.get())),
        }))
    }

    async fn write_terminal(&self, node: Target, args: &JsonObject) -> Result<Value, ToolError> {
        let terminal = terminal(args)?;
        let mut bytes = text(args, "text")?.unwrap_or_default().as_bytes().to_vec();
        if let Some(keys) = args.get("keys") {
            let keys = keys
                .as_array()
                .ok_or_else(|| ToolError::Arguments("`keys` is a list of key names".into()))?;
            for key in keys {
                let name = key
                    .as_str()
                    .ok_or_else(|| ToolError::Arguments("a key is a name".into()))?;
                bytes.extend_from_slice(key_bytes(name).ok_or_else(|| {
                    ToolError::Arguments(format!("no key `{name}`; see the tool's description"))
                })?);
            }
        }
        if bytes.is_empty() {
            return Err(ToolError::Arguments("give `text`, `keys` or both".into()));
        }
        if bytes.len() > WRITE_BYTES {
            return Err(ToolError::Arguments("at most 16 KiB a call".into()));
        }
        let written = bytes.len();
        let _ack = self
            .on(
                node,
                SendTerminal {
                    terminal,
                    input: TerminalInput::Data {
                        data: TerminalBytes::new(bytes.into()),
                    },
                },
            )
            .await?;
        Ok(json!({ "written": written }))
    }
}

fn terminal(args: &JsonObject) -> Result<TerminalId, ToolError> {
    required(args, "terminal")?
        .parse()
        .map_err(|_invalid| ToolError::Arguments("`terminal` is not a terminal id".into()))
}

/// The bytes a named key sends (xterm's).
fn key_bytes(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "enter" => b"\r",
        "tab" => b"\t",
        "escape" => b"\x1b",
        "backspace" => b"\x7f",
        "up" => b"\x1b[A",
        "down" => b"\x1b[B",
        "right" => b"\x1b[C",
        "left" => b"\x1b[D",
        "ctrl_c" => b"\x03",
        "ctrl_d" => b"\x04",
        "ctrl_z" => b"\x1a",
        _ => return None,
    })
}

/// Terminal output as text a model reads: escape sequences (CSI, OSC and
/// two-byte ones) removed, carriage returns before a newline dropped,
/// other control characters but newline and tab removed, and bytes that
/// are not UTF-8 replaced.
fn plain_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\u{1b}' => match chars.next() {
                // CSI: parameters and intermediates, then a final byte.
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&next) {
                            break;
                        }
                    }
                }
                // OSC: up to BEL or ST (ESC \).
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                            let _st = chars.next();
                            break;
                        }
                    }
                }
                // A two-byte sequence (ESC =, ESC 7, ESC ( B …).
                Some('(' | ')') => {
                    let _set = chars.next();
                }
                Some(_) | None => {}
            },
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\t' => out.push(character),
            other if other.is_control() => {}
            other => out.push(other),
        }
    }
    out
}

pub(super) fn tools(node: &Value) -> Vec<Tool> {
    let terminal = json!({ "type": "string", "description": "A terminal id." });
    vec![
        tool(
            "create_terminal",
            "Open a shell beside a session: your own (the default) or one you started. Allowed \
             only where a person turned agent terminals on, for your session's profile and for \
             that session's; at most 4 at once. It runs in the session's folder, without your \
             credentials, and ends when your session or that session ends.",
            json!({
                "node": node,
                "session": { "type": "string", "description": "A session id; yours by default." },
                "title": { "type": "string" }
            }),
            &[],
        ),
        tool(
            "list_terminals",
            "The terminals you opened, live and ended.",
            json!({ "node": node }),
            &[],
        ),
        tool(
            "read_terminal",
            "A terminal's output as text (escape sequences removed), at most 32 KiB, from a \
             cursor (`from`, 0 for the start; `next` in the answer reads on). The oldest output \
             is dropped past 1 MiB (`dropped_before`). With `wait_secs`, waits for output.",
            json!({
                "node": node,
                "terminal": terminal,
                "from": { "type": "integer", "minimum": 0 },
                "wait_secs": { "type": "integer", "minimum": 0, "maximum": 30 }
            }),
            &["terminal"],
        ),
        tool(
            "write_terminal",
            "Type into a terminal you opened: `text` as is, then `keys` by name (enter, tab, \
             escape, backspace, up, down, left, right, ctrl_c, ctrl_d, ctrl_z). At most 16 KiB. \
             This runs programs, as your own shell tool does.",
            json!({
                "node": node,
                "terminal": terminal,
                "text": { "type": "string" },
                "keys": { "type": "array", "items": { "type": "string" } }
            }),
            &["terminal"],
        )
        .annotate(ToolAnnotations::new().destructive(true).open_world(true)),
        tool(
            "close_terminal",
            "End a terminal you opened and forget it.",
            json!({ "node": node, "terminal": terminal }),
            &["terminal"],
        ),
        tool(
            "list_ports",
            "The TCP ports your session's and your terminals' processes listen on (scanned every \
             few seconds while asked), with the address: 127.0.0.1 or ::1 is this machine only.",
            json!({ "node": node }),
            &[],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_sequences_are_removed() {
        let raw = b"\x1b[1;32mok\x1b[0m done\r\n\x1b]0;title\x07$ \x1b(Bls\r\n\x07";
        assert_eq!(plain_text(raw), "ok done\n$ ls\n");
    }

    #[test]
    fn keys_by_name() {
        assert_eq!(key_bytes("enter"), Some(&b"\r"[..]));
        assert_eq!(key_bytes("ctrl_c"), Some(&b"\x03"[..]));
        assert_eq!(key_bytes("f13"), None);
    }

    #[test]
    fn only_the_write_is_destructive() {
        let tools = tools(&json!({}));
        for tool in &tools {
            let destructive = tool
                .annotations
                .as_ref()
                .is_some_and(ToolAnnotations::is_destructive);
            assert_eq!(destructive, tool.name == "write_terminal", "{}", tool.name);
        }
    }
}
