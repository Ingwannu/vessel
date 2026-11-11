//! The client's commands: prompt, cancel, steer (`steer`), answer a
//! permission request, read and change the agent's options.

use agent_client_protocol::Agent;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema as acp;
use vessel_proto::{AgentEvent, ContentBlock, OptionChoice};

use super::{Driver, Inbound, forward};
use crate::error::InputError;
use crate::handshake;
use crate::input::{Command, OptionsReply, Reply};
use crate::mapper;

impl Driver {
    pub(super) async fn command(&mut self, cx: &ConnectionTo<Agent>, command: Command) {
        match command {
            Command::Prompt { content, reply } => {
                let result = self.prompt(cx, content).await;
                answer(reply, result);
            }
            Command::CancelTurn { reply } => {
                let result = self.cancel(cx).await;
                answer(reply, result);
            }
            Command::Steer { content, reply } => self.steer(cx, content, reply),
            Command::Answer { id, outcome, reply } => {
                let result = self.permissions.answer(id, &outcome);
                if result.is_ok() {
                    self.emit(AgentEvent::PermissionResolved { id, outcome })
                        .await;
                }
                answer(reply, result);
            }
            Command::TurnRunning { reply } => {
                // The asker may have given up; nothing to do then.
                let _gone = reply.send(self.turn);
            }
            Command::Options { reply } => {
                let _gone = reply.send(self.options.listed());
            }
            Command::SetOption { choice, reply } => self.set_option(cx, choice, reply),
        }
    }

    /// Check `choice` against what the agent offers and send it; the
    /// answer comes back through `inbound` (`OptionSet`), in order with the
    /// agent's updates. Mid-turn too: whether a change applies then is the
    /// agent's to say, and its refusal is the caller's error.
    fn set_option(&self, cx: &ConnectionTo<Agent>, choice: OptionChoice, reply: OptionsReply) {
        let change = match self.options.change(&choice) {
            Ok(Some(change)) => change,
            Ok(None) => {
                let _gone = reply.send(Ok(self.options.listed()));
                return;
            }
            Err(error) => {
                let _gone = reply.send(Err(InputError::Option(error)));
                return;
            }
        };
        let Some(session) = self.session.clone() else {
            let _gone = reply.send(Err(InputError::Closed));
            return;
        };
        let inbound = self.inbound.clone();
        let task = {
            let cx = cx.clone();
            async move {
                let result = handshake::send(&cx, &session, change).await;
                let id = choice.id;
                forward(&inbound, Inbound::OptionSet { id, result, reply }).await;
                Ok(())
            }
        };
        if let Err(error) = cx.spawn(task) {
            // The reply went with the task; the caller sees the connection
            // closed when it is dropped.
            tracing::debug!(%error, "option change not sent: the connection is closing");
        }
    }

    async fn prompt(
        &mut self,
        cx: &ConnectionTo<Agent>,
        content: Vec<ContentBlock>,
    ) -> Result<(), InputError> {
        if self.turn {
            return Err(InputError::TurnInProgress);
        }
        let Some(session) = &self.session else {
            return Err(InputError::Closed);
        };
        let blocks = content
            .iter()
            .cloned()
            .map(mapper::prompt_block)
            .collect::<Result<Vec<_>, _>>()?;
        let sent = cx.send_request(acp::PromptRequest::new(session.clone(), blocks));
        let inbound = self.inbound.clone();
        let spawned = cx.spawn(async move {
            let result = sent.block_task().await;
            forward(&inbound, Inbound::TurnDone(result)).await;
            Ok(())
        });
        if spawned.is_err() {
            return Err(InputError::Closed);
        }
        self.turn = true;
        self.cancelling = false;
        self.emit(AgentEvent::TurnStarted { content }).await;
        Ok(())
    }

    async fn cancel(&mut self, cx: &ConnectionTo<Agent>) -> Result<(), InputError> {
        let Some(session) = &self.session else {
            return Err(InputError::Closed);
        };
        if !self.turn {
            return Ok(());
        }
        if cx
            .send_notification(acp::CancelNotification::new(session.clone()))
            .is_err()
        {
            return Err(InputError::Closed);
        }
        // ACP: after `session/cancel` the client answers every pending
        // permission request with `cancelled` (also those still on their
        // way, see `permission`); the agent then ends the turn with the
        // `cancelled` stop reason.
        self.cancelling = true;
        self.withdraw_permissions().await;
        Ok(())
    }
}

fn answer(reply: Reply, result: Result<(), InputError>) {
    // The caller may have stopped waiting; the command was applied anyway.
    if reply.send(result).is_err() {
        tracing::debug!("agent command reply dropped");
    }
}
