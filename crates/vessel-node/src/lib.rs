//! `vesseld`: one node per machine, serving its harness sessions over
//! the protocol (versions 2 and 3).
//!
//! This crate assembles the others; it holds wiring and node policy, not
//! mechanisms:
//!
//! ```text
//!  client --ws /rpc (bearer token)-----------------> server ---+
//!  ssh -> vesseld attach --local socket (token)----> ipc ------+--> vessel-rpc Server
//!                                                                    | Handler
//!                                                     service (routing only), lifecycle
//!                                                                    |
//!            sessions: live runtime sessions + stored ones (ended, handed over)
//!              |  create ---> vessel-session ---durable records---> persist ---> vessel-store
//!              |  subscribe -> follow (ring) + store gap fill ---> Outbound
//!              |  read ------> wait until durable, then the store
//! ```
//!
//! - `config`, `identity`: what to run and who this node is.
//! - `server`, `auth`, `transport`: HTTP (`GET /health`) and the `/rpc`
//!   websocket, token check before the upgrade.
//! - `service`: the `Handler` implementation; routes each call, maps typed
//!   errors to wire errors once.
//! - `sessions`: the node's view of sessions, live and stored.
//! - `persist`: drains each session's durable channel into the store and
//!   publishes how far it is committed; once a log is complete, `sessions`
//!   removes the session from the runtime and the store serves it.
//! - `subscribe`: forwards a session's records to one subscription.
//! - `attachments`: files sent with prompts (protocol 10, ADR 0025):
//!   uploads held by whoever began them, reading one back, a prompt's
//!   references made the node's records, and the loader sessions ask for
//!   a prompt's bodies.
//! - `ipc`: the local socket (or named pipe) `vesseld attach` reaches.
//! - `attach`: that proxy, which also starts the node when none runs.
//! - `lifecycle`: `node.stop` and draining (protocol 3).
//! - `fleet`: fleet sync (ADR 0012): the home publishing `fleet.toml`'s
//!   sources and serving `fleet.*`, members pulling and applying into every
//!   profile's home; `init`, `join`, `leave`, `status`, `restore` for the
//!   CLI.
//! - `cleanup`: `vesseld cleanup`, removing installed binaries nothing
//!   needs (ADR 0008).
//! - `node`, `shutdown`: start (taking the data directory's node lock), and
//!   stop in an order that loses nothing.
//!
//! [Decision Log]
//! - Purpose: the node is the product (ARCHITECTURE "Design rules"); it must
//!   keep every record, survive slow and vanishing clients, and stop
//!   without leaving work half done.
//! - Persistence policy lives here, not in the runtime (ADR 0001): the
//!   runtime hands over records on a bounded channel, and this crate
//!   decides how they are committed and when a reader may rely on them.
//! - "Durable through N": `persist` publishes, per session, the sequence
//!   number below which every record is committed. Gap fills and
//!   `session.read` wait on it, because a record can leave the runtime's
//!   ring before the store has committed it; reading the store without that
//!   wait could skip it.
//! - Shutdown order: stop accepting; end sessions (they publish
//!   `Interrupted`); wait until everything is committed; let connections
//!   flush their last frames; close the store. Subscribers therefore see
//!   the final status, and a restart finds every log complete.

mod added;
mod agents;
pub mod attach;
mod attachments;
mod auth;
mod automation;
mod catalogue;
pub mod cleanup;
mod config;
mod devices;
mod envelope;
mod error;
pub mod fleet;
mod forge;
mod guard;
mod http_tls;
mod identity;
mod install;
mod ipc;
mod label_ingress;
mod lifecycle;
mod limits;
mod lsp;
mod lsp_config;
mod models;
mod node;
mod persist;
mod principal;
mod probes;
mod profiles;
mod push;
mod readiness;
mod relays;
mod secret_copy;
mod secure;
mod server;
mod service;
mod sessions;
mod settings;
mod shutdown;
mod side_questions;
mod subscribe;
mod terminals;
mod tidying;
mod transfers;
mod transport;
mod usage_reports;
mod waits;
mod workspace;

pub use config::{Config, DEFAULT_PARK_AFTER, VesselMcp};

/// This binary's version and target triple, as `vesseld version` prints
/// them: what the remote installer checks (ADR 0008).
pub const VERSION_LINE: &str = concat!(env!("CARGO_PKG_VERSION"), " ", env!("VESSEL_TARGET"));
pub use error::NodeError;
pub use fleet::{FleetConfig, FleetTargets};
pub use forge::ForgeSetup;
pub use lsp::LspRoot;
pub use lsp_config::{LspConfigError, load_lsp_config};
pub use models::ModelsError;
pub use node::Node;
pub use profiles::ProfilesFileError;
pub use shutdown::{StopSignals, signal};
pub use terminals::{KNOWN_CREDENTIALS, Shell, TerminalsConfig, credentials_in};
// The store types named by `Config`'s public fields, so callers can build
// a `Config` without depending on `vessel-store` themselves.
pub use vessel_store::{ByteCount, DataDir, NodeToken, Retention, StoreConfig, StoreError};
/// Local lifecycle commands share the node's persistent lock rather than a PID file.
pub use vessel_store::{KernelLock, KernelProbe, NodeLock};

// The first speech consumer removes staging after its product wiring.
#[cfg(test)]
mod speech_audio;

// The first connection-owned resource wires this into the real context once.
#[cfg(test)]
mod connection_life;
