//! Local author scaffolding; node management is registered only with its service.
use clap::Subcommand;
use std::path::PathBuf;
use vessel_proto::plugins::PluginId;

#[derive(Debug, Subcommand)]
pub(crate) enum PluginCommand {
    /// Create a native declarative plugin locally, without installing or running it.
    Init {
        /// New or empty local directory. Existing files are never overwritten.
        directory: PathBuf,
        /// Manifest identity; defaults to the directory's final component.
        #[arg(long)]
        id: Option<PluginId>,
    },
}
