//! File tool descriptions are display data, never file-system authorization.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::{DetailCount, DetailOffset};

/// A file read retains the provider's reported range convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadDetail {
    /// Source path as reported on the node.
    pub file_path: PathBuf,
    /// Returned content, independent of the read range metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The provider's original offset, without guessing its base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<DetailOffset>,
    /// Reported requested line count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<DetailCount>,
}

/// A replacement edit can carry both a structured replacement and unified diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EditDetail {
    /// File affected; no path is opened by deserializing this detail.
    pub file_path: PathBuf,
    /// Original content, absent when not reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_string: Option<String>,
    /// Replacement content; empty means deletion when explicitly reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_string: Option<String>,
    /// The provider's diff is retained without inventing file contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unified_diff: Option<String>,
}

/// Whole-file writes stay distinct from edits for client presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WriteDetail {
    /// The output file path.
    pub file_path: PathBuf,
    /// Written content, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}
