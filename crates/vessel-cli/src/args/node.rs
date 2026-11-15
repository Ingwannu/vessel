//! `vessel node`: start, pair, install, update and stop a node, and its
//! paired devices.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use vessel_proto::HarnessKind;
use vessel_proto::parking::{IdleMinutes, ParkAfter};
use vessel_proto::settings::AgentDriver;

use super::{Stop, ttl_secs};

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
    /// Stop gracefully; live sessions require --when-idle or --now.
    Stop(Stop),
    /// Show the node: version, state (serving or draining), live sessions.
    Status,
    /// Collect a bounded metadata-only report from the selected node.
    Diagnostics {
        /// Emit validated canonical JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Make a single-use link that pairs a new device with the node (needs
    /// the `admin` scope). Prints the link, and a QR code on a terminal.
    Pair {
        /// Name for the device on the node (else it names itself).
        #[arg(long)]
        label: Option<String>,
        /// How long the link works: `90s`, `10m`, `2h` (at most `24h`).
        #[arg(long, default_value = "10m", value_parser = ttl_secs)]
        ttl: u32,
        /// Let the new device read only; it cannot operate sessions or
        /// administer this machine.
        #[arg(long)]
        read_only: bool,
    },
    /// List the node's paired devices, or revoke one.
    Devices {
        #[command(subcommand)]
        command: Option<DevicesCommand>,
    },
    /// Show the node's settings, or change one (needs the `admin` scope).
    /// A change is kept across restarts and applies to live sessions.
    Settings {
        /// How long a conversation may stay idle before its agent's process
        /// is stopped (parked): minutes (1 to 10080), or `never`.
        #[arg(long, value_name = "MINUTES|never", value_parser = park_after)]
        park_after: Option<ParkAfter>,
        /// Which protocol drives an agent's new sessions: `acp`, or
        /// `direct`, its own (Codex, Claude Code and `OpenCode`; refused
        /// until this build offers it). Repeat for several agents.
        #[arg(long, value_name = "AGENT=acp|direct", value_parser = driver)]
        driver: Vec<(HarnessKind, AgentDriver)>,
    },
}

/// `--driver`: an agent's name, `=`, then `acp` or `direct`.
fn driver(text: &str) -> Result<(HarnessKind, AgentDriver), String> {
    let (agent, driver) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not AGENT=acp or AGENT=direct"))?;
    let kind = agent
        .parse::<HarnessKind>()
        .ok()
        .filter(|kind| *kind != HarnessKind::Gemini)
        .ok_or_else(|| format!("`{agent}` is not codex, claude or opencode"))?;
    let driver = match driver.trim() {
        "acp" => AgentDriver::Acp,
        "direct" => AgentDriver::Direct,
        other => return Err(format!("`{other}` is not acp or direct")),
    };
    Ok((kind, driver))
}

/// `--park-after`: minutes, or `never` (`0` and `off` too).
fn park_after(text: &str) -> Result<ParkAfter, String> {
    let text = text.trim();
    if matches!(text, "never" | "off" | "0") {
        return Ok(ParkAfter::Never);
    }
    text.parse::<u16>()
        .ok()
        .and_then(IdleMinutes::new)
        .map(|minutes| ParkAfter::Idle { minutes })
        .ok_or_else(|| format!("`{text}` is not minutes from 1 to 10080, or `never`"))
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
    /// Show this connection's own device id and effective scopes. Works
    /// without Read or Admin; does not list other devices.
    #[command(name = "self")]
    SelfInfo,
    /// Revoke a device: it is refused from now on and its open connections
    /// close. Other devices are not affected.
    Revoke {
        /// The device's id, or a unique prefix of it.
        device: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_after_takes_minutes_or_never() {
        assert_eq!(park_after("never"), Ok(ParkAfter::Never));
        assert_eq!(park_after("0"), Ok(ParkAfter::Never));
        assert_eq!(
            park_after("30"),
            Ok(ParkAfter::Idle {
                minutes: IdleMinutes::new(30).unwrap()
            })
        );
        assert!(park_after("10081").is_err());
        assert!(park_after("soon").is_err());
    }

    #[test]
    fn driver_takes_an_agent_and_a_protocol() {
        assert_eq!(
            driver("codex=direct"),
            Ok((HarnessKind::Codex, AgentDriver::Direct))
        );
        assert_eq!(
            driver("claude=acp"),
            Ok((HarnessKind::ClaudeCode, AgentDriver::Acp))
        );
        assert!(driver("codex").is_err());
        assert!(driver("codex=fast").is_err());
        assert!(driver("vim=acp").is_err());
        assert!(driver("gemini=acp").is_err(), "no protocol of its own");
    }
}
