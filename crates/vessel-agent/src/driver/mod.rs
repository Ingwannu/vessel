//! The ACP connection driver: one task per agent session.
//!
//! Owns the ACP connection, the session state (ready or not, turn running
//! or not, pending permissions and questions, announced tool calls) and
//! turns ACP traffic into [`AgentOutput`]s. Must not touch the process (the
//! owner of [`AgentProcess`](crate::AgentProcess) stops it) or read ACP
//! payloads (the `mapper` does).
//!
//! [Decision Log]
//! - Purpose: ACP handlers run inside the SDK's dispatch loop, which is
//!   blocked until they return; a handler that waited for a client's
//!   permission answer, or for later traffic, would stall or deadlock the
//!   connection.
//! - Choice: handlers only forward what arrives into one bounded `inbound`
//!   channel and return. The handshake and each prompt run as spawned
//!   connection tasks that also report into `inbound`. One loop consumes
//!   `inbound` and client commands and owns all state.
//! - Why one channel: the SDK routes a prompt's response only after the
//!   handlers for earlier notifications have returned, so a single FIFO
//!   keeps "last message chunk, then turn ended" in order. Two channels
//!   could deliver the end of a turn before its last chunk. The same holds
//!   for `session/load`, whose replayed history arrives before its
//!   response: the handshake runs in a task, so the loop keeps draining the
//!   replay while the handshake waits.
//! - Backpressure: `inbound` and the outputs are bounded, and the SDK reads
//!   the agent's stdout through `transport`, which holds each update and
//!   permission request back until this loop has taken the previous ones.
//!   When the consumer stops reading, the loop stops, the transport stops
//!   reading, and the agent blocks on its writes. (The SDK's own queues are
//!   unbounded; `transport` is what bounds what reaches them.)
//!
//! - Option changes (protocol 9) wait on the agent's answer like a prompt,
//!   so each runs as a task that reports into `inbound` (`OptionSet`);
//!   the loop keeps the session's options (`options`) and reports them
//!   whenever they differ from what it last reported.
//!
//! - Questions (protocol 10) are the agent's requests, like permission
//!   requests; the agent may withdraw one, which a task per question
//!   watches and reports into `inbound` (`Withdrawn`), as option changes
//!   report back (`questions`).
//!
//! Layout: `connection` sets the connection up; this module runs the
//! loop, which takes the client's commands (`commands`, `steer`) and the
//! agent's traffic (`agent`, `questions`) in order; `close` settles what a
//! closed connection leaves open. All of them share the one [`Driver`]
//! state.

mod agent;
mod close;
mod commands;
mod connection;
mod questions;
mod steer;
mod tail;

use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::v1::{ConnectionTo, Responder};
use agent_client_protocol::{Agent, Error};
use tokio::sync::mpsc;
use vessel_proto::{
    AgentAbilities, AgentCommand, AgentEvent, ContentBlock, ContextUsage, HarnessSessionId,
    OptionId, QuestionId, SessionOption, ToolCallId, TurnPoint,
};

use crate::error::Disconnect;
use crate::handshake::{self, AgentSetup, ForkRefusal, HandshakeError, Started};
use crate::input::{Command, OptionsReply, SteerReply};
use crate::mapper;
use crate::options::{Applied, Options};
use crate::output::AgentOutput;
use crate::permissions::Permissions;
use crate::questions::Questions;
use crate::recent::Recent;
use crate::transport::Gate;

pub(crate) use connection::run;

/// Bound of the inbound queue between the SDK's handlers and the loop.
const INBOUND: usize = 64;

/// After the connection closed with a turn open, how long to wait for each
/// further item while the turn's outcome is still on its way.
const AFTER_CLOSE: Duration = Duration::from_secs(2);

/// Something that arrived from the agent, in arrival order.
enum Inbound {
    Started(Result<Started, HandshakeError>),
    Update(acp::SessionNotification),
    Permission(
        acp::RequestPermissionRequest,
        Responder<acp::RequestPermissionResponse>,
    ),
    /// The agent asks the person something (protocol 10).
    Question(
        acp::CreateElicitationRequest,
        Responder<acp::CreateElicitationResponse>,
    ),
    /// The agent withdrew a question it asked (`$/cancel_request`).
    Withdrawn(QuestionId),
    TurnDone(Result<acp::PromptResponse, Error>),
    /// The agent answered an option change.
    OptionSet {
        id: OptionId,
        result: Result<Applied, Error>,
        reply: OptionsReply,
    },
    /// The agent answered a steer (protocol 10).
    Steered {
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        content: Vec<ContentBlock>,
        result: Result<serde_json::Value, Error>,
        reply: SteerReply,
    },
}

/// Session state, owned by the driver loop.
struct Driver {
    /// Kept for the tasks the loop spawns (handshake, prompts).
    inbound: mpsc::Sender<Inbound>,
    inbound_receiver: mpsc::Receiver<Inbound>,
    commands: mpsc::Receiver<Command>,
    commands_open: bool,
    outputs: mpsc::Sender<AgentOutput>,
    /// The ACP session, once the handshake succeeded.
    session: Option<acp::SessionId>,
    /// Whether a prompt is in flight.
    turn: bool,
    /// The turn in flight was cancelled: permission requests still on
    /// their way are answered `cancelled` as they arrive.
    cancelling: bool,
    permissions: Permissions,
    questions: Questions,
    /// Tool calls already announced on the timeline.
    /// Recent ones only, so a long session does not keep them all.
    tool_calls: Recent<ToolCallId>,
    /// The agent's last message and what followed it: a turn's point and
    /// a fork's check (protocol 14, `tail`).
    tail: tail::Tail,
    /// Holds the agent's lines back while this loop is behind (see
    /// `transport`); each update and permission request taken from
    /// `inbound` returns its permit.
    gate: Gate,
    /// The session's config options, as the agent last reported them.
    options: Options,
    /// What was last reported as [`AgentOutput::Options`]; `None` before
    /// the handshake ends.
    reported: Option<Vec<SessionOption>>,
    /// What the agent said it can do (protocol 10), once it has.
    abilities: AgentAbilities,
    /// The last commands and usage reported (protocol 10): an agent that
    /// sends the same list again logs nothing.
    last_commands: Option<Vec<AgentCommand>>,
    last_usage: Option<ContextUsage>,
}

impl Driver {
    async fn run(
        &mut self,
        cx: ConnectionTo<Agent>,
        setup: AgentSetup,
    ) -> Result<Disconnect, Error> {
        cx.spawn({
            let cx = cx.clone();
            let inbound = self.inbound.clone();
            async move {
                let started = handshake::run_within(&cx, &setup).await;
                forward(&inbound, Inbound::Started(started)).await;
                Ok(())
            }
        })?;
        loop {
            // Order matters. Commands first: they are few (the queue holds
            // 16 and every caller waits for its reply) and cheap, while the
            // agent can stream updates faster than a slow client reads them,
            // keeping `inbound` ready on every turn of the loop; polled
            // second, a `CancelTurn` or a permission answer would wait for
            // the stream to end. Taking a command before queued updates is
            // safe: a turn's end (`TurnEnded`) comes only from its
            // `TurnDone`, which is handled in order with the updates. The
            // close is last, so everything queued is handled before it.
            tokio::select! {
                biased;
                command = self.commands.recv(), if self.session.is_some() && self.commands_open => {
                    match command {
                        Some(command) => self.command(&cx, command).await,
                        None => self.commands_open = false,
                    }
                }
                item = self.inbound_receiver.recv() => {
                    // The loop holds a sender itself, so this never ends
                    // while it runs.
                    let Some(item) = item else {
                        return Ok(Disconnect::Closed);
                    };
                    if let Some(end) = self.handle(&cx, item).await {
                        return Ok(end);
                    }
                }
                () = cx.incoming_closed() => {
                    self.settle_after_close(&cx).await;
                    return Ok(Disconnect::Closed);
                }
            }
        }
    }

    /// The turn's point (protocol 14): its last agent message, in the
    /// conversation now open.
    fn point(&self) -> Option<TurnPoint> {
        let message = self.tail.point()?;
        let session = self.session.as_ref()?;
        Some(TurnPoint {
            message,
            conversation: HarnessSessionId::new(Arc::clone(&session.0)),
        })
    }

    /// Apply one inbound item; `Some` ends the session.
    async fn handle(&mut self, cx: &ConnectionTo<Agent>, item: Inbound) -> Option<Disconnect> {
        match item {
            Inbound::Started(Ok(Started {
                session,
                fork_point,
                deletes,
                options,
                abilities,
                introduction,
            })) => {
                // A fork at a point is checked against its replay, which
                // came before this (protocol 14, ADR 0052). A refused fork
                // is deleted where the agent can, not left in its history.
                if let Some(anchor) = fork_point
                    && !self.tail.ends_with(&anchor)
                {
                    tracing::info!(%anchor, "the agent's fork does not end at its point");
                    if deletes {
                        handshake::delete_orphan(cx, &session).await;
                    }
                    return Some(Disconnect::Handshake {
                        message: String::from("the agent did not fork at the chosen point"),
                        refused: None,
                        fork_refused: Some(ForkRefusal::NotAtPoint),
                    });
                }
                let harness_session = HarnessSessionId::new(Arc::clone(&session.0));
                self.session = Some(session);
                self.options = options;
                self.abilities = abilities;
                self.output(AgentOutput::Introduced(Box::new(introduction)))
                    .await;
                // Before `Ready`, always, even empty: whoever sees the
                // session ready knows what it runs with, and "offers
                // nothing" is told from "not known yet".
                self.report_options().await;
                self.emit(AgentEvent::Ready {
                    harness_session,
                    abilities: Some(abilities),
                })
                .await;
            }
            Inbound::Started(Err(error)) => {
                return Some(Disconnect::Handshake {
                    message: describe(&error),
                    refused: error.refused().map(Box::new),
                    fork_refused: error.fork_refused(),
                });
            }
            Inbound::Update(notification) => {
                self.gate.release();
                self.update(notification).await;
            }
            Inbound::Permission(request, responder) => {
                self.gate.release();
                self.permission(request, responder).await;
            }
            Inbound::Question(request, responder) => {
                self.gate.release();
                self.question(cx, request, responder).await;
            }
            Inbound::Withdrawn(id) => self.withdrawn(id).await,
            Inbound::TurnDone(result) => {
                self.turn = false;
                self.cancelling = false;
                self.withdraw_permissions().await;
                self.withdraw_questions().await;
                let event = match result {
                    Ok(response) => AgentEvent::TurnEnded {
                        reason: mapper::stop_reason(response.stop_reason),
                        usage: response.usage.as_ref().map(mapper::turn_usage),
                        point: self.point(),
                    },
                    Err(error) => AgentEvent::TurnFailed {
                        message: describe(&error),
                    },
                };
                self.emit(event).await;
            }
            Inbound::OptionSet { id, result, reply } => {
                let answer = self.option_applied(id, result).await;
                // The caller may have stopped waiting; the change stands.
                if reply.send(answer).is_err() {
                    tracing::debug!("option change reply dropped");
                }
            }
            Inbound::Steered {
                provenance,
                client_message_id,
                content,
                result,
                reply,
            } => {
                self.steered(content, result, client_message_id, provenance, reply)
                    .await;
            }
        }
        None
    }

    async fn option_applied(
        &mut self,
        id: OptionId,
        result: Result<Applied, Error>,
    ) -> Result<Vec<SessionOption>, crate::InputError> {
        match result {
            Ok(applied) => {
                self.options.apply(applied);
                self.report_options().await;
                Ok(self.options.listed())
            }
            Err(error) => Err(crate::error::InputError::Refused {
                id,
                message: describe(&error),
            }),
        }
    }

    /// Report the options if they differ from the last report (always the
    /// first time).
    pub(super) async fn report_options(&mut self) {
        let listed = self.options.listed();
        if self.reported.as_ref() == Some(&listed) {
            return;
        }
        self.reported = Some(listed.clone());
        if self
            .outputs
            .send(AgentOutput::Options(listed))
            .await
            .is_err()
        {
            tracing::debug!("agent outputs dropped");
        }
    }

    /// Hand an output other than a timeline event to the session.
    pub(super) async fn output(&mut self, output: AgentOutput) {
        if self.outputs.send(output).await.is_err() {
            tracing::debug!("agent outputs dropped");
        }
    }

    /// `&mut self` rather than `&self`: the session holds permission
    /// responders, which are `Send` but not `Sync`, so a shared borrow held
    /// across this `.await` would make the driver future non-`Send`.
    async fn emit(&mut self, event: AgentEvent) {
        if self.outputs.send(AgentOutput::Event(event)).await.is_err() {
            tracing::debug!("agent outputs dropped");
        }
    }

    async fn echo(&mut self, echo: crate::InputEcho) {
        if self
            .outputs
            .send(AgentOutput::InputEcho(echo))
            .await
            .is_err()
        {
            tracing::debug!("agent accepted input outputs dropped");
        }
    }
}

async fn forward(inbound: &mpsc::Sender<Inbound>, item: Inbound) {
    // Fails only after the loop ended; the connection is going away and a
    // dropped permission responder answers the agent with an error.
    if inbound.send(item).await.is_err() {
        tracing::debug!("agent traffic after the session loop ended");
    }
}

/// An error and its sources, as one line for people.
pub(crate) fn describe(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
