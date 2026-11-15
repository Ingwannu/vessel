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
//! - `drivers`: which protocol drives an agent ([`Driver`], ADR 0057); ACP
//!   is the only one built so far;
//! - `driver`: the ACP connection and the session state machine
//!   (handshake, prompt, cancel, permission and question round trips);
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
//! - `history`: a page of the agent's own conversations, over `session/list`
//!   (protocol 17's `agent.history`);
//! - `permissions`: pending permission requests, first answer wins;
//! - `questions`: pending questions (ACP form elicitations, protocol 10),
//!   first answer wins;
//! - `mapper`: pure ACP-to-Vessel conversions, tested with fixtures;
//! - `stderr`: harness stderr lines as `HarnessLog` events;
//! - `input`, `output`: the public command and event ends.
//!
//! Client capabilities are small: no `fs`, no `terminal`. Agents use their
//! own tools, as they do in a terminal, and Vessel does not re-implement
//! them. Form elicitation (questions) only for a session that asks for it
//! ([`AgentSetup::questions`]); never URL elicitation.
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
mod attached;
mod claude;
mod codex;
mod context;
mod driver;
mod drivers;
mod error;
mod handshake;
mod history;
mod input;
mod introduction;
mod mapper;
mod mcp;
mod model;
mod opencode;
mod options;
mod output;
mod permissions;
mod probe;
mod process;
mod questions;
mod recent;
mod setup;
mod stderr;
mod system_prompt;
mod transport;

pub use attached::{AttachmentBody, Attachments};
pub use context::PromptContext;
pub use drivers::Driver;
pub use error::{AgentError, Disconnect, InputError, LaunchRefusal};
pub use handshake::{AgentSetup, DEFAULT_HANDSHAKE_TIMEOUT, ForkFrom, ForkRefusal};
pub use history::{
    HistoryEntry, HistoryError, HistoryFrom, HistoryPage, MAX_ENTRIES as HISTORY_MAX_ENTRIES,
    NotACursor, history, history_with,
};
pub use input::{AgentInput, SteerOutcome};
pub use introduction::{Introduction, Refusal, Refused, SPEAKS};
pub use options::{KindName, OptionError};
pub use output::{AgentOutput, AgentOutputs};
pub use probe::{ProbeError, Probed, probe, probe_with};
pub use process::AgentProcess;
pub use system_prompt::SystemPromptOverride;
/// The command [`AgentProcess::spawn`] runs, and the signals it takes;
/// re-exported so callers need no edge to `vessel-proc`.
pub use vessel_proc::{CommandSpec, Signal};

/// The most permission requests, and apart the most questions, an agent
/// session keeps waiting (protocol 10's bound). A newer one withdraws the
/// oldest, answered `cancelled`; the session's projection of what waits
/// keeps the same number, so the two drop the same one.
pub const MAX_WAITING: usize = 64;
