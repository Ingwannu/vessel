//! Host/home aliases map onto the existing connection target before effects.

use crate::{args::Target, error::CliError};
use clap::Args;
use std::path::PathBuf;

/// Keep legacy --node/--data-dir together usable while --host/--home conflict.
#[derive(Debug, Default, Args)]
pub(crate) struct ConnectionOptions {
    #[arg(long, global = true, conflicts_with_all = ["home", "node"])]
    host: Option<String>,
    #[arg(long, global = true, conflicts_with_all = ["host", "data_dir"])]
    home: Option<PathBuf>,
}
impl ConnectionOptions {
    pub(crate) fn apply(self, target: &mut Target) -> Result<(), CliError> {
        if let Some(host) = self.host {
            target.node = Some(host);
        }
        if let Some(home) = self.home {
            target.data_dir = Some(home);
        }
        if target.node.as_deref().is_some_and(str::is_empty) {
            return Err(CliError::Usage("--host must name a node"));
        }
        Ok(())
    }
}
