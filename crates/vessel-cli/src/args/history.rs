//! `vessel history` (protocol 14, ADR 0053): an agent's own conversations,
//! to import one.

use std::path::PathBuf;

use clap::Args;
use vessel_proto::{HarnessRef, ProfileName};

/// `vessel history`'s arguments.
#[derive(Debug, Args)]
pub(crate) struct History {
    /// The agent: claude-code (or claude), codex, gemini, opencode, or an
    /// agent added on the node.
    pub(crate) harness: HarnessRef,
    /// Under this profile of the node (another account keeps other
    /// conversations); the harness's default when absent.
    #[arg(long)]
    pub(crate) profile: Option<ProfileName>,
    /// Only the conversations held in this folder on the node (relative:
    /// from here, for a node on this machine).
    #[arg(long)]
    pub(crate) cwd: Option<PathBuf>,
    /// Go on from an earlier listing's last line (`--more <cursor>`).
    #[arg(long)]
    pub(crate) more: Option<String>,
}
