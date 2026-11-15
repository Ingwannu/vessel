//! A script's run as the node keeps it: what it ran, so it can run again
//! under the same approval.

use std::path::PathBuf;
use std::time::Duration;

use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::terminals::{TerminalKind, TerminalPlace};
use vessel_proto::workspace::CommandsDigest;

use crate::workspace::ScriptRoot;

/// The longest `terminal.close` and an ending session wait for a terminal
/// to end after it was killed: the scripts' grace and five seconds more.
/// A process stuck in the kernel is forgotten anyway, and logged.
pub(crate) const CLOSE_WAIT: Duration = Duration::from_secs(10);

/// What a run ran.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    /// Where it was placed.
    pub(crate) place: TerminalPlace,
    /// The checkout.
    pub(crate) script_root: ScriptRoot,
    /// The folder `vessel.toml` was read in (the checkout's).
    pub(crate) root: PathBuf,
    /// The script, as parsed from the approved file.
    pub(crate) entry: ScriptEntry,
    /// The approved set it came from.
    pub(crate) digest: CommandsDigest,
}

impl Plan {
    /// The terminal kind of its run.
    pub(crate) fn kind(&self) -> TerminalKind {
        match self.entry.kind() {
            ScriptKind::Script => TerminalKind::Script {
                script: self.entry.name.clone(),
                command: self.entry.command.clone(),
            },
            ScriptKind::Service => TerminalKind::Service {
                script: self.entry.name.clone(),
                command: self.entry.command.clone(),
            },
        }
    }
}
