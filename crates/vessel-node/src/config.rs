//! How to run a node.
#[cfg(target_os = "linux")]
mod checkpoint;

use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::{DEFAULT_PORT, RouteUrl};
use vessel_relay::Keepalive;
use vessel_rpc::ServerConfig;
use vessel_session::{DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY};
use vessel_store::{DataDir, StoreConfig};

use crate::fleet::FleetConfig;

/// How long a structured session may stay idle before its agent is parked
/// (protocol 12, ADR 0038): an hour, as Paseo releases idle agents. Long on
/// purpose: Vessel cannot ask an agent whether it left work running in the
/// background (ACP has no such call), and parking stops its process group.
pub const DEFAULT_PARK_AFTER: Duration = Duration::from_secs(60 * 60);

/// Everything a node needs to start. Build with [`Config::new`] and
/// override fields as needed.
#[derive(Debug, Clone)]
pub struct Config {
    /// Language-server selection and bounded lifecycle. Library defaults find no real tools.
    pub lsp: vessel_lsp::Config,
    /// Destination roots allowed to Read callers; local agents also have their own cwd.
    pub lsp_roots: Vec<crate::LspRoot>,
    /// TCP address for the websocket (`/rpc`). Loopback by default;
    /// anything else is reachable by other machines and logs a warning
    /// naming the token file. `None` serves the local socket only (reached
    /// through `vesseld attach`, for example over SSH; ADR 0007).
    pub bind: Option<SocketAddr>,
    /// Name shown to clients; the hostname when `None`.
    pub name: Option<String>,
    /// Identity, token and event log.
    pub data_dir: DataDir,
    /// The account harnesses run as: the user's home and the harness home
    /// variables (`Config::new` says why it is required).
    pub harness_env: HarnessEnv,
    #[cfg(target_os = "linux")]
    pub(crate) checkpoint_ignores: Result<
        vessel_workspace::checkpoint::IgnoreDefaults,
        vessel_proto::checkpoint::CheckpointError,
    >,
    /// Where harness binaries are looked up (`PATH` by default).
    pub harness_lookup: Lookup,
    /// Event log limits and retention.
    pub store: StoreConfig,
    /// Records each session keeps in memory for live followers.
    pub ring_capacity: NonZeroUsize,
    /// Records a session may hand over before persistence catches up (the
    /// session pauses its output while this is full).
    pub durable_capacity: NonZeroUsize,
    /// Longest a structured session's agent may take to start (the ACP
    /// handshake) before the session fails. 60 s by default; raise it
    /// where adapters start slowly (`vesseld --agent-handshake-timeout`).
    pub agent_handshake_timeout: Duration,
    /// How long a structured session may stay idle before its agent's
    /// process is parked, to be started again by the next prompt (protocol
    /// 12, ADR 0038; `vesseld --park-after`). An hour by default (Paseo's);
    /// `None` never parks. A value a person saved through
    /// `node.set_settings` (`settings.json`) wins over it at start.
    pub park_after: Option<Duration>,
    /// Whether [`park_after`](Self::park_after) was given (`--park-after`),
    /// not the default: a saved value then overriding it is logged.
    pub park_after_given: bool,
    /// Per-connection RPC limits.
    pub rpc: ServerConfig,
    /// Relays this node registers with, in preference order (`--relay`);
    /// its pairing links name them (ADR 0009). Needs
    /// [`relay_token_file`](Self::relay_token_file).
    pub relays: Vec<RouteUrl>,
    /// The file holding the relays' registration token
    /// (`--relay-token-file`): a secret shared with the nodes that may use
    /// them (plan decision 4).
    pub relay_token_file: Option<PathBuf>,
    /// Direct addresses pairing links advertise (`--public`). Nothing is
    /// guessed: a node behind NAT has none, and is reached through a relay.
    pub public: Vec<RouteUrl>,
    /// How the node keeps its own `/secure` connections alive: a ping every
    /// 10 s, and a connection silent for 30 s is dropped, as a relay does
    /// for relayed ones. Clients give up after the same silence.
    pub keepalive: Keepalive,
    /// The command that runs `vessel mcp`, attached to every session as its
    /// `vessel` MCP server so the session's agent can reach this node with
    /// a credential of its own (ADR 0013). `None` (the default): sessions
    /// get neither. `vessel node start` sets its own executable; `vesseld`
    /// a `vessel` beside it.
    pub vessel_mcp: Option<VesselMcp>,
    /// Fleet sync (ADR 0012): where bundles are written and how often the
    /// node looks; the defaults suit a real node.
    pub fleet: FleetConfig,
    /// Behind a reverse proxy every direct client comes from the proxy:
    /// the header in which it passes the client's address (`X-Real-IP`),
    /// so `/secure`'s per-address budget is per client
    /// (`vesseld --client-address-header`). Believed only from
    /// [`trusted_proxies`](Self::trusted_proxies), which it needs.
    pub client_address_header: Option<axum::http::HeaderName>,
    /// The peers whose [`client_address_header`](Self::client_address_header)
    /// is believed (`vesseld --trusted-proxy`); anyone else is counted by
    /// its own address.
    pub trusted_proxies: Vec<vessel_relay::TrustedProxy>,
    /// How the node reaches forges for pull requests (protocol 11, ADR
    /// 0029): the tokens and the `gh` the binary found, or a test's
    /// in-memory forge. The default finds neither, so a test's node never
    /// reaches a real forge.
    pub forge: crate::ForgeSetup,
    /// Terminals beside sessions (protocol 11, ADR 0031): the shell, the
    /// credentials removed from their environment, their scrollback. No
    /// shell by default, so `terminal.create` is `unsupported` until a
    /// binary finds one (`TerminalsConfig::shell`); scripts run anyway.
    pub terminals: crate::TerminalsConfig,
    /// Where `agent.catalogue` fetches the ACP registry (protocol 11, ADR
    /// 0032): its published index. https only, or plain http to this
    /// machine (what a test serves).
    pub registry_url: String,
    /// Unit-test transport injection; never exists in an installed node.
    #[cfg(test)]
    pub(crate) push_test_origin: Option<url::Url>,
    #[cfg(test)]
    pub(crate) push_preparation: Option<std::sync::Arc<crate::push::Preparation>>,
    /// Explicit quota backend; the library default accesses no saved sign-in.
    pub limits: vessel_harness::LimitProbe,
    /// Nonsecret inherited route facts captured only by the binaries.
    pub limit_environment: vessel_harness::LimitEnvironment,
}

/// A command that runs `vessel mcp` ([`Config::vessel_mcp`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselMcp {
    /// The program, an absolute path.
    pub program: PathBuf,
    /// Its arguments (`["mcp"]` for `vessel`).
    pub args: Vec<String>,
}

impl VesselMcp {
    /// `program mcp`.
    #[must_use]
    pub fn vessel(program: PathBuf) -> Self {
        Self {
            program,
            args: vec!["mcp".to_owned()],
        }
    }
}

impl Config {
    /// Defaults for a node keeping its state in `data_dir`, running
    /// harnesses as `harness_env`'s account.
    ///
    /// `harness_env` has no default on purpose: `vesseld` and the CLI make
    /// it from their own environment (`HarnessEnv::from_vars`), and a test
    /// gives a temporary home (`HarnessEnv::new`), so no in-process node
    /// reaches the machine's real harness homes by an argument forgotten.
    #[must_use]
    pub fn new(data_dir: DataDir, harness_env: HarnessEnv) -> Self {
        Self {
            lsp: vessel_lsp::Config {
                registry: vessel_lsp::registry::Registry {
                    home: Some(harness_env.user_home().to_owned()),
                    // Language servers run in an empty directory of the
                    // node's own, never in a repository, and keep their
                    // temporary files there instead of in the account home.
                    state_dir: Some(data_dir.root().join("lsp")),
                    ..vessel_lsp::registry::Registry::default()
                },
                ..vessel_lsp::Config::default()
            },
            lsp_roots: Vec::new(),
            bind: Some(SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_PORT))),
            name: None,
            data_dir,
            #[cfg(target_os = "linux")]
            checkpoint_ignores: checkpoint::defaults(harness_env.user_home(), None),
            harness_env,
            harness_lookup: Lookup::from_env(),
            store: StoreConfig::default(),
            ring_capacity: DEFAULT_RING_CAPACITY,
            durable_capacity: DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: Some(DEFAULT_PARK_AFTER),
            park_after_given: false,
            rpc: ServerConfig::default(),
            relays: Vec::new(),
            relay_token_file: None,
            public: Vec::new(),
            keepalive: Keepalive::DEFAULT,
            vessel_mcp: None,
            fleet: FleetConfig::default(),
            client_address_header: None,
            trusted_proxies: Vec::new(),
            forge: crate::ForgeSetup::default(),
            terminals: crate::TerminalsConfig::default(),
            registry_url: crate::catalogue::REGISTRY_URL.to_owned(),
            #[cfg(test)]
            push_test_origin: None,
            #[cfg(test)]
            push_preparation: None,
            limits: vessel_harness::LimitProbe::Disabled,
            limit_environment: vessel_harness::LimitEnvironment::new(),
        }
    }

    /// This machine's saved sign-ins as the quota backend, with the
    /// nonsecret route facts `var` reads (ADR 0042). Only a binary calls
    /// this, with its own process environment: [`Config::new`] probes
    /// nothing, so no in-process or test node reaches the account's
    /// sign-ins by a default. It lives here, beside the rest of the node's
    /// configuration, rather than as field assignments in `vesseld`'s
    /// `run`, which holds no rule of its own about limits.
    #[must_use]
    pub fn with_local_limits(mut self, var: impl Fn(&str) -> Option<OsString>) -> Self {
        self.limits = vessel_harness::LimitProbe::Local;
        self.limit_environment = vessel_harness::LimitEnvironment::from_vars(var);
        self
    }
}
