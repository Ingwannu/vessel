//! Search and fetch display metadata; URLs here never initiate network I/O.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::{DetailCount, ToolByteCount};
use crate::{DurationMs, HttpUrl};

/// Closed upstream search sources select presentation without tool-name guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SearchSource {
    /// A general search.
    Search,
    /// Searching file contents.
    Grep,
    /// Matching file names.
    Glob,
    /// Searching the web.
    WebSearch,
}

/// Search mode describes the output, not permission to search new resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Matching lines or extracts.
    Content,
    /// File names only.
    FilesWithMatches,
    /// Match counts only.
    Count,
}

/// A search result keeps its target separate from its display title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WebSearchResult {
    /// A result's text is not interpreted as a URL.
    pub title: String,
    /// Public HTTP(S) address validated by the existing URL owner.
    pub url: HttpUrl,
}

/// Canonical search output retains both readable content and structured results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchDetail {
    /// The query is content and may contain punctuation or line breaks.
    pub query: String,
    /// Which search vocabulary the provider used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<SearchSource>,
    /// Readable output, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Matching paths; absent differs from an explicitly empty result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_paths: Option<Vec<PathBuf>>,
    /// Web results, when the provider returns structured targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_results: Option<Vec<WebSearchResult>>,
    /// Provider notes are content, never executable markup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<String>>,
    /// Total files, independent of how many are present in this output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_files: Option<DetailCount>,
    /// Total matches, independent of the displayed output length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_matches: Option<DetailCount>,
    /// Decoders normalize observed timing to the existing duration unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationMs>,
    /// Whether the provider reported a cut output, not a client guess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<ToolTruncation>,
    /// How the output should be interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<SearchMode>,
}

/// An explicit truncation state avoids unexplained boolean arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ToolTruncation {
    /// The provider says the output is complete.
    Complete,
    /// The provider says some output was cut.
    Truncated,
}

/// Fetch results preserve status and size independently of the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FetchDetail {
    /// An observed public URL, never an instruction to fetch it.
    pub url: HttpUrl,
    /// Optional extraction instructions as content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Returned body or extraction result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Reported status, retained even for unsuccessful fetches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<FetchStatusCode>,
    /// Provider's status phrase, not a closed status discriminant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_text: Option<String>,
    /// Reported byte count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<ToolByteCount>,
    /// Reported duration in the common unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationMs>,
}

/// Reported status has its own unit without assuming a specific HTTP version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct FetchStatusCode(u32);

impl FetchStatusCode {
    /// Preserve an observed status without treating zero as absence.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Its numeric spelling for presentation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
