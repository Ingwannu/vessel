//! [`AgentProcess`]: an ACP agent process and the tasks that serve it.

use tokio::sync::mpsc;
use vessel_proc::{CommandSpec, ProcessLifetime, Signal, SignalError, Spawned};

use crate::driver;
use crate::drivers::Driver;
use crate::error::AgentError;
use crate::handshake::AgentSetup;
use crate::input::AgentInput;
use crate::output::{AgentOutput, AgentOutputs};
use crate::stderr;

/// Outputs buffered between the driver and the consumer.
const OUTPUTS: usize = 64;

/// Commands queued for the driver. Small: each caller waits for its reply
/// anyway, so a deep queue would only hide a stuck agent.
const COMMANDS: usize = 16;

/// One launch as its driver shaped it: what each direct driver keeps for
/// the session's life (ADR 0057; its `launch`), handed to its `run`.
enum Launch {
    Acp,
    Codex(crate::codex::Launched),
    Claude(crate::claude::Launched),
    Opencode(crate::opencode::Launched),
}

/// A running ACP agent (a harness or its adapter).
///
/// Owned by one task (the session actor), which stops it and waits for it.
/// The ACP connection runs in a driver task fed by the process's stdio;
/// stderr lines become `HarnessLog` events. Dropping an `AgentProcess`
/// whose process is still running kills its whole process group, which
/// also ends the driver.
#[derive(Debug)]
pub struct AgentProcess {
    child: vessel_proc::Child,
}

impl AgentProcess {
    /// Start `command` as the leader of a new process group and begin the
    /// ACP handshake described by `setup`.
    ///
    /// Must be called inside a Tokio runtime: the driver and the stderr
    /// pump are spawned onto it.
    ///
    /// # Errors
    /// Fails if the program cannot be started. Handshake failures arrive
    /// later, as [`AgentOutput::Disconnected`].
    pub fn spawn(
        command: &CommandSpec,
        setup: AgentSetup,
    ) -> Result<(Self, AgentInput, AgentOutputs), AgentError> {
        Self::spawn_with(Driver::Acp, command, setup)
    }

    /// [`spawn`](Self::spawn) over the protocol `driver` speaks (ADR 0057):
    /// `command` must be the one for it (an ACP launch, or
    /// `vessel_harness::launch_direct`).
    ///
    /// # Errors
    /// As [`spawn`](Self::spawn), and [`AgentError::NoDriver`] for a driver
    /// this build does not run ([`Driver::built`]), before anything starts.
    pub fn spawn_with(
        driver: Driver,
        command: &CommandSpec,
        setup: AgentSetup,
    ) -> Result<(Self, AgentInput, AgentOutputs), AgentError> {
        if setup.system_prompt.is_some()
            && driver.system_prompt_support() == vessel_harness::SystemPromptSupport::Unsupported
        {
            return Err(crate::LaunchRefusal::SystemPromptUnsupported(driver).into());
        }
        if !driver.built() {
            return Err(AgentError::NoDriver(driver));
        }
        // A direct driver shapes its own launch before anything starts:
        // what its agent takes only at launch (Claude Code's resume, fork,
        // permission route and MCP file; OpenCode's server password) comes
        // from `setup` here, and a launch it cannot make is refused here.
        // A driver that needs this step adds its arm to `Launch`; nothing
        // else in this file changes (`docs/plans/agents-own-protocols.md`,
        // "Frozen by slice 0").
        let (shaped, launch) = match driver {
            Driver::Acp => (None, Launch::Acp),
            Driver::CodexAppServer => {
                let (shaped, launched) = crate::codex::launch(command, &setup)?;
                (Some(shaped), Launch::Codex(launched))
            }
            Driver::ClaudeStream => {
                let (shaped, launched) = crate::claude::launch(command, &setup)?;
                (Some(shaped), Launch::Claude(launched))
            }
            Driver::OpencodeHttp => {
                let (shaped, launched) = crate::opencode::launch(command, &setup)?;
                (Some(shaped), Launch::Opencode(launched))
            }
        };
        let command = shaped.as_ref().unwrap_or(command);
        let Spawned {
            child,
            stdin,
            stdout,
            stderr,
        } = vessel_proc::spawn(command)?;
        let (outputs, receiver) = mpsc::channel::<AgentOutput>(OUTPUTS);
        let (commands, command_receiver) = mpsc::channel(COMMANDS);
        drop(tokio::spawn(stderr::pump(stderr, outputs.clone())));
        let commands_in = command_receiver;
        drop(match launch {
            Launch::Acp => tokio::spawn(driver::run(stdin, stdout, commands_in, outputs, setup)),
            Launch::Codex(launched) => tokio::spawn(crate::codex::run(
                stdin,
                stdout,
                commands_in,
                outputs,
                setup,
                launched,
            )),
            Launch::Claude(launched) => tokio::spawn(crate::claude::run(
                stdin,
                stdout,
                commands_in,
                outputs,
                setup,
                launched,
            )),
            Launch::Opencode(launched) => tokio::spawn(crate::opencode::run(
                stdin,
                stdout,
                commands_in,
                outputs,
                setup,
                launched,
            )),
        });
        tracing::debug!(pid = child.pid(), program = %command.program.display(), %driver, "agent spawned");
        Ok((
            Self { child },
            AgentInput::new(commands),
            AgentOutputs::new(receiver),
        ))
    }

    /// The agent's process id (on Unix also its process group id).
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.child.pid()
    }

    /// Pass the actual child observation through without adding driver policy.
    #[must_use]
    pub fn lifetime(&self) -> ProcessLifetime {
        self.child.lifetime()
    }

    /// Ask the agent's process group to stop ([`Signal::Terminate`] is the
    /// polite choice). Does nothing once it has exited.
    ///
    /// # Errors
    /// Fails if the OS refuses to deliver the signal.
    pub fn signal(&mut self, signal: Signal) -> Result<(), SignalError> {
        self.child.signal(signal)
    }

    /// Wait for the agent to exit; returns its exit code, or `None` when it
    /// was ended by a signal or its status is unknown. The rest of its
    /// group is killed once it exits. Cancel-safe.
    pub async fn wait(&mut self) -> Option<i32> {
        match self.child.wait().await {
            Ok(code) => code,
            Err(error) => {
                tracing::warn!(pid = self.child.pid(), %error, "cannot wait for agent");
                None
            }
        }
    }
}
