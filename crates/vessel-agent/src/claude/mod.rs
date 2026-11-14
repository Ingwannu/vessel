//! Claude Code's stream-json driver (ADR 0057; `docs/plans/agents-own-protocols.md`,
//! "Claude Code"). Not built yet: [`BUILT`] is false, so
//! `AgentProcess::spawn_with` refuses it before anything starts and
//! [`run`] and [`history`] are not reached.
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

use std::path::Path;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proc::CommandSpec;

use crate::drivers::{Driver, not_built};
use crate::error::AgentError;
use crate::handshake::AgentSetup;
use crate::history::{HistoryError, HistoryFrom, HistoryPage};
use crate::input::Command;
use crate::output::AgentOutput;

/// Whether this driver runs sessions; true once its first slice lands.
pub(crate) const BUILT: bool = false;

/// What this driver's launch leaves to its session: nothing yet. Its
/// first slice gives it what Claude Code takes only on its command line
/// (resume, fork, the permission route, the MCP configuration file it
/// owns for the session's life).
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
    _stdin: ChildStdin,
    _stdout: ChildStdout,
    _commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    _setup: AgentSetup,
    _launched: Launched,
) {
    not_built(&outputs, Driver::ClaudeStream).await;
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
