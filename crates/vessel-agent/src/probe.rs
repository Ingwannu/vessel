//! [`probe`]: what an agent offers before any session of it exists
//! (protocol 9's `harness.options`, ADR 0022).
//!
//! [Decision Log]
//! - Purpose: a new-session form shows the models, efforts and permissions
//!   modes an agent offers before anything runs. Only the agent knows them,
//!   and ACP tells them only in a session's handshake.
//! - Alternatives: a list per harness written into Vessel (stale with
//!   every agent release, and wrong for an account that offers other
//!   models); reading each agent's own configuration files (a parser per
//!   harness, and still not what its adapter offers).
//! - Choice: start the agent exactly as a structured session would, run
//!   the handshake (`initialize`, `session/new`), take the options it
//!   reports, and stop it. No prompt is ever sent, so a probe costs no
//!   model call. The whole probe is bounded by the handshake timeout and a
//!   short stop, so a hung agent cannot hold a caller.
//! - Cost: one agent start per probe (a few seconds); the node caches the
//!   result. `session/new` may leave a trace in the harness's home (Claude
//!   Code makes an empty project folder for the directory); the node
//!   probes in one scratch directory of its own so it is always the same.

use std::time::Duration;

use vessel_proc::{CommandSpec, Signal};
use vessel_proto::{AgentAbilities, AgentEvent, SessionOption};

use crate::drivers::Driver;
use crate::error::{AgentError, Disconnect};
use crate::handshake::AgentSetup;
use crate::introduction::Introduction;
use crate::output::AgentOutput;
use crate::process::AgentProcess;

/// How long the agent may take to exit once the probe has what it needs,
/// before its process group is killed.
const STOP_GRACE: Duration = Duration::from_secs(2);

/// How many of the agent's last stderr lines a failure carries: its reason
/// (not signed in, a missing runtime) is usually there.
const LOG_LINES: usize = 5;

/// Why a probe found nothing.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// The agent could not be started.
    #[error("cannot start the agent")]
    Spawn(#[source] AgentError),
    /// The agent's connection ended before the handshake completed: it
    /// failed (for example not signed in), or the agent exited.
    #[error("{disconnect}{}", Said(log))]
    Disconnected {
        /// How it ended.
        disconnect: Disconnect,
        /// Its last lines on stderr, oldest first.
        log: Vec<String>,
    },
    /// The handshake did not complete within the limit.
    #[error("the agent did not complete the ACP handshake within {} s{}", limit.as_secs(), Said(log))]
    TimedOut {
        /// The limit.
        limit: Duration,
        /// Its last lines on stderr, oldest first.
        log: Vec<String>,
    },
}

/// `; the agent said: a | b`, or nothing.
struct Said<'a>(&'a [String]);

impl std::fmt::Display for Said<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        f.write_str("; the agent said: ")?;
        f.write_str(&self.0.join(" | "))
    }
}

/// What a probe learned from the handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    /// The options the agent offers by default.
    pub options: Vec<SessionOption>,
    /// What it says it can do (protocol 10).
    pub abilities: AgentAbilities,
    /// What it said of itself in `initialize` (protocol 11).
    pub introduction: Introduction,
}

/// How the handshake ended, before the log is attached.
enum Unready {
    Disconnected(Disconnect),
    TimedOut,
}

/// Start `command` with `setup` (its `model`, `resume` and `options`
/// should be empty: a probe asks what the agent offers by default; its
/// `questions` is ignored, a probe never advertises them), take
/// the options and abilities from the handshake and stop the agent.
/// Bounded by `setup.handshake_timeout` plus twice [`STOP_GRACE`].
///
/// # Errors
/// [`ProbeError::Spawn`] when the program cannot start,
/// [`ProbeError::Disconnected`] or [`ProbeError::TimedOut`] when the
/// handshake does not complete.
pub async fn probe(command: &CommandSpec, setup: AgentSetup) -> Result<Probed, ProbeError> {
    probe_with(Driver::Acp, command, setup).await
}

/// [`probe`] over the protocol `driver` speaks (ADR 0057): what the agent
/// offers comes through the same outputs whichever driver runs it.
///
/// # Errors
/// As [`probe`]; [`ProbeError::Spawn`] holds
/// [`AgentError::NoDriver`](crate::AgentError::NoDriver) for a driver this
/// build does not run.
pub async fn probe_with(
    driver: Driver,
    command: &CommandSpec,
    setup: AgentSetup,
) -> Result<Probed, ProbeError> {
    // Never questions, whatever the caller set: nobody watches a probe,
    // and what the agent offers does not depend on it.
    let setup = AgentSetup {
        questions: false,
        ..setup
    };
    let limit = setup.handshake_timeout;
    let (mut agent, _input, mut outputs) =
        AgentProcess::spawn_with(driver, command, setup).map_err(ProbeError::Spawn)?;
    let mut log: Vec<String> = Vec::new();
    let mut options = None;
    let mut introduction = Introduction::default();
    // The handshake has its own deadline (the same `limit`); this one only
    // covers an agent whose outputs stay open after a failure.
    let read = async {
        while let Some(output) = outputs.next().await {
            match output {
                AgentOutput::Options(listed) => options = Some(listed),
                AgentOutput::Introduced(said) => introduction = *said,
                AgentOutput::Event(AgentEvent::Ready { abilities, .. }) => {
                    return Ok(Probed {
                        options: options.take().unwrap_or_default(),
                        abilities: abilities.unwrap_or_default(),
                        introduction: std::mem::take(&mut introduction),
                    });
                }
                AgentOutput::Event(AgentEvent::HarnessLog { line }) => keep(&mut log, line),
                AgentOutput::Event(_) | AgentOutput::Commands(_) | AgentOutput::Usage(_) => {}
                AgentOutput::Disconnected(disconnect) => {
                    return Err(Unready::Disconnected(disconnect));
                }
            }
        }
        Err(Unready::Disconnected(Disconnect::Closed))
    };
    let outcome = tokio::time::timeout(limit, read)
        .await
        .unwrap_or(Err(Unready::TimedOut));
    stop(&mut agent).await;
    if outcome.is_err() {
        // The agent's stderr is read apart from its ACP stream, so the
        // lines it wrote before going away can come after the disconnect.
        // It has stopped now, so they end soon; bounded all the same (a
        // child of its own could hold stderr open).
        let rest = async {
            while let Some(output) = outputs.next().await {
                if let AgentOutput::Event(AgentEvent::HarnessLog { line }) = output {
                    keep(&mut log, line);
                }
            }
        };
        let _drained = tokio::time::timeout(STOP_GRACE, rest).await;
    }
    outcome.map_err(|unready| match unready {
        Unready::Disconnected(disconnect) => ProbeError::Disconnected { disconnect, log },
        Unready::TimedOut => ProbeError::TimedOut { limit, log },
    })
}

/// Keep `line`, the oldest dropped past [`LOG_LINES`].
fn keep(log: &mut Vec<String>, line: String) {
    if log.len() == LOG_LINES {
        let _oldest = log.remove(0);
    }
    log.push(line);
}

/// Ask the agent to exit, then kill its group after [`STOP_GRACE`].
async fn stop(agent: &mut AgentProcess) {
    if let Err(error) = agent.signal(Signal::Terminate) {
        tracing::debug!(pid = agent.pid(), %error, "cannot ask the probed agent to stop");
    }
    if tokio::time::timeout(STOP_GRACE, agent.wait())
        .await
        .is_err()
    {
        if let Err(error) = agent.signal(Signal::Kill) {
            tracing::warn!(pid = agent.pid(), %error, "cannot kill the probed agent");
        }
        let _code = agent.wait().await;
    }
}
