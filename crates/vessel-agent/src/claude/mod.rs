//! Claude Code's stream-json driver (ADR 0057; `docs/plans/agents-own-protocols.md`,
//! "Claude Code"). Slice 1 is written: a launch names the conversation
//! (new or resumed), the permission route and mode, the model and the MCP
//! file; a session opens the control protocol, its prompts are turns whose
//! text, reasoning, tool calls and tokens stream as events, and a turn can
//! be cancelled. Every tool request is denied until slice 2, so [`BUILT`]
//! stays false: `AgentProcess::spawn_with` refuses the driver before
//! anything starts, and this module's tests drive [`launch`] and [`run`]
//! over the fake CLI directly.
//!
//! Layout: `wire` spells the frames, `io` reads and writes them a line
//! each, `map` turns them into the timeline, `launch` shapes the command,
//! `session` runs the loop.
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

use crate::drivers::Driver;
use crate::error::AgentError;
use crate::handshake::AgentSetup;
use crate::history::{HistoryError, HistoryFrom, HistoryPage};
use crate::input::Command;
use crate::output::AgentOutput;

pub(crate) use launch::{Launched, launch};

/// Whether this driver runs sessions. False until its permissions slice:
/// slice 1 runs over the fake in this module's tests, and a driver that
/// cannot ask a person must never run a session (the coordinator's rule,
/// stricter than `vessel_harness::direct`'s `selectable` alone).
pub(crate) const BUILT: bool = false;

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
        Driver::ClaudeStream,
    )))
}

/// Pure native input validation, shared with identified-prompt preflight.
pub(crate) fn validate_prompt(
    content: &[vessel_proto::ContentBlock],
) -> Result<(), crate::InputError> {
    session::validate_prompt(content)
}
