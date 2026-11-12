//! `vessel node`: start, pair, install, update and stop a node, and its
//! paired devices.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use super::ttl_secs;

#[derive(Debug, Subcommand)]
pub(crate) enum NodeCommand {
    /// Start a node and serve until Ctrl-C.
    Start {
        /// Operator-owned LSP roots and executable overrides (JSON).
        #[arg(long)]
        lsp_config: Option<PathBuf>,
        /// Saved-source root allowed to Read callers, including the fleet (repeatable).
        #[arg(long = "lsp-root")]
        lsp_roots: Vec<PathBuf>,
        /// Trusted root where LSP may run build scripts/proc macros (repeatable).
        #[arg(long = "lsp-full-analysis-root")]
        lsp_full_roots: Vec<PathBuf>,
        /// Address to listen on.
        #[arg(long, env = "VESSEL_BIND", default_value = "127.0.0.1:7077")]
        bind: SocketAddr,
        /// Node name shown to clients (defaults to the hostname).
        #[arg(long, env = "VESSEL_NAME")]
        name: Option<String>,
    },
    /// Install this client's vesseld on an SSH node (`--node`, or the
    /// default node). Nothing is restarted: a running node keeps its
    /// version until `vessel node update`.
    Install(Install),
    /// Install this client's vesseld on an SSH node and restart the node
    /// into it, as its live sessions allow.
    Update(Update),
    /// Show the node: version, state (serving or draining), live sessions.
    Status,
    /// Make a single-use link that pairs a new device with the node (needs
    /// the `admin` scope). Prints the link, and a QR code on a terminal.
    Pair {
        /// Name for the device on the node (else it names itself).
        #[arg(long)]
        label: Option<String>,
        /// How long the link works: `90s`, `10m`, `2h` (at most `24h`).
        #[arg(long, default_value = "10m", value_parser = ttl_secs)]
        ttl: u32,
    },
    /// List the node's paired devices, or revoke one.
    Devices {
        #[command(subcommand)]
        command: Option<DevicesCommand>,
    },
}

/// `vessel node update`.
#[derive(Debug, Args)]
pub(crate) struct Update {
    /// With live sessions: stop once the last one has ended (the node
    /// refuses new sessions meanwhile).
    #[arg(long, conflicts_with = "now")]
    pub(crate) when_idle: bool,
    /// With live sessions: stop now; they end interrupted (asks first).
    #[arg(long)]
    pub(crate) now: bool,
    /// Do not ask before `--now` interrupts sessions.
    #[arg(long, requires = "now")]
    pub(crate) yes: bool,
    /// Where vesseld comes from (see `vessel node install`).
    #[arg(long, value_enum, default_value_t = Source::Auto)]
    pub(crate) from: Source,
    /// The local vesseld build to upload (implies `--from local`).
    #[arg(long)]
    pub(crate) vesseld: Option<PathBuf>,
}

/// `vessel node install`.
#[derive(Debug, Args)]
pub(crate) struct Install {
    /// Where vesseld comes from: `local` uploads a vesseld built here (the
    /// one next to `vessel`, or `--vesseld`), `release` this version's
    /// release (downloaded with `gh`), `auto` a local build if there is one
    /// for that machine, else the release.
    #[arg(long, value_enum, default_value_t = Source::Auto)]
    pub(crate) from: Source,
    /// The local vesseld build to upload (implies `--from local`).
    #[arg(long)]
    pub(crate) vesseld: Option<PathBuf>,
}

/// Where an installed vesseld comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Source {
    Auto,
    Local,
    Release,
}

/// `vessel node devices ...`.
#[derive(Debug, Subcommand)]
pub(crate) enum DevicesCommand {
    /// Revoke a device: it is refused from now on and its open connections
    /// close. Other devices are not affected.
    Revoke {
        /// The device's id, or a unique prefix of it.
        device: String,
    },
}
