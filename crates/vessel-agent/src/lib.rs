//! Vessel's ACP client: structured sessions.
//!
//! Drives a harness (or its adapter) over the Agent Client Protocol on
//! stdio and turns what it reports into Vessel's own timeline
//! ([`vessel_proto::AgentEvent`]). ACP types never leave this crate (ADR
//! 0003): an ACP version bump changes this crate, not the wire format.
//!
//! Modules, by responsibility:
//!
//! - `process`: [`AgentProcess`], spawning through `vessel-proc` (own
//!   process group) and starting the tasks below;
//! - `driver`: the ACP connection and the session state machine
//!   (handshake, prompt, cancel, permission round trip);
//! - `transport`: the agent's stdio as the SDK's line transport, holding
//!   lines back while the driver is behind (backpressure);
//! - `handshake`: `initialize`, `session/new` or `session/load`, model,
//!   option choices;
//! - `mcp`: Vessel's MCP server definitions as ACP's, for those requests;
//! - `model`: picking the requested model among ACP config options;
//! - `options`: the agent's config options (and ACP's older modes), shown
//!   as Vessel's and changed by a client's choice (protocol 9);
//! - `probe`: asking an agent what it offers without a session (protocol
//!   9's `harness.options`);
//! - `permissions`: pending permission requests, first answer wins;
//! - `mapper`: pure ACP-to-Vessel conversions, tested with fixtures;
//! - `stderr`: harness stderr lines as `HarnessLog` events;
//! - `input`, `output`: the public command and event ends.
//!
//! Client capabilities are empty: no `fs`, no `terminal`. Agents use their
//! own tools, as they do in a terminal, and Vessel does not re-implement
//! them.
//!
//! [Decision Log]
//! - Purpose: one client for every structured harness (ADR 0003).
//! - Alternatives: the SDK's `ActiveSession` helper (it buffers updates in
//!   an unbounded channel and hides permission requests behind its own
//!   flow); the SDK's `process` feature (`AcpAgent` spawns without a
//!   process group, so a harness's MCP servers would outlive it).
//! - Choice: raw `Client.v1()` handlers that forward into one bounded
//!   channel consumed by a single driver loop (see `driver`), over the
//!   child's piped stdio from `vessel-proc`.
//! - Cost: the session state machine is written here (~300 lines), not
//!   borrowed from the SDK.

mod abilities;
mod driver;
mod error;
mod handshake;
mod input;
mod mapper;
mod mcp;
mod model;
mod options;
mod output;
mod permissions;
mod probe;
mod process;
mod recent;
mod stderr;
mod transport;

pub use error::{AgentError, Disconnect, InputError};
pub use handshake::{AgentSetup, DEFAULT_HANDSHAKE_TIMEOUT};
pub use input::AgentInput;
pub use options::{KindName, OptionError};
pub use output::{AgentOutput, AgentOutputs};
pub use probe::{ProbeError, Probed, probe};
pub use process::AgentProcess;
/// The command [`AgentProcess::spawn`] runs, and the signals it takes;
/// re-exported so callers need no edge to `vessel-proc`.
pub use vessel_proc::{CommandSpec, Signal};
