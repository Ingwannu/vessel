//! `vessel plugin …` (ADR 0115): the node's plugins, as Paseo's
//! `paseo plugin` (`packages/cli/src/commands/plugin/index.ts:211-268`).
//! `--json` (the shared output flag) prints the node's answer as it is.

use std::path::PathBuf;

use clap::Subcommand;
use vessel_proto::plugins::PluginId;

/// What to do with the node's plugins.
#[derive(Debug, Subcommand)]
pub(crate) enum PluginCommand {
    /// List the node's plugins, or show one.
    Ls {
        /// Only this installation.
        id: Option<PluginId>,
    },
    /// Install a plugin and start it when plugins are on (`vessel node
    /// settings --plugins on`). Plugins are trusted code: they run with the
    /// node user's access to files and the network. This node installs from
    /// a directory on it: an absolute path, one starting with `~`, or a
    /// path relative to here (made absolute before it is sent).
    Install {
        /// The plugin's directory (`DIR` or `DIR:SUBPATH`).
        source: String,
        /// The installation id; the manifest's by default.
        #[arg(long)]
        id: Option<PluginId>,
        /// A Git ref (Git sources only).
        #[arg(long = "ref")]
        reference: Option<String>,
    },
    /// Turn an installation on, and start it when plugins are on.
    Enable {
        /// The installation.
        id: PluginId,
    },
    /// Turn an installation off and stop it.
    Disable {
        /// The installation.
        id: PluginId,
    },
    /// Stop an installation and start it again from its files.
    Reload {
        /// The installation.
        id: PluginId,
    },
    /// Stop and forget an installation, with its settings and logs. Its
    /// directory stays on disk.
    Remove {
        /// The installation.
        id: PluginId,
    },
    /// An installation's recent log lines.
    Logs {
        /// The installation.
        id: PluginId,
    },
    /// Write a new plugin into a directory (a theme, as a start). Writes
    /// files only, and downloads nothing.
    Init {
        /// Where; created if missing, refused if it already holds a manifest.
        dir: PathBuf,
        /// The plugin's id; the directory's name by default.
        #[arg(long)]
        id: Option<PluginId>,
    },
}
