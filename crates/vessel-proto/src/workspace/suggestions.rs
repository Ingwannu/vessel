//! Validated root-scoped picker discovery (ADR 0083).

use serde::{Deserialize, Serialize};

use super::{RelPath, WorkspaceRoot, trim_project_text};

/// Vessel resource floor on raw Unicode scalars before trim or disk work.
/// Paseo's wire query is unbounded; this is a deliberate admission difference.
pub const MAX_SUGGESTION_QUERY_CHARS: usize = 256;
/// Paseo's picker default result count.
pub const DEFAULT_SUGGESTIONS: u16 = 30;
/// Paseo's maximum result count.
pub const MAX_SUGGESTIONS: u16 = 100;

/// A validated result count, decoded before root resolution or disk work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u16", into = "u16")]
pub struct SuggestionLimit(u16);

/// A picker cannot allocate an unbounded response or request zero entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("directory suggestions require 1 to {MAX_SUGGESTIONS} results")]
pub struct InvalidSuggestionLimit;

impl TryFrom<u16> for SuggestionLimit {
    type Error = InvalidSuggestionLimit;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value == 0 || value > MAX_SUGGESTIONS {
            Err(InvalidSuggestionLimit)
        } else {
            Ok(Self(value))
        }
    }
}

impl Default for SuggestionLimit {
    fn default() -> Self {
        Self(DEFAULT_SUGGESTIONS)
    }
}

impl From<SuggestionLimit> for u16 {
    fn from(limit: SuggestionLimit) -> Self {
        limit.0
    }
}

impl SuggestionLimit {
    /// Validated entry count, distinct from a byte budget.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A bounded, possibly empty query; empty lists immediate children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct SuggestionQuery(String);

/// Refusal occurs before directory discovery, never after starting work.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a directory query is one line of at most {MAX_SUGGESTION_QUERY_CHARS} characters")]
pub struct InvalidSuggestionQuery;

impl TryFrom<String> for SuggestionQuery {
    type Error = InvalidSuggestionQuery;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.chars().count() > MAX_SUGGESTION_QUERY_CHARS
            || text.chars().any(|character| {
                character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
            })
        {
            return Err(InvalidSuggestionQuery);
        }
        // Match JavaScript BOM/space trimming only after refusing controls; a
        // prohibited edge control must not disappear before admission checks.
        Ok(Self(trim_project_text(&text).to_owned()))
    }
}

impl From<SuggestionQuery> for String {
    fn from(query: SuggestionQuery) -> Self {
        query.0
    }
}

impl SuggestionQuery {
    /// Text to score; an empty query deliberately means a directory listing.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which entries a picker accepts; avoids independent boolean combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SuggestionKinds {
    /// Only regular files.
    Files,
    /// Only directories.
    Directories,
    /// Both files and directories.
    Both,
}

/// Reference matching modes, distinct from exact-path retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SuggestionMatch {
    /// A scored Unicode subsequence.
    Fuzzy,
    /// A case-insensitive path suffix.
    Suffix,
}

/// A picker query within a registered workspace, never an arbitrary host root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SuggestEntries {
    /// Root authorization uses the same boundary as file reads.
    pub root: WorkspaceRoot,
    /// Directory below that root to discover.
    pub directory: RelPath,
    /// Bounded query; blank lists children without walking descendants.
    pub query: SuggestionQuery,
    /// Entry kinds the caller can use.
    pub kinds: SuggestionKinds,
    /// How paths match the query.
    pub matching: SuggestionMatch,
    /// Requested result budget, validated by the node against the shared maximum.
    pub limit: Option<SuggestionLimit>,
}

/// The two entry kinds a picker can actually consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SuggestedEntryKind {
    /// A regular file.
    File,
    /// A directory eligible for discovery.
    Directory,
}

/// A usable regular file or directory, with a validated in-root path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SuggestedEntry {
    /// Relative to the authorized workspace, never a client-local path.
    pub path: RelPath,
    /// Files/directories only; symlinks and special entries are excluded.
    pub kind: SuggestedEntryKind,
}

/// Results are bounded and explicitly disclose an incomplete scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EntrySuggestions {
    /// Best matches first; deterministic path order breaks score ties.
    pub entries: Vec<SuggestedEntry>,
    /// Whether discovery reached its depth/entry budget or results were clipped.
    pub more: bool,
}

#[cfg(test)]
#[path = "suggestions_tests.rs"]
mod tests;
