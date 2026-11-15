//! Codex's app-server driver (ADR 0057; `docs/plans/agents-own-protocols.md`,
//! "Codex"). Slices 1 and 2 are built: a session starts a thread or
//! resumes one, its prompts are turns, their messages, reasoning, plans,
//! tool calls and tokens stream as events, a turn can be steered and
//! cancelled, Codex's approvals are permission requests and its questions
//! are questions, and the session's mode decides what it asks. The node
//! may select the driver from slice 2 (`vessel_harness::direct`). Listing
//! uses the same prompt-free transport as native catalogue discovery.
//!
//! Layout: `wire` reads and writes the JSON-RPC lines, `protocol` holds
//! the messages as types (checked against `schema/app-server.json` by
//! `contract`), `mapper` turns them into events, `mode` is the session's
//! mode option, `asks` holds what waits on a person, `session` runs the
//! loop.
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
mod history;
mod mapper;
mod mode;
mod protocol;
mod read_probe;
mod session;
mod wire;

#[cfg(test)]
mod contract;

use std::path::Path;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proc::CommandSpec;

use crate::error::AgentError;
use crate::handshake::AgentSetup;
use crate::history::{HistoryError, HistoryFrom, HistoryPage};
use crate::input::Command;
use crate::output::AgentOutput;

/// Whether this driver runs sessions: since slice 1. Whether the node
/// offers it is `vessel_harness::direct`'s `selectable`.
pub(crate) const BUILT: bool = true;

/// What this driver's launch leaves to its session: nothing (the
/// app-server takes a thread's start, resume and MCP servers over its
/// protocol).
#[derive(Debug, Default)]
pub(crate) struct Launched;

/// `command` shaped for one launch from `setup`, before it is spawned
/// (`AgentProcess::spawn_with`), and what the session keeps of it: the
/// identity for now.
///
/// # Errors
/// None yet; a launch this driver cannot make is refused here, before the
/// agent starts.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the seam every direct driver implements; this one refuses nothing yet"
)]
pub(crate) fn launch(
    command: &CommandSpec,
    _setup: &AgentSetup,
) -> Result<(CommandSpec, Launched), AgentError> {
    Ok((command.clone(), Launched))
}

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
    _launched: Launched,
) {
    session::run(stdin, stdout, commands, outputs, setup).await;
}

/// One page of native conversations, using prompt-free initialize/thread/list.
/// # Errors
/// Spawn, timeout, unsupported listing or malformed bounded native metadata.
pub(crate) async fn history(
    command: &CommandSpec,
    cwd: Option<&Path>,
    from: HistoryFrom,
    limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    history::list(command, cwd, from, limit).await
}
