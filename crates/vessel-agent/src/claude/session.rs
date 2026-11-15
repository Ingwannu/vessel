//! One session over Claude Code's stream-json: the control handshake,
//! turns, cancelling, and what the agent asks (slice 1 of the plan's
//! Claude Code section).
//!
//! [Decision Log]
//! - Purpose: the session actors see what the ACP driver gives them, in
//!   the same order: `Introduced`, `Options`, `Ready`; a prompt answered
//!   once sent, with `TurnStarted`; the turn's events; its `TurnEnded` or
//!   `TurnFailed`; `Disconnected` last.
//! - Handshake: the control protocol's `initialize`, the Agent SDK's own
//!   first request, answered before any prompt, within the setup's
//!   handshake time. The session is ready with the conversation its
//!   launch named (`Launched`): Claude Code writes its `init` frame only
//!   once a turn starts. A CLI that does not answer `initialize` fails the
//!   handshake with that reason; the fallback to the documented MCP
//!   permission tool for exactly that case is slice 2's.
//! - One loop owns the state and never awaits an answer in place; the
//!   agent's frames and the client's commands arrive through channels.
//!   Commands are polled first, as in the ACP driver: the agent can stream
//!   faster than the loop takes frames, and a cancel must not wait for
//!   the stream to pause. Taking a command before frames already queued is
//!   safe: a turn ends only at its `result`, handled in order with its
//!   frames.
//! - The floor (ADR 0057): every tool request is answered (denied, until
//!   the permissions slice turns them into permission requests, with a
//!   line in the session's log); a control request the driver does not
//!   serve or cannot read is refused and the running turn is stopped and
//!   ends failed, saying why. Nothing is ever allowed by default, and
//!   nothing the agent asks is left waiting.
//! - Layout: this file holds the loop, its state and its outputs; `agent`
//!   handles the agent's traffic (frames, its requests, its answers),
//!   `commands` the client's commands and the driver's own requests, so
//!   each file has one side of the conversation.
//! - A turn the driver ends itself (a refusal) still ends at its `result`
//!   (the CLI writes one after the interrupt), so the next prompt never
//!   meets the previous turn's end; the failure replaces what the result
//!   would have said.

use std::collections::HashSet;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proto::{AgentAbilities, AgentEvent, HarnessSessionId};

use super::io::{self, Ended, Read, Writer};
use super::launch::Launched;
use super::map::Translator;
use super::wire::HostRequest;
use crate::error::Disconnect;
use crate::handshake::AgentSetup;
use crate::input::Command;
use crate::output::AgentOutput;

mod agent;
mod commands;

/// Frames read ahead of the loop.
const INBOUND: usize = 64;

/// What the session can do over this driver, as its handshake says.
const ABILITIES: AgentAbilities = AgentAbilities {
    steer: false,
    resume: true,
    image: false,
    audio: false,
    fork: false,
};

/// Where the session's turn is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Turn {
    Idle,
    Running {
        /// The client asked to stop it.
        cancelled: bool,
        /// The driver stopped it, for this reason.
        failure: Option<String>,
    },
}

/// Drive one session over Claude Code's stdio.
pub(super) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
    launched: Launched,
) {
    let (lines, incoming) = mpsc::channel(INBOUND);
    drop(tokio::spawn(io::read(stdout, lines)));
    let children = super::children::Children::new(launched.resumed);
    let mut session = Session {
        out: Writer::start(stdin),
        outputs,
        handshake_timeout: setup.handshake_timeout,
        conversation: launched.conversation.clone(),
        resumed: launched.resumed,
        _launched: launched,
        translator: Translator::default(),
        children,
        next: 0,
        initialize: None,
        interrupts: HashSet::new(),
        ready: false,
        turn: Turn::Idle,
    };
    let end = session.drive(incoming, commands).await;
    session.close(end).await;
}

struct Session {
    out: Writer,
    outputs: mpsc::Sender<AgentOutput>,
    handshake_timeout: Duration,
    /// The conversation, as the launch named it.
    conversation: HarnessSessionId,
    /// Whether the launch resumed it.
    resumed: bool,
    /// Kept for the session's life: its MCP file goes with it.
    _launched: Launched,
    translator: Translator,
    children: super::children::Children,
    /// The next request id's number.
    next: u64,
    /// `initialize`'s id while it is unanswered.
    initialize: Option<String>,
    /// Interrupts sent and not answered.
    interrupts: HashSet<String>,
    ready: bool,
    turn: Turn,
}

impl Session {
    /// Run until the connection ends; why it did.
    async fn drive(
        &mut self,
        mut incoming: mpsc::Receiver<Result<Read, Ended>>,
        mut commands: mpsc::Receiver<Command>,
    ) -> Disconnect {
        let Some(id) = self.request(HostRequest::Initialize {}).await else {
            return handshake_failed("Claude Code closed its input".to_owned());
        };
        self.initialize = Some(id);
        let handshake = tokio::time::sleep(self.handshake_timeout);
        tokio::pin!(handshake);
        let mut commanding = true;
        loop {
            tokio::select! {
                biased;
                command = commands.recv(), if self.ready && commanding => match command {
                    Some(command) => self.command(command).await,
                    // Nobody sends any more; the agent's output is still
                    // read until its owner stops it.
                    None => commanding = false,
                },
                item = incoming.recv() => match item {
                    Some(Ok(read)) => {
                        if let Some(end) = self.read(read).await {
                            return end;
                        }
                    }
                    Some(Err(Ended::Protocol(message))) => return self.failed(message),
                    Some(Err(Ended::Closed)) | None => {
                        return if self.ready {
                            Disconnect::Closed
                        } else if self.resumed {
                            // Whatever the CLI's reason (its log has it): a
                            // wake then fails, typed, and the session stays
                            // parked to be tried again, never waits.
                            handshake_failed(format!(
                                "Claude Code exited before it was ready, resuming conversation {}",
                                self.conversation
                            ))
                        } else {
                            handshake_failed("Claude Code exited before it was ready".to_owned())
                        };
                    }
                },
                () = &mut handshake, if !self.ready => {
                    return handshake_failed(format!(
                        "Claude Code did not answer the control protocol's initialize within {} s",
                        self.handshake_timeout.as_secs()
                    ));
                }
            }
        }
    }

    /// The connection failed for `message`.
    fn failed(&self, message: String) -> Disconnect {
        if self.ready {
            Disconnect::Protocol { message }
        } else {
            handshake_failed(message)
        }
    }

    /// Settle what is open, then say why the connection ended.
    async fn close(&mut self, end: Disconnect) {
        if self.turn != Turn::Idle {
            self.turn = Turn::Idle;
            self.emit(AgentEvent::TurnFailed {
                message: "the agent exited during the turn".to_owned(),
            })
            .await;
        }
        self.output(AgentOutput::Disconnected(end)).await;
    }

    async fn emit_all(&self, events: Vec<AgentEvent>) {
        for event in events {
            self.emit(event).await;
        }
    }

    async fn emit(&self, event: AgentEvent) {
        self.output(AgentOutput::Event(event)).await;
    }

    async fn output(&self, output: AgentOutput) {
        // The consumer gone means the session is over; the owner stops the
        // agent.
        let _gone = self.outputs.send(output).await;
    }
}

fn handshake_failed(message: String) -> Disconnect {
    Disconnect::Handshake {
        message,
        refused: None,
        fork_refused: None,
    }
}

#[cfg(test)]
mod tests;
