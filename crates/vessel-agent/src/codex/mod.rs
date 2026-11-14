//! Codex's app-server driver (ADR 0057; `docs/plans/agents-own-protocols.md`,
//! "Codex"). Slice 1 is built: a session starts a thread or resumes one,
//! its prompts are turns, their messages, reasoning, plans, tool calls
//! and tokens stream as events, and a turn can be steered and cancelled.
//! Every approval is declined until slice 2, so the node does not offer
//! the driver yet (`vessel_harness::direct`). Listing is slice 4's.
//!
//! Layout: `wire` reads and writes the JSON-RPC lines, `protocol` holds
//! the messages as types (checked against `schema/app-server.json` by
//! `contract`), `mapper` turns them into events, `session` runs the loop.
//!
//! [Decision Log]
//! - Purpose: slice 0 fixes where this driver lives and what it is given,
//!   so its slices change this directory, its fake in the testkit and
//!   [`BUILT`], and no file another driver's worker edits.
//! - What it implements: the seam of `crate::drivers`, as the ACP driver
//!   (`crate::driver`) does: [`run`] drives one session over the agent's
//!   process and [`history`] lists its conversations.

mod mapper;
mod protocol;
mod session;
mod wire;

#[cfg(test)]
mod contract;

use std::path::Path;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proc::CommandSpec;

use crate::drivers::Driver;
use crate::error::AgentError;
use crate::handshake::AgentSetup;
use crate::history::{HistoryError, HistoryFrom, HistoryPage};
use crate::input::Command;
use crate::output::AgentOutput;

/// Whether this driver runs sessions: since slice 1. Whether the node
/// offers it is `vessel_harness::direct`'s `selectable`.
pub(crate) const BUILT: bool = true;

/// Drive one session over the agent's stdio until it ends, as
/// `crate::driver::run` does for ACP: apply `commands`, publish `outputs`
/// (its handshake's options and abilities first), end with
/// [`AgentOutput::Disconnected`].
pub(crate) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
) {
    session::run(stdin, stdout, commands, outputs, setup).await;
}

/// One page of the agent's own conversations in `cwd` (all without), from
/// `from`, within `limit`, as `crate::history::history` does for ACP.
#[expect(
    clippy::unused_async,
    reason = "the driver's own is async; this stands in for it until it is built"
)]
pub(crate) async fn history(
    _command: &CommandSpec,
    _cwd: Option<&Path>,
    _from: HistoryFrom,
    _limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    Err(HistoryError::Spawn(AgentError::NoDriver(
        Driver::CodexAppServer,
    )))
}
