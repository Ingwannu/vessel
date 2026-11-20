//! Pasted text and a path quoted for the node's shell.
//!
//! The path is text the node named. This module does not open it. The
//! result is the characters to type, which is why it is a `String` and
//! not a path.

use super::grid::TerminalModel;
use super::types::TerminalError;

/// Whether the program asked for bracketed paste (DEC private mode 2004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Bracketed {
    /// Paste is typed as itself.
    #[default]
    Off,
    /// Paste is wrapped so the program can tell it from typing.
    On,
}

/// The operating system of the node whose shell receives the paste.
///
/// The branch is the node's, not the path's shape and not the app's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePlatform {
    /// A POSIX shell.
    Posix,
    /// A Windows shell.
    Windows,
}

const DANGEROUS: &[char] = &['`', '$', '|', '&', '>', '~', '#', '!', '^', '*', ';', '<'];

impl TerminalModel {
    /// `text` as the bytes to type, bracketed when the program asked.
    ///
    /// Nothing is sent.
    #[must_use]
    pub fn paste(&self, text: &str) -> Vec<u8> {
        encode_paste(text, self.bracketed())
    }
}

/// Bracketed-paste wrapping. The mode is the program's, already parsed.
#[must_use]
fn encode_paste(text: &str, mode: Bracketed) -> Vec<u8> {
    match mode {
        Bracketed::Off => text.as_bytes().to_vec(),
        Bracketed::On => {
            let mut out = Vec::new();
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(b"\x1b[201~");
            out
        }
    }
}

/// Quote `path` for `platform`'s shell.
///
/// POSIX refuses `` ` $ | & > ~ # ! ^ * ; < `` instead of deleting them:
/// deleting would paste a different path. An accepted path doubles
/// backslashes and then uses Paseo's three quote forms. Windows does not
/// refuse those characters; a space is wrapped in double quotes.
///
/// # Errors
///
/// [`TerminalError::Path`] when a POSIX path contains a character Paseo
/// would delete. The message does not include the path.
pub fn quote_path(path: &str, platform: NodePlatform) -> Result<String, TerminalError> {
    match platform {
        NodePlatform::Windows => Ok(quote_windows(path)),
        NodePlatform::Posix => quote_posix(path),
    }
}

fn quote_windows(path: &str) -> String {
    if path.chars().any(|ch| ch == ' ') {
        let mut out = String::new();
        out.push('"');
        out.push_str(path);
        out.push('"');
        out
    } else {
        path.to_owned()
    }
}

fn quote_posix(path: &str) -> Result<String, TerminalError> {
    if path.chars().any(|ch| DANGEROUS.contains(&ch)) {
        return Err(TerminalError::Path);
    }
    Ok(quote_forms(&double_backslashes(path)))
}

fn double_backslashes(path: &str) -> String {
    let mut out = String::new();
    for ch in path.chars() {
        if ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn quote_forms(text: &str) -> String {
    let single = text.chars().any(|ch| ch == '\'');
    let double = text.chars().any(|ch| ch == '"');
    let mut out = String::new();
    if single && double {
        out.push_str("$'");
        push_escaped(&mut out, text);
        out.push('\'');
    } else if single {
        out.push('\'');
        push_escaped(&mut out, text);
        out.push('\'');
    } else {
        out.push('\'');
        out.push_str(text);
        out.push('\'');
    }
    out
}

fn push_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        if ch == '\'' {
            out.push('\\');
        }
        out.push(ch);
    }
}
