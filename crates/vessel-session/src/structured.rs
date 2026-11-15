//! The structured-mode session actor: an ACP agent's outputs become events.

use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};
use vessel_agent::{AgentInput, AgentOutput, AgentOutputs, AgentProcess, Disconnect};
use vessel_proto::{AgentEvent, ProfileName, SessionEvent, SessionOption, SessionStatus};

use crate::attachments::AttachmentLoader;
use crate::delivery::Desk;
use crate::lifecycle::{Lifecycle, drain_deadline};
use crate::moving::{Control, Launch};
use crate::parking::{Parking, Route};

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
    /// Isolated stop-boundary fault; it cannot exist in a product actor.
    #[cfg(test)]
    pub(crate) stop_signal_error: Option<vessel_proc::SignalError>,
    pub(crate) outputs: AgentOutputs,
    pub(crate) outputs_open: bool,
    /// The current agent's input, and where handles find it (`None` while
    /// the session moves to another profile).
    pub(crate) input: AgentInput,
    pub(crate) route: watch::Sender<Route>,
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
    pub(crate) questions: Option<std::sync::Arc<dyn crate::durable_questions::QuestionPersistence>>,
    /// The idle clock and the parked state (protocol 12, `parking`).
    pub(crate) parking: Parking,
    /// The node's `park_after` as it changes (protocol 12, `node.settings`);
    /// apart from `parking`, so the actor's `select` can wait on both.
    pub(crate) park_setting: watch::Receiver<Option<Duration>>,
    /// A record the log starts with (a fork's `forked`, protocol 14).
    /// Kept after it is published: a fork that fails before its agent is
    /// ready never held a conversation (`empty_fork`).
    pub(crate) opening: Option<Box<SessionEvent>>,
    /// Why the session cannot go on although its agent was ready: a
    /// rewind failed and its conversation did not start again (protocol
    /// 14, `forking`). It then ends `failed` with this, not `exited`.
    pub(crate) unrecoverable: Option<String>,
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
        if let Some(opening) = self.opening.as_deref() {
            self.life.publish(&mut self.agent, opening.clone()).await;
        }
        // The queue a session started again brought back, held (protocol
        // 10); nothing for a new session.
        self.publish_queue().await;

        let mut control_open = true;
        let code = loop {
            tokio::select! {
                biased;
                step = self.life.stop.next() => {
                    self.life.apply(&mut self.agent, step);
                    // A parked session has no process to wait for: a kill
                    // or the node stopping ends it now (protocol 12).
                    if self.parking.parked.is_some() {
                        break None;
                    }
                }
                code = self.agent.wait(), if self.parking.parked.is_none() => break code,
                output = self.outputs.next(), if self.outputs_open => match output {
                    Some(output) => self.output(output).await,
                    None => self.outputs_open = false,
                },
                control = self.control.recv(), if control_open => {
                    self.parking.touch();
                    match control {
                        Some(Control::Move(order)) => self.move_profile(*order).await,
                        Some(Control::Deliver(order)) => self.deliver(order).await,
                        Some(Control::Submit(order)) => self.submit_identified(order).await,
                        Some(Control::Plugin(order)) => self.append_plugin(order).await,
                        Some(Control::Question(order)) => self.answer_durable_question(order).await,
                        Some(Control::SideQuestion(order)) => self.side_question(order).await,
                        Some(Control::Queue(order)) => self.change_queue(order).await,
                        Some(Control::Cancel(order)) => self.cancel(order).await,
                        Some(Control::Park(order)) => self.park_order(order).await,
                        Some(Control::Rewind(order)) => self.rewind(order).await,
                        #[cfg(test)]
                        Some(Control::PrivateDir(reply)) => {
                            drop(reply.send(self.launch.setup.private_dir.clone()));
                        }
                        #[cfg(test)]
                        Some(Control::RefuseNextStop { source, ready }) => {
                            self.stop_signal_error = Some(source);
                            let _gone = ready.send(());
                        }
                        None => control_open = false,
                    }
                },
                Some(call) = self.life.model_calls.recv() => {
                    self.life.publish(&mut self.agent, SessionEvent::ModelCall(call)).await;
                }
                Some(rename) = self.life.renames.recv() => {
                    self.life.renamed(&mut self.agent, rename).await;
                }
                Ok(()) = self.life.forge.changed() => self.life.forge_changed(),
                // The node's setting changed: the timer follows it.
                Ok(()) = self.park_setting.changed() => {
                    let after = *self.park_setting.borrow_and_update();
                    self.parking.set_after(after);
                }
                // Last: anything else that is ready goes first, and is
                // activity (protocol 12, `parking`).
                () = self.parking.timer().as_mut(), if self.parking.armed() && self.ready => {
                    self.idle().await;
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
        } else if let Some(message) = self.unrecoverable.take() {
            SessionStatus::failed(message)
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
                empty_fork: matches!(self.opening.as_deref(), Some(SessionEvent::Forked { .. })),
            }
        };
        self.life.status(&mut self.agent, status).await;
    }

    /// A restarted agent's replay (a move, a wake, a session started
    /// again): the log has it already. Its stderr still goes in; `Ready`
    /// ends the replay.
    async fn replayed(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::Ready {
                abilities,
                harness_session,
            } => {
                self.move_state.replaying = false;
                self.desk.ready(abilities);
                self.parking.harness_session = Some(harness_session);
            }
            AgentEvent::HarnessLog { .. } => {
                self.life
                    .publish(&mut self.agent, SessionEvent::Agent { event })
                    .await;
            }
            AgentEvent::Notification(_)
            | AgentEvent::Compaction(_)
            | AgentEvent::Plugin(_)
            | AgentEvent::Error { .. }
            | AgentEvent::TurnStarted { .. }
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

    pub(crate) async fn output(&mut self, output: AgentOutput) {
        if let AgentOutput::Event(AgentEvent::QuestionAsked(question)) = &output
            && question.lifetime == vessel_proto::timeline::QuestionLifetime::Session
        {
            self.offer_durable_question(question.clone()).await;
            return;
        }
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
                // What a wake starts the agent with (protocol 12); an
                // option change is activity.
                self.parking.choices = options.iter().filter_map(SessionOption::choice).collect();
                self.parking.touch();
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
            AgentOutput::Usage(mut usage) => {
                // Moves, wakes and rewinds consume their Ready event. A
                // context report during that load is useful, but must not
                // charge historical cost under the previous account.
                usage.observation = if self.move_state.replaying {
                    vessel_proto::conversation::UsageObservation::Replayed
                } else {
                    vessel_proto::conversation::UsageObservation::Current
                };
                self.life
                    .publish(&mut self.agent, SessionEvent::UsageChanged { usage })
                    .await;
            }
            AgentOutput::Event(event) if self.move_state.replaying => {
                self.replayed(event).await;
            }
            AgentOutput::Disconnected(disconnect)
                if self.move_state.active || self.parking.parked.is_some() =>
            {
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
                if let AgentEvent::Ready {
                    harness_session, ..
                } = &event
                {
                    self.parking.harness_session = Some(harness_session.clone());
                    // A fork is made once (protocol 14): a later start of
                    // this agent (a wake, a move's fallback) loads it.
                    if self.launch.setup.fork.take().is_some() {
                        self.launch.setup.resume = Some(harness_session.clone());
                    }
                }
                // The agent's timeline is activity; its stderr is not (a
                // chatty adapter would never be parked).
                if !matches!(event, AgentEvent::HarnessLog { .. }) {
                    self.parking.touch();
                }
                let title = if let AgentEvent::TurnStarted { content, .. } = &event {
                    self.life.title_from(content)
                } else {
                    None
                };
                self.desk.observe(&event, self.life.out.next_seq());
                self.life
                    .publish(&mut self.agent, SessionEvent::Agent { event })
                    .await;
                if let Some(title) = title {
                    self.life
                        .rename(&mut self.agent, title, vessel_proto::TitleOrigin::Given)
                        .await;
                }
                if became_ready {
                    self.ready = true;
                    self.restore_durable_questions().await;
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
        // A fork the agent refused (protocol 14): the sentence a person
        // reads, not the transport's.
        Some(Disconnect::Handshake {
            fork_refused: Some(refusal),
            ..
        }) => crate::error::fork_refusal(*refusal).to_owned(),
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
            fork_refused: None,
        };
        assert_eq!(
            start_failure(Some(&handshake), Some(0)),
            "the ACP handshake failed: the agent does not offer model `x`"
        );
        assert!(start_failure(Some(&Disconnect::Closed), Some(3)).contains("code 3"));
        assert!(start_failure(None, None).contains("stopped"));
    }
}
