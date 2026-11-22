//! Browser workspace choices belong to the page bridge, so they follow the
//! client across UI launches and do not change the node's PTY retention.

use serde::{Deserialize, Serialize};

/// Terminal history lines retained by the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "u32", into = "u32")]
pub struct TerminalScrollback(u32);

impl Default for TerminalScrollback {
    fn default() -> Self {
        Self(10_000)
    }
}

impl TryFrom<u32> for TerminalScrollback {
    type Error = ScrollbackOutOfRange;

    fn try_from(lines: u32) -> Result<Self, Self::Error> {
        if lines <= 1_000_000 {
            Ok(Self(lines))
        } else {
            Err(ScrollbackOutOfRange)
        }
    }
}

impl From<TerminalScrollback> for u32 {
    fn from(lines: TerminalScrollback) -> Self {
        lines.0
    }
}

/// A terminal history size beyond the client buffer limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("terminal scrollback must be between 0 and 1000000 lines")]
pub struct ScrollbackOutOfRange;

/// Settings shared by terminals on this page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TerminalPreferences {
    /// History lines in addition to the visible terminal rows.
    pub scrollback: TerminalScrollback,
}

/// Editing choices belong to the browser bridge, independent of the file's host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EditorPreferences {
    /// Use modal Vim bindings in the source editor.
    pub vim_keybindings: bool,
}

/// An action that opens workspace content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum OpenLocationSource {
    /// A file clicked or found in the explorer.
    ExplorerFiles,
    /// A change opened from an agent or workspace.
    Diffs,
    /// A file opened from the diff or Changes view.
    DiffFiles,
}

/// Where to open a workspace view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum OpenLocationDestination {
    /// The main tab strip.
    #[default]
    Main,
    /// The supporting pane on wide workspaces.
    Side,
}

/// Where to open a pull request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PullRequestLocation {
    /// The main tab strip.
    Main,
    /// The supporting pane on wide workspaces.
    Side,
    /// Open in the Files explorer.
    #[default]
    Explorer,
}

/// Destinations for the workspace actions the page serves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenLocationPreferences {
    /// Files selected in the explorer.
    pub explorer_files: OpenLocationDestination,
    /// Changes opened from the workspace or chat.
    pub diffs: OpenLocationDestination,
    /// Source files selected in Changes or Diff.
    pub diff_files: OpenLocationDestination,
    /// The workspace pull request.
    pub pull_requests: PullRequestLocation,
}
