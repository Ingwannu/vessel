//! Binary-only argument parsing, shared by startup and its help/parse tests.
//!
//! Keep command attributes and documentation here unchanged when moving them:
//! generated CLI help is a public interface even when runtime code is split.
use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf, str::FromStr};
use vessel_proto::RouteUrl;

/// Vessel node daemon.
#[derive(Debug, Parser)]
#[command(
    name = "vesseld",
    version,
    about,
    args_conflicts_with_subcommands = true
)]
pub(super) struct Args {
    /// Data directory (defaults to ~/.vessel).
    #[arg(long, global = true, env = "VESSEL_DATA_DIR")]
    pub(super) data_dir: Option<PathBuf>,
    #[command(flatten)]
    pub(super) run: Run,
    #[command(subcommand)]
    pub(super) command: Option<Command>,
}

/// Running the node (the default).
#[derive(Debug, clap::Args)]
pub(super) struct Run {
    /// Operator-owned LSP roots and trusted executable overrides (JSON).
    #[arg(long)]
    pub(super) lsp_config: Option<PathBuf>,
    /// Saved-source root allowed to Read callers, including the fleet (repeatable).
    #[arg(long = "lsp-root")]
    pub(super) lsp_roots: Vec<PathBuf>,
    /// Trusted root where LSP may run build scripts/proc macros (repeatable).
    #[arg(long = "lsp-full-analysis-root")]
    pub(super) lsp_full_roots: Vec<PathBuf>,
    /// TCP address for the websocket, or `none` to serve only the local
    /// socket (reached through `vesseld attach`, for example over SSH).
    #[arg(long, env = "VESSEL_BIND", default_value = "127.0.0.1:7077")]
    pub(super) bind: Bind,
    /// Node name shown to clients (defaults to the hostname).
    #[arg(long, env = "VESSEL_NAME")]
    pub(super) name: Option<String>,
    /// Seconds a structured session's agent may take to start (the ACP
    /// handshake) before the session fails. Raise it where adapters start
    /// slowly, for example an `npx` adapter downloading on first use.
    #[arg(
        long,
        env = "VESSEL_AGENT_HANDSHAKE_TIMEOUT",
        value_name = "SECONDS",
        default_value_t = vessel_session::DEFAULT_HANDSHAKE_TIMEOUT.as_secs(),
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub(super) agent_handshake_timeout: u64,
    /// Minutes a conversation session may stay idle before its agent's
    /// process is stopped (parked); the next prompt starts it again with
    /// the conversation. 0 never parks; at most 10080 (a week). Without it,
    /// an hour. A value saved from a client (`vessel node settings`) wins.
    #[arg(
        long,
        env = "VESSEL_PARK_AFTER",
        value_name = "MINUTES",
        value_parser = clap::value_parser!(u16).range(0..=10_080)
    )]
    pub(super) park_after: Option<u16>,
    /// A relay to register with, as a `ws://` or `wss://` URL (repeatable,
    /// tried in order). Pairing links name it, so devices reach this node
    /// through it without SSH or an open port.
    #[arg(long = "relay", value_name = "URL")]
    pub(super) relays: Vec<RouteUrl>,
    /// File holding the relays' registration token (needed with
    /// `--relay`).
    #[arg(long, value_name = "PATH", env = "VESSEL_RELAY_TOKEN_FILE")]
    pub(super) relay_token_file: Option<PathBuf>,
    /// An address devices may dial directly, advertised in pairing links
    /// (repeatable): `ws://HOST:PORT`, `wss://...`, or just `HOST:PORT`.
    /// Nothing is guessed: without it, devices go through a relay.
    #[arg(long = "public", value_name = "ADDR", value_parser = public_route)]
    pub(super) public: Vec<RouteUrl>,
    /// Behind a reverse proxy, the header in which it passes the client's
    /// address (`X-Real-IP`), so the direct route's limits are per client.
    /// Believed only from --trusted-proxy, which it needs; the proxy must
    /// overwrite it with one bare address.
    #[arg(long, env = "VESSEL_CLIENT_ADDRESS_HEADER")]
    pub(super) client_address_header: Option<axum::http::HeaderName>,
    /// A reverse proxy whose --client-address-header is believed: an
    /// address or a network in CIDR form (`127.0.0.1`, `10.0.0.0/8`).
    /// Repeatable; comma-separated in the environment.
    #[arg(
        long = "trusted-proxy",
        env = "VESSEL_TRUSTED_PROXY",
        value_delimiter = ','
    )]
    pub(super) trusted_proxies: Vec<vessel_relay::TrustedProxy>,
    /// Started in the background by `vesseld attach`: announce readiness
    /// on stdout and keep running when the SSH session hangs up.
    #[arg(long, hide = true)]
    pub(super) detached: bool,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Bridge stdin and stdout to this machine's node, starting it if none
    /// runs. This is what `vessel` runs over SSH.
    Attach,
    /// Print this binary's version and target triple.
    Version,
    /// Delete installed vesseld binaries that are no longer needed.
    Cleanup {
        /// Newest installed binaries to keep besides the running one.
        #[arg(long, default_value = "1")]
        keep: usize,
    },
    /// Run a relay that nodes register with and clients reach them through.
    /// It speaks plain HTTP: put an HTTPS reverse proxy in front for
    /// `wss://`.
    // Boxed: the relay's flags outgrow every other command's (clippy's
    // `large_enum_variant`, which Windows' larger `PathBuf` tips over).
    Relay(Box<RelayArgs>),
}

/// `vesseld relay`.
#[derive(Debug, clap::Args)]
pub(super) struct RelayArgs {
    /// Address to listen on.
    #[arg(long, env = "VESSEL_RELAY_BIND", default_value = "127.0.0.1:7078")]
    pub(super) bind: SocketAddr,
    /// File holding the registration token nodes must present.
    #[arg(long, env = "VESSEL_RELAY_TOKEN_FILE")]
    pub(super) token_file: PathBuf,
    /// Header in which the reverse proxy passes the client's address
    /// (`X-Real-IP`, `CF-Connecting-IP`), for per-client limits. Believed
    /// only from --trusted-proxy, which it needs; the proxy must overwrite
    /// it.
    #[arg(long, env = "VESSEL_RELAY_CLIENT_ADDRESS_HEADER")]
    pub(super) client_address_header: Option<axum::http::HeaderName>,
    /// A reverse proxy whose --client-address-header is believed: an
    /// address or a network in CIDR form (`127.0.0.1`, `10.0.0.0/8`).
    /// Repeatable; comma-separated in the environment. Anyone else is
    /// counted by their own address.
    #[arg(
        long = "trusted-proxy",
        env = "VESSEL_RELAY_TRUSTED_PROXY",
        value_delimiter = ','
    )]
    pub(super) trusted_proxies: Vec<vessel_relay::TrustedProxy>,
    /// Nodes registered at once; another is refused until one leaves.
    #[arg(long, env = "VESSEL_RELAY_MAX_NODES", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_nodes)]
    pub(super) max_nodes: usize,
    /// Connections from nodes that are still registering, at once.
    #[arg(long, env = "VESSEL_RELAY_MAX_REGISTERING", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_registering)]
    pub(super) max_registering: usize,
    /// Relayed client connections at once, in all. Each holds two sockets
    /// here: keep `max-nodes + 2 x max-clients` under the open-file limit.
    #[arg(long, env = "VESSEL_RELAY_MAX_CLIENTS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_clients)]
    pub(super) max_clients: usize,
    /// Relayed client connections to one node at once.
    #[arg(long, env = "VESSEL_RELAY_MAX_CLIENTS_PER_NODE", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_clients_per_node)]
    pub(super) max_clients_per_node: usize,
    /// Bytes a second through each direction of one relayed connection,
    /// after a burst of one second's worth; 0 for no limit. At least
    /// 64 KiB (one record): a smaller value is raised to it.
    #[arg(long, env = "VESSEL_RELAY_MAX_BYTES_PER_SECOND",
        default_value_t = vessel_relay::Limits::default().max_bytes_per_second)]
    pub(super) max_bytes_per_second: u64,
    /// Connections open at once, of every kind, counted from accept.
    #[arg(long, env = "VESSEL_RELAY_MAX_CONNECTIONS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_connections)]
    pub(super) max_connections: usize,
    /// Of --max-connections, the connections that have not authenticated
    /// yet (registered as a node, or paired as a client): the rest is kept
    /// for those that have. Less than half of --max-connections.
    #[arg(long, env = "VESSEL_RELAY_MAX_UNAUTHENTICATED", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_unauthenticated)]
    pub(super) max_unauthenticated: usize,
    /// Connections not authenticated yet from one address (an IPv6 /64; a
    /// /56 twice this, a /48 four times), counted from accept; not
    /// applied to a trusted proxy's connections. Four times it must be
    /// less than half of --max-unauthenticated.
    #[arg(long, env = "VESSEL_RELAY_MAX_CONNECTIONS_PER_ADDRESS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_connections_per_address)]
    pub(super) max_connections_per_address: usize,
    /// Nodes registering at once from one address (a /48 four times this,
    /// which must be less than half of --max-registering).
    #[arg(long, env = "VESSEL_RELAY_MAX_REGISTERING_PER_ADDRESS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_registering_per_address)]
    pub(super) max_registering_per_address: usize,
    /// Of --max-registering, the places kept for nodes that registered
    /// here before (at most half).
    #[arg(long, env = "VESSEL_RELAY_RESERVED_REGISTERING",
        default_value_t = vessel_relay::Limits::default().reserved_registering)]
    pub(super) reserved_registering: usize,
    /// Relayed client connections at once from one address (a /48 four
    /// times this, which must be less than half of --max-clients).
    #[arg(long, env = "VESSEL_RELAY_MAX_CLIENTS_PER_ADDRESS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_clients_per_address)]
    pub(super) max_clients_per_address: usize,
    /// Relayed client connections to one node at once from one address
    /// while the node admits them (their first seconds); a /48 four times
    /// this, which must be less than half of what a node admits at once
    /// through one relay (`vessel_relay::RELAYED_UNADMITTED`).
    #[arg(long, env = "VESSEL_RELAY_MAX_NODE_CLIENTS_PER_ADDRESS", value_parser = at_least_one,
        default_value_t = vessel_relay::Limits::default().max_node_clients_per_address)]
    pub(super) max_node_clients_per_address: usize,
}

/// A count of at least one: a relay with no room for anything would only
/// refuse.
fn at_least_one(text: &str) -> Result<usize, String> {
    match text.parse::<usize>() {
        Ok(0) => Err("must be at least 1".to_owned()),
        Ok(count) => Ok(count),
        Err(error) => Err(error.to_string()),
    }
}

/// `--bind`: an address, or `none`.
#[derive(Debug, Clone, Copy)]
pub(super) enum Bind {
    Tcp(SocketAddr),
    None,
}

impl FromStr for Bind {
    type Err = std::net::AddrParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.eq_ignore_ascii_case("none") {
            Ok(Self::None)
        } else {
            text.parse().map(Self::Tcp)
        }
    }
}

/// `--public`: a websocket URL, or `HOST:PORT` taken as `ws://HOST:PORT`.
fn public_route(text: &str) -> Result<RouteUrl, String> {
    let text = text.trim();
    let url = if text.contains("://") {
        text.to_owned()
    } else {
        format!("ws://{text}")
    };
    url.parse()
        .map_err(|error: vessel_proto::InvalidRoute| error.to_string())
}

#[cfg(test)]
mod tests;
