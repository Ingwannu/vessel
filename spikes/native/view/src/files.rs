//! Why one of the app's own files (the machine list, the preferences) did
//! not read: where, and what kind of problem. Never the file's text: the
//! machine list may hold a token, and an error is shown to whoever looks
//! at the screen (the client's `file_fault`, the same rule; the
//! coordinator's review of 0.0.0.0.8, 2026-11-13). The words are the
//! app's, in both languages (`text/files.rs`).

use serde::{Deserialize, Serialize};

/// Why a file did not read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFault {
    /// The file, as display text.
    pub path: String,
    /// The line and column (both from one), when the reader said.
    pub at: Option<(usize, usize)>,
    /// What kind of problem.
    pub problem: FileProblem,
    /// The key on that line when it is one of the file's own keys (the
    /// reader names no other: never a word taken from the file).
    pub field: Option<String>,
}

/// What kind of problem stopped a file from reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileProblem {
    /// Not TOML.
    NotToml,
    /// Not JSON.
    NotJson,
    /// A key the file does not take.
    UnknownKey,
    /// A key given twice.
    DuplicateKey,
    /// A key the file needs is missing.
    MissingKey,
    /// A value of the wrong kind.
    WrongKind,
    /// The file is there but cannot be opened (its owner or mode).
    CannotOpen,
}

impl FileProblem {
    /// The kind of a JSON reader's error from the start of its message
    /// (serde's fixed wording; nothing of it is kept) and whether the text
    /// was JSON at all.
    #[must_use]
    pub fn of_json(message: &str, syntax: bool) -> Self {
        let starts = |words: &str| message.trim_start().starts_with(words);
        if syntax {
            Self::NotJson
        } else if starts("unknown field") {
            Self::UnknownKey
        } else if starts("duplicate field") {
            Self::DuplicateKey
        } else if starts("missing field") {
            Self::MissingKey
        } else {
            Self::WrongKind
        }
    }
}
