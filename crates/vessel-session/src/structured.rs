//! The structured-mode session actor: an ACP agent's outputs become events.

use tokio::sync::{mpsc, oneshot, watch};
use vessel_agent::{AgentInput, AgentOutput, AgentOutputs, AgentProcess, Disconnect};
use vessel_proto::{AgentEvent, ProfileName, SessionEvent, SessionStatus};

use crate::attachments::AttachmentLoader;
use crate::delivery::Desk;
use crate::lifecycle::{Lifecycle, drain_deadline};
use crate::moving::{Control, Launch};

/// Owns an ACP agent process for the life of a structured session.
///
/// Status follows the handshake: `Starting` until the agent's `Ready`
/// event, then `Running`. A session that never became ready ends `Failed`
/// with the reason (the handshake error, or that the agent exited first),
/// unless it was killed while starting, which ends `Exited` as a killed
/// terminal session does; one that became ready ends `Exited` with the
/// exit code. If the ACP connection
/// ends while the process lives on, the process is stopped: it can no
/// longer be driven.
pub(crate) struct StructuredActor {
    pub(crate) agent: AgentProcess,
    pub(crate) outputs: AgentOutputs,
    pub(crate) outputs_open: bool,
    /// The current agent's input, and where handles find it (`None` while
    /// the session moves to another profile).
    pub(crate) input: AgentInput,
    pub(crate) route: watch::Sender<Option<AgentInput>>,
    /// Moves to another profile, from the runtime.
    pub(crate) control: mpsc::Receiver<Control>,
    /// How the current agent was started.
    pub(crate) launch: Launch,
    /// The profile it runs with; `None` is the harness's default.
    pub(crate) profile: Option<ProfileName>,
    pub(crate) life: Lifecycle,
    pub(crate) ready: bool,
    pub(crate) disconnect: Option<Disconnect>,
    /// The handshake or the protocol failed before anyone killed the
    /// session (a failure after a kill is the kill's doing).
    pub(crate) failed_before_kill: bool,
    /// Where a move to another profile stands.
    pub(crate) move_state: MoveState,
    /// Deliveries and the queue (protocol 10, `delivery`).
    pub(crate) desk: Desk,
    /// A session started again (protocol 10, `again`) publishes nothing
    /// until its row is reopened: answered then, dropped if it is not.
    pub(crate) gate: Option<oneshot::Receiver<()>>,
    /// Where a prompt's attachments are loaded (protocol 10,
    /// `attachments`); `None` in a runtime that keeps none.
    pub(crate) loader: Option<AttachmentLoader>,
}

/// A structured session's move state (see `moving`).
#[derive(Debug, Default)]
pub(crate) struct MoveState {
    /// A move is under way: the agent's end is not the session's.
    pub(crate) active: bool,
    /// A restarted agent replays the conversation before it is ready; the
    /// log has it already, so it is not published again.
    pub(crate) replaying: bool,
    /// A turn ended (for a forced move or an interrupt, which wait for
    /// its cancel).
    pub(crate) turn_over: bool,
    /// How an agent's connection ended during the move.
    pub(crate) disconnect: Option<Disconnect>,
}

impl StructuredActor {
    /// Run until the agent has exited and the final status is published.
    pub(crate) async fn run(mut self) {
        // A session started again waits for its row to be reopened. If it
        // is not, it ends without a record, and its agent goes with it
        // (dropping the process kills its group).
        if let Some(gate) = self.gate.take()
            && gate.await.is_err()
        {
            tracing::debug!("not started again: its row was not reopened");
            return;
        }
        self.life.starting(&mut self.agent).await;
        // The queue a session started again brought back, held (protocol
        // 10); nothing for a new session.
        self.publish_queue().await;

        let mut control_open = true;
        let code = loop {
            tokio::select! {
                biased;
                step = self.life.stop.next() => self.life.apply(&mut self.agent, step),
                code = self.agent.wait() => break code,
                output = self.outputs.next(), if self.outputs_open => match output {
                    Some(output) => self.output(output).await,
                    None => self.outputs_open = false,
                },
                control = self.control.recv(), if control_open => match control {
                    Some(Control::Move(order)) => self.move_profile(*order).await,
                    Some(Control::Deliver(order)) => self.deliver(order).await,
                    Some(Control::Queue(order)) => self.change_queue(order).await,
                    Some(Control::Cancel(order)) => self.cancel(order).await,
                    None => control_open = false,
                },
                Some(call) = self.life.model_calls.recv() => {
                    self.life.publish(&mut self.agent, SessionEvent::ModelCall(call)).await;
                }
                Some(rename) = self.life.renames.recv() => {
                    self.life.renamed(&mut self.agent, rename).await;
                }
            }
            // The queue moves once a turn's end is in the log.
            self.after_turn().await;
        };
        if self.outputs_open {
            self.drain().await;
        }
        self.life.drain_model_calls(&mut self.agent).await;
        self.queue_at_end().await;

        let status = if self.life.stop.interrupted() {
            SessionStatus::Interrupted
        } else if self.ready || (self.life.stop.killed() && !self.failed_before_kill) {
            // Killed while it was still starting ends as a killed terminal
            // session does: exited, with whatever code the stop left. (The
            // kill also fails the pending handshake; that failure is the
            // kill's doing.) A handshake that had failed before the kill
            // stays the more useful answer.
            SessionStatus::Exited { code }
        } else {
            SessionStatus::Failed {
                message: start_failure(self.disconnect.as_ref(), code),
            }
        };
        self.life.status(&mut self.agent, status).await;
    }

    pub(crate) async fn output(&mut self, output: AgentOutput) {
        // A session started again drops its agent's replay of the
        // conversation until `ready`, as a move does (`session/load`); that
        // `ready` is its run's first, so it is published as a new
        // session's is, and the session runs.
        if self.move_state.replaying
            && !self.ready
            && matches!(output, AgentOutput::Event(AgentEvent::Ready { .. }))
        {
            self.move_state.replaying = false;
        }
        match output {
            // Also from an agent a move restarts: under another profile it
            // may offer other options, and its report is the session's now.
            AgentOutput::Options(options) => {
                self.life
                    .publish(&mut self.agent, SessionEvent::OptionsChanged { options })
                    .await;
            }
            // Protocol 10: the driver reports each only when it differs
            // from its last, so each is one record per change.
            AgentOutput::Commands(commands) => {
                self.life
                    .publish(&mut self.agent, SessionEvent::CommandsChanged { commands })
                    .await;
            }
            // What the agent said of itself is `agent.status`'s, read from
            // probes; a session has nothing to keep of it.
            AgentOutput::Introduced(_) => {}
            AgentOutput::Usage(usage) => {
                self.life
                    .publish(&mut self.agent, SessionEvent::UsageChanged { usage })
                    .await;
            }
            AgentOutput::Event(event) if self.move_state.replaying => {
                // A restarted agent's replay: the log has it already. Its
                // stderr still goes in; `Ready` ends the replay.
                match event {
                    AgentEvent::Ready { abilities, .. } => {
                        self.move_state.replaying = false;
                        self.desk.ready(abilities);
                    }
                    AgentEvent::HarnessLog { .. } => {
                        self.life
                            .publish(&mut self.agent, SessionEvent::Agent { event })
                            .await;
                    }
                    AgentEvent::TurnStarted { .. }
                    | AgentEvent::UserMessageChunk { .. }
                    | AgentEvent::MessageChunk { .. }
                    | AgentEvent::ThoughtChunk { .. }
                    | AgentEvent::ToolCall(_)
                    | AgentEvent::ToolCallUpdate(_)
                    | AgentEvent::Plan { .. }
                    | AgentEvent::PermissionRequested(_)
                    | AgentEvent::PermissionResolved { .. }
                    | AgentEvent::TurnEnded { .. }
                    | AgentEvent::TurnFailed { .. }
                    | AgentEvent::Steered { .. }
                    | AgentEvent::QuestionAsked(_)
                    | AgentEvent::QuestionResolved { .. } => {}
                }
            }
            AgentOutput::Disconnected(disconnect) if self.move_state.active => {
                // The old agent being stopped, or the new one failing to
                // start: the move decides what follows, not the session.
                tracing::debug!(%disconnect, "agent connection ended during a move");
                self.move_state.disconnect = Some(disconnect);
            }
            AgentOutput::Event(event) => {
                if matches!(
                    event,
                    AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. }
                ) {
                    self.move_state.turn_over = true;
                }
                let became_ready = !self.ready && matches!(event, AgentEvent::Ready { .. });
                let title = if let AgentEvent::TurnStarted { content } = &event {
                    self.life.title_from(content)
                } else {
                    None
                };
                self.desk.observe(&event, self.life.out.next_seq());
                self.life
                    .publish(&mut self.agent, SessionEvent::Agent { event })
                    .await;
                if let Some(title) = title {
                    self.life.rename(&mut self.agent, title).await;
                }
                if became_ready {
                    self.ready = true;
                    self.life
                        .status(&mut self.agent, SessionStatus::Running)
                        .await;
                }
            }
            AgentOutput::Disconnected(disconnect) => {
                tracing::debug!(%disconnect, "agent connection ended");
                self.failed_before_kill = !self.life.stop.killed()
                    && matches!(
                        disconnect,
                        Disconnect::Handshake { .. } | Disconnect::Protocol { .. }
                    );
                self.disconnect = Some(disconnect);
                self.life.begin_stop(&mut self.agent);
            }
        }
    }

    /// Publish what is still buffered after the exit, for a bounded time.
    async fn drain(&mut self) {
        let deadline = drain_deadline();
        loop {
            tokio::select! {
                output = self.outputs.next() => match output {
                    Some(output) => self.output(output).await,
                    None => return,
                },
                () = tokio::time::sleep_until(deadline) => {
                    tracing::warn!(
                        pid = self.agent.pid(),
                        "agent output still open after exit; a process escaped the group"
                    );
                    return;
                }
            }
        }
    }
}

/// Why a session that never became ready failed.
fn start_failure(disconnect: Option<&Disconnect>, code: Option<i32>) -> String {
    match disconnect {
        Some(failure @ (Disconnect::Handshake { .. } | Disconnect::Protocol { .. })) => {
            failure.to_string()
        }
        Some(Disconnect::Closed) | None => match code {
            Some(code) => format!(
                "the agent exited with code {code} before the ACP handshake completed; see its log"
            ),
            None => String::from("the agent was stopped before the ACP handshake completed"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_failures_say_what_happened() {
        let handshake = Disconnect::Handshake {
            message: "the agent does not offer model `x`".to_owned(),
            refused: None,
        };
        assert_eq!(
            start_failure(Some(&handshake), Some(0)),
            "the ACP handshake failed: the agent does not offer model `x`"
        );
        assert!(start_failure(Some(&Disconnect::Closed), Some(3)).contains("code 3"));
        assert!(start_failure(None, None).contains("stopped"));
    }
}
