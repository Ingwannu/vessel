//! Claude Code's stream-json driver (ADR 0057; `docs/plans/agents-own-protocols.md`,
//! "Claude Code"). Slices 1 and 2 are written: a launch names the
//! conversation (new or resumed), the permission route and mode, the model
//! and the MCP file; a session opens the control protocol, its prompts are
//! turns whose text, reasoning, tool calls and tokens stream as events, a
//! turn can be cancelled, and every tool request is asked of a person (a
//! permission request, a plan's approval, a question). So [`BUILT`] is
//! true and `vessel_harness::direct` lets the node select it.
//!
//! Layout: `wire` spells the frames, `io` reads and writes them a line
//! each, `map` turns them into the timeline, `launch` shapes the command,
//! `asks` holds what the agent asks a person, `session` runs the loop.
//!
//! [Decision Log]
//! - Purpose: slice 0 fixes where this driver lives and what it is given,
//!   so its slices change this directory, its fake in the testkit and
//!   [`BUILT`], and no file another driver's worker edits.
//! - What it implements: the seam of `crate::drivers`, as the ACP driver
//!   (`crate::driver`) does: [`launch`] shapes the agent's command for one
//!   launch before it is spawned, [`run`] drives one session over the
//!   agent's process and [`history`] lists its conversations (spawning its
//!   own process, so a listing shapes its own command there).

mod asks;
mod io;
mod launch;
mod map;
mod session;
mod wire;

#[cfg(test)]
mod contract;

use std::path::Path;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proc::CommandSpec;

use crate::handshake::AgentSetup;
use crate::history::{HistoryError, HistoryFrom, HistoryPage};
use crate::input::Command;
use crate::output::AgentOutput;

pub(crate) use launch::{Launched, launch};

/// Whether this driver runs sessions: true since its permissions slice
/// (slice 2), the coordinator's rule that a driver which cannot ask a
/// person must never run a session.
pub(crate) const BUILT: bool = true;

/// Drive one session over the agent's stdio until it ends, as
/// `crate::driver::run` does for ACP: apply `commands`, publish `outputs`
/// (its handshake's options and abilities first), end with
/// [`AgentOutput::Disconnected`]. `launched` lives as long as the session.
pub(crate) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
    launched: Launched,
) {
    session::run(stdin, stdout, commands, outputs, setup, launched).await;
}

/// One page of the agent's own conversations: not listed until slice 4
/// (`unsupported`), as an ACP agent without `session/list` cannot, and as
/// Codex's driver answers until its own slice 4.
///
/// # Errors
/// [`HistoryError::CannotList`], until slice 4.
#[expect(
    clippy::unused_async,
    reason = "slice 4's listing is async; this stands in for it until then"
)]
pub(crate) async fn history(
    _command: &CommandSpec,
    _cwd: Option<&Path>,
    _from: HistoryFrom,
    _limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    Err(HistoryError::CannotList)
}
