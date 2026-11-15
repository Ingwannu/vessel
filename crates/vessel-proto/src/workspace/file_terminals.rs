//! Approved terminal commands from a project file; no process is started here.

use super::ProjectCommand;
use crate::{ProtocolVersion, SessionTitle};
use serde::{Deserialize, Serialize};

/// Assign only after the web dialog, web command-changes and native approval
/// card display and compare every terminal command and service port declaration,
/// including both choices of the allocation block.
/// Until all three land, `None` refuses these sets on negotiated connections;
/// clients also refuse sets whose complete contents they cannot display.
pub const PROJECT_FILE_KEYS: Option<ProtocolVersion> = None;

/// A shell command a person must see before approving a project's terminals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileTerminal {
    /// The terminal's title; a blank file name is represented as absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<SessionTitle>,
    /// Kept as validated text so approval covers exactly the shell input.
    pub command: ProjectCommand,
}
