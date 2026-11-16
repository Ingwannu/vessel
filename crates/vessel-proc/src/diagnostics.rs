//! Diagnostic policy for command data; never changes what the OS executes.
use crate::CommandSpec;
use std::{fmt, io, path::Path};

/// Whether process diagnostics may include caller-provided command content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandDiagnostics {
    /// Existing program/argument diagnostics; environment values stay hidden.
    #[default]
    Standard,
    /// Hide program, argv, paths and backend error text for private commands.
    Redacted,
}

impl CommandSpec {
    /// Set diagnostic privacy without changing program, arguments or environment.
    #[must_use]
    pub const fn with_diagnostics(mut self, policy: CommandDiagnostics) -> Self {
        self.diagnostics = policy;
        self
    }

    /// The process/PTY boundary applies the same policy to logs and errors.
    #[must_use]
    pub const fn diagnostic_policy(&self) -> CommandDiagnostics {
        self.diagnostics
    }

    /// A diagnostic label only; never use it to execute or resolve a program.
    #[must_use]
    pub fn diagnostic_program(&self) -> &Path {
        match self.diagnostics {
            CommandDiagnostics::Standard => &self.program,
            CommandDiagnostics::Redacted => Path::new("<redacted>"),
        }
    }

    /// Backend errors may quote a command/path even when their outer label hides it.
    #[must_use]
    pub fn diagnostic_message(&self, error: &impl fmt::Display) -> String {
        match self.diagnostics {
            CommandDiagnostics::Standard => error.to_string(),
            CommandDiagnostics::Redacted => "private command detail omitted".to_owned(),
        }
    }

    /// Retain the original cause privately, without exposing its text or chain.
    #[must_use]
    pub fn diagnostic_io_error(&self, error: io::Error) -> io::Error {
        match self.diagnostics {
            CommandDiagnostics::Standard => error,
            CommandDiagnostics::Redacted => {
                io::Error::new(error.kind(), PrivateIoCause { _original: error })
            }
        }
    }
}

// The existing io::Error owns this concrete cause. Public formatting and source
// traversal terminate here, but the actual original error/evidence remains owned
// until the returned error is dropped. No raw accessor or text sanitizer exists.
struct PrivateIoCause {
    _original: io::Error,
}
impl fmt::Debug for PrivateIoCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateIoCause(<redacted>)")
    }
}
impl fmt::Display for PrivateIoCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private command I/O detail omitted")
    }
}
impl std::error::Error for PrivateIoCause {}

#[cfg(test)]
mod tests;
