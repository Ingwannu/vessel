//! Host/home aliases map onto the existing connection target before effects.

use crate::{
    args::{Command, Target},
    error::CliError,
};
use clap::Args;
use std::path::PathBuf;

/// Keep legacy --node/--data-dir together usable while --host/--home conflict.
#[derive(Debug, Default, Args)]
pub(crate) struct ConnectionOptions {
    #[arg(long, global = true, conflicts_with = "node")]
    host: Option<String>,
    /// Node data directory; native init keeps --home as its account-scan root.
    #[arg(long, global = true)]
    home: Option<PathBuf>,
}
impl ConnectionOptions {
    pub(crate) fn apply(self, target: &mut Target, command: &Command) -> Result<(), CliError> {
        // Native init already owns --home as its user/account scan root. Clap
        // propagates matching global ids; it must never become a node data dir.
        let home = if matches!(command, Command::Init(_)) {
            None
        } else {
            self.home
        };
        if home.is_some() && (self.host.is_some() || target.data_dir.is_some()) {
            return Err(CliError::Usage(
                "choose --home separately from --host or --data-dir",
            ));
        }
        if self
            .host
            .as_deref()
            .or(target.node.as_deref())
            .is_some_and(str::is_empty)
        {
            return Err(CliError::Usage("--host must name a node"));
        }
        if let Some(host) = self.host {
            target.node = Some(host);
        }
        if let Some(home) = home {
            target.data_dir = Some(home);
        }
        Ok(())
    }
}
