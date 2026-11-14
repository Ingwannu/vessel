//! Literal executable paths decoded without the client's native path rules.
//! No filesystem access, normalization or process creation belongs here.

mod windows;

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use super::RelPath;

/// Which native grammars may interpret an approved path without changing its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecPathSyntax {
    /// A Unix root and literal `/`-separated components.
    Unix,
    /// A rooted Windows drive, UNC path or namespace.
    Windows,
    /// Both interpretations are well formed; invocation selects the node's grammar.
    Either,
}

/// An absolute executable path that a remote client can inspect on either host OS.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ExecPath {
    text: Arc<str>,
    syntax: ExecPathSyntax,
}

/// Why literal path text cannot safely identify an absolute program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidExecPath {
    /// A drive-relative or rooted-without-drive Windows path is not absolute.
    #[error("an executable path needs a Unix root or a rooted Windows prefix")]
    NotAbsolute,
    /// A root names no program to invoke.
    #[error("an executable path must name a program after its root")]
    NoProgram,
    /// Inspect raw components before a native parser could normalize them away.
    #[error("an executable path cannot have an empty, dot or parent component")]
    Component,
    /// Keep the approval's path bounded by the existing path representation.
    #[error("an executable path is at most {max} bytes", max = ExecPath::MAX_BYTES)]
    TooLong,
    /// NUL would truncate an operating-system argument.
    #[error("an executable path cannot contain NUL")]
    Nul,
}

impl ExecPath {
    /// Share the existing path bound rather than introducing a second limit.
    pub const MAX_BYTES: usize = RelPath::MAX_BYTES;

    /// The exact text that approval covers and a compatible node may invoke.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Let invocation refuse an incompatible host without reinterpreting this text.
    #[must_use]
    pub const fn syntax(&self) -> ExecPathSyntax {
        self.syntax
    }
}

impl FromStr for ExecPath {
    type Err = InvalidExecPath;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > Self::MAX_BYTES {
            return Err(InvalidExecPath::TooLong);
        }
        if text.contains('\0') {
            return Err(InvalidExecPath::Nul);
        }
        let unix = text
            .strip_prefix('/')
            .ok_or(InvalidExecPath::NotAbsolute)
            .and_then(|tail| validate_parts(tail, Separators::Unix));
        let syntax = match (unix, windows::validate(text)) {
            (Ok(()), Ok(())) => ExecPathSyntax::Either,
            (Ok(()), Err(_)) => ExecPathSyntax::Unix,
            (Err(_), Ok(())) => ExecPathSyntax::Windows,
            (Err(unix), Err(windows)) => {
                return Err(if windows == InvalidExecPath::NotAbsolute {
                    unix
                } else {
                    windows
                });
            }
        };
        Ok(Self {
            text: Arc::from(text),
            syntax,
        })
    }
}

impl fmt::Display for ExecPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

crate::key::serde_via_text!(ExecPath);

#[derive(Clone, Copy)]
enum Separators {
    Unix,
    Windows,
    Verbatim,
}

impl Separators {
    fn matches(self, character: char) -> bool {
        match self {
            Self::Unix => character == '/',
            Self::Windows => matches!(character, '/' | '\\'),
            Self::Verbatim => character == '\\',
        }
    }
}

fn valid_part(part: &str) -> bool {
    !part.is_empty() && part != "." && part != ".."
}

fn validate_parts(text: &str, separators: Separators) -> Result<(), InvalidExecPath> {
    if text.is_empty() {
        return Err(InvalidExecPath::NoProgram);
    }
    if text.split(|c| separators.matches(c)).all(valid_part) {
        Ok(())
    } else {
        Err(InvalidExecPath::Component)
    }
}

#[cfg(test)]
mod tests;
