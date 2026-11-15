//! The client's commands: prompt, cancel, steer (`steer`), answer a
//! permission request or a question (`questions`), read and change the
//! agent's options.

use agent_client_protocol::Agent;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema as acp;
use vessel_proto::{AgentEvent, ContentBlock, OptionChoice};

use super::{Driver, Inbound, forward};
use crate::attached::Attachments;
use crate::error::InputError;
use crate::handshake;
use crate::input::{Command, OptionsReply, Reply};
use crate::mapper;

impl Driver {
    pub(super) async fn command(&mut self, cx: &ConnectionTo<Agent>, command: Command) {
        match command {
            Command::NativeCommandOffers { reply } => {
                let _closed = reply.send(crate::NativeCommandOffers::None);
            }
            Command::DecidePlan { request, reply } => {
                crate::input::plan::unsupported(request, reply);
            }
            Command::Native { intent, reply } => {
                crate::input::native_command::unsupported(intent, reply);
            }
            Command::Prompt {
                content,
                attachments,
                context: prefix,
                client_message_id,
                reply,
            } => {
                let result = self
                    .prompt(cx, content, attachments, prefix, client_message_id)
                    .await;
                answer(reply, result);
            }
            Command::CancelTurn { reply } => {
                let result = self.cancel(cx).await;
                answer(reply, result);
            }
            Command::Steer {
                content,
                attachments,
                client_message_id,
                reply,
            } => self.steer(cx, content, attachments, client_message_id, reply),
            Command::Answer { id, outcome, reply } => {
                let result = self.permissions.answer(id, &outcome);
                if result.is_ok() {
                    self.emit(AgentEvent::PermissionResolved { id, outcome })
                        .await;
                }
                answer(reply, result);
            }
            Command::AnswerQuestion { id, answer, reply } => {
                let result = self.answer_question(id, answer).await;
                self::answer(reply, result);
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
        mut attachments: Attachments,
        prefix: Option<crate::PromptContext>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
    ) -> Result<(), InputError> {
        if self.turn {
            return Err(InputError::TurnInProgress);
        }
        let Some(session) = &self.session else {
            return Err(InputError::Closed);
        };
        let has_prefix = prefix.is_some();
        let supplemental = prefix.map(|context| ContentBlock::Text {
            text: context.as_str().to_owned(),
        });
        let blocks = supplemental
            .into_iter()
            .chain(content.iter().cloned())
            .map(|block| mapper::prompt_block(block, &mut attachments, self.abilities))
            .collect::<Result<Vec<_>, _>>()?;
        let request = acp::PromptRequest::new(session.clone(), blocks);
        // Preparing reserves the actual request id without publishing. The
        // bounded side request is cloned only here so its complete JSON-RPC
        // envelope can be measured before any bytes reach the provider.
        let prepared = if has_prefix {
            let prepared = cx.prepare_request(request.clone());
            crate::context::check_request(&request, prepared.id())?;
            prepared
        } else {
            cx.prepare_request(request)
        };
        let sent = prepared.block_task();
        let inbound = self.inbound.clone();
        let spawned = cx.spawn(async move {
            let result = sent.await;
            forward(&inbound, Inbound::TurnDone(result)).await;
            Ok(())
        });
        if spawned.is_err() {
            return Err(InputError::Closed);
        }
        self.turn = true;
        self.cancelling = false;
        self.tail.reset();
        self.emit(AgentEvent::TurnStarted {
            provenance: None,
            content,
            client_message_id,
        })
        .await;
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
        // way, see `permission`), and every question with `cancel`; the
        // agent then ends the turn with the `cancelled` stop reason.
        self.cancelling = true;
        self.withdraw_permissions().await;
        self.withdraw_questions().await;
        Ok(())
    }
}

fn answer(reply: Reply, result: Result<(), InputError>) {
    // The caller may have stopped waiting; the command was applied anyway.
    if reply.send(result).is_err() {
        tracing::debug!("agent command reply dropped");
    }
}
