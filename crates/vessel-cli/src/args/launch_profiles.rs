//! Named launch bundles are distinct from the account `profiles` command.

use clap::Subcommand;
use std::path::PathBuf;
use vessel_proto::SessionId;

#[derive(Debug, Subcommand)]
pub(crate) enum LaunchProfilesCommand {
    /// Read the complete ordered snapshot as JSON (including its CAS revision).
    List,
    /// Replace the whole list from a typed JSON request: expected and profiles.
    Replace { input: PathBuf },
    /// Probe a typed JSON request containing selection; never sends a prompt.
    Options { input: PathBuf },
    /// Read the presets applicable to this structured session's actual account.
    Applicable { session: SessionId },
    /// Apply a typed JSON request containing session and ordered options.
    /// A failed prefix is printed in full and the command exits unsuccessfully.
    Apply { input: PathBuf },
}
