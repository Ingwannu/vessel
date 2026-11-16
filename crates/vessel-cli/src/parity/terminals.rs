//! Terminal CLI effects use existing terminal calls; capture stays gated with its node handler.
use super::output::{self, Row};
use crate::{
    args::{
        Target,
        parity::{Presentation, TerminalCommand, TerminalSelector},
    },
    connect::connect,
    error::CliError,
};
use bytes::Bytes;
use serde::Serialize;
use std::path::PathBuf;
use vessel_client::NodeClient;
use vessel_proto::{
    TerminalBytes, TerminalId, TerminalInput,
    terminals::{KillTerminal, ListTerminals, SendTerminal, TerminalInfo},
};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    command: TerminalCommand,
) -> Result<(), CliError> {
    match command {
        TerminalCommand::Ls {
            all,
            cwd,
            workspace,
        } => {
            if workspace.is_some() {
                return Err(CliError::ParityBlocked(
                    "terminal --workspace awaits the reviewed workspace placement contract",
                ));
            }
            let cwd = if all {
                None
            } else {
                Some(match cwd {
                    Some(path) => std::path::absolute(path)?,
                    None => std::env::current_dir()?,
                })
            };
            list(target, presentation, cwd).await
        }
        TerminalCommand::Create { .. } => Err(CliError::ParityBlocked(
            "terminal create requires real workspace identity; project/session IDs must not be used as substitutes",
        )),
        TerminalCommand::Capture {
            id,
            start,
            end,
            scrollback,
            ansi,
        } => {
            if vessel_proto::host_registry::CLI_PARITY_PROTOCOL.is_none() {
                return Err(CliError::ParityBlocked(
                    "terminal capture awaits the integrator's CLI_PARITY_PROTOCOL assignment",
                ));
            }
            let start = if scrollback {
                Some(
                    "0".parse()
                        .map_err(|_invalid| CliError::Usage("invalid scrollback range"))?,
                )
            } else {
                start
            };
            let options = vessel_proto::terminals::CaptureOptions {
                range: vessel_proto::terminals::CaptureRange { start, end },
                style: if ansi {
                    vessel_proto::terminals::CaptureStyle::Preserve
                } else {
                    vessel_proto::terminals::CaptureStyle::Plain
                },
            };
            capture(target, presentation, id, options).await
        }
        TerminalCommand::Kill { id } => {
            let client = connect(target).await?.client;
            let terminal = resolve(&client, &id).await?;
            let _ack = client
                .call(KillTerminal {
                    terminal: terminal.id,
                })
                .await?;
            output::record(
                presentation,
                &Killed {
                    terminal_id: terminal.id,
                    success: output::Success,
                },
            )
            .await
        }
        TerminalCommand::SendKeys { id, keys, literal } => {
            let input = encode_keys(
                &keys,
                if literal {
                    KeyMode::Literal
                } else {
                    KeyMode::Tokens
                },
            )?;
            let keys_sent = KeyCount(input.encode_utf16().count());
            send_keys(
                target,
                presentation,
                id,
                TerminalBytes::new(Bytes::from(input)),
                keys_sent,
            )
            .await
        }
    }
}

/// None means the explicit --all scope; folder selection is resolved before connection.
async fn list(
    target: &Target,
    presentation: Presentation,
    cwd: Option<PathBuf>,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let entries = client
        .call(ListTerminals { place: None })
        .await?
        .terminals
        .into_iter()
        .filter(|t| cwd.as_ref().is_none_or(|cwd| &t.folder == cwd))
        .collect::<Vec<_>>();
    let rows = entries.into_iter().map(ListedTerminal).collect::<Vec<_>>();
    output::list(presentation, &rows).await
}

/// Keep the actual native structured fields until terminal `WorkspaceId` lands.
#[derive(Serialize)]
#[serde(transparent)]
struct ListedTerminal(TerminalInfo);
impl Row for ListedTerminal {
    fn id(&self) -> String {
        self.0.id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["ID", "NAME", "CWD", "WORKSPACE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.0.id.to_string().chars().take(8).collect(),
            self.0
                .title
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
            self.0.folder.display().to_string(),
            // A project or session identifier is not a workspace identity.
            "-".to_owned(),
        ]
    }
}

async fn capture(
    target: &Target,
    presentation: Presentation,
    id: TerminalSelector,
    options: vessel_proto::terminals::CaptureOptions,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let terminal = resolve(&client, &id).await?;
    let result = client.capture_terminal(terminal.id, options).await?;
    match presentation {
        Presentation::Json | Presentation::Yaml | Presentation::Quiet => {
            output::single(
                presentation,
                &terminal.id.to_string(),
                &Captured {
                    terminal_id: terminal.id,
                    lines: result.lines,
                    total_lines: result.total_lines,
                },
            )
            .await
        }
        Presentation::Table(_) => {
            if result.lines.is_empty() {
                Ok(())
            } else {
                Ok(crate::io::out(format!("{}\n", result.lines.join("\n")).as_bytes()).await?)
            }
        }
    }
}

async fn send_keys(
    target: &Target,
    presentation: Presentation,
    id: TerminalSelector,
    input: TerminalBytes,
    keys_sent: KeyCount,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let terminal = resolve(&client, &id).await?;
    let _ack = client
        .call(SendTerminal {
            terminal: terminal.id,
            input: TerminalInput::Data { data: input },
        })
        .await?;
    match presentation {
        Presentation::Table(_) => Ok(()),
        Presentation::Json | Presentation::Yaml | Presentation::Quiet => {
            output::single(
                presentation,
                &terminal.id.to_string(),
                &Sent {
                    terminal_id: terminal.id,
                    keys_sent,
                },
            )
            .await
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Killed {
    terminal_id: TerminalId,
    success: output::Success,
}
impl Row for Killed {
    fn id(&self) -> String {
        self.terminal_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["ID", "SUCCESS"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.terminal_id.to_string().chars().take(8).collect(),
            "true".to_owned(),
        ]
    }
}
#[derive(Serialize)]
#[serde(transparent)]
struct KeyCount(usize);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sent {
    terminal_id: TerminalId,
    keys_sent: KeyCount,
}

async fn resolve(client: &NodeClient, query: &TerminalSelector) -> Result<TerminalInfo, CliError> {
    let entries = client.call(ListTerminals { place: None }).await?.terminals;
    pick(entries, query)
}
fn pick(entries: Vec<TerminalInfo>, selector: &TerminalSelector) -> Result<TerminalInfo, CliError> {
    let query = selector.text();
    if let Some(entry) = entries.iter().find(|entry| entry.id.to_string() == query) {
        return Ok(entry.clone());
    }
    let folded = query.to_lowercase();
    let prefixes = entries
        .iter()
        .filter(|entry| entry.id.to_string().starts_with(&folded))
        .cloned()
        .collect::<Vec<_>>();
    if !prefixes.is_empty() {
        return crate::workspace::one(prefixes, query, "terminal ID");
    }
    let names = entries
        .iter()
        .filter(|entry| {
            entry
                .title
                .as_ref()
                .is_some_and(|title| title.as_str().to_lowercase() == folded)
        })
        .cloned()
        .collect::<Vec<_>>();
    if !names.is_empty() {
        return crate::workspace::one(names, query, "terminal name");
    }
    let names = entries
        .into_iter()
        .filter(|entry| {
            entry
                .title
                .as_ref()
                .is_some_and(|title| title.as_str().to_lowercase().contains(&folded))
        })
        .collect();
    crate::workspace::one(names, query, "terminal name")
}
#[derive(Clone, Copy)]
enum KeyMode {
    Literal,
    Tokens,
}
fn encode_keys(keys: &[String], mode: KeyMode) -> Result<String, CliError> {
    let mut input = String::new();
    for key in keys {
        let text = match mode {
            KeyMode::Literal => key.as_str(),
            KeyMode::Tokens => match key.as_str() {
                "Enter" => "\r",
                "Tab" => "\t",
                "Escape" => "\u{1b}",
                "Space" => " ",
                "BSpace" => "\u{7f}",
                "C-c" => "\u{3}",
                "C-d" => "\u{4}",
                "C-z" => "\u{1a}",
                "C-l" => "\u{c}",
                "C-a" => "\u{1}",
                "C-e" => "\u{5}",
                other => other,
            },
        };
        if input
            .len()
            .checked_add(text.len())
            .is_none_or(|bytes| bytes > SendTerminal::MAX_BYTES)
        {
            return Err(CliError::Usage(
                "terminal input exceeds the existing terminal.send byte budget",
            ));
        }
        input.push_str(text);
    }
    Ok(input)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Captured {
    terminal_id: TerminalId,
    lines: Vec<String>,
    total_lines: vessel_proto::terminals::CaptureLineCount,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_tokens_are_case_sensitive_and_literal_mode_does_not_expand() {
        let keys = [
            "한😀".to_owned(),
            "Enter".to_owned(),
            "C-c".to_owned(),
            "enter".to_owned(),
        ];
        assert_eq!(
            encode_keys(&keys, KeyMode::Tokens).unwrap(),
            "한😀\r\u{3}enter"
        );
        assert_eq!(
            encode_keys(&keys, KeyMode::Literal).unwrap(),
            "한😀EnterC-center"
        );
    }
}
