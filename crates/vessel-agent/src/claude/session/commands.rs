//! The client's commands: prompts, cancels and what this slice does not
//! offer yet, and the requests the driver itself sends (the session
//! loop's other half is `agent`).

use vessel_proto::{ContentBlock, OptionChoice};

use super::super::wire::{HostAnswer, HostRequest, Outgoing, OutgoingBlock, OutgoingMessage};
use super::{Session, Turn};
use crate::error::InputError;
use crate::input::{Command, Reply};
use crate::options::OptionError;

impl Session {
    pub(super) async fn command(&mut self, command: Command) {
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
                provenance,
                content,
                context: side,
                client_message_id,
                reply,
                ..
            } => {
                self.prompt(content, side, client_message_id, provenance, reply)
                    .await
            }
            Command::CancelTurn { reply } => {
                self.cancel().await;
                answer(reply, Ok(()));
            }
            Command::Steer { reply, .. } => {
                let _gone = reply.send(Err(InputError::CannotSteer));
            }
            Command::Answer { id, reply, .. } => {
                answer(reply, Err(InputError::PermissionNotFound(id)));
            }
            Command::AnswerQuestion { id, reply, .. } => {
                answer(reply, Err(InputError::QuestionNotFound(id)));
            }
            Command::TurnRunning { reply } => {
                let _gone = reply.send(self.turn != Turn::Idle);
            }
            Command::Options { reply } => {
                let _gone = reply.send(Vec::new());
            }
            Command::SetOption { choice, reply } => {
                let _gone = reply.send(Err(InputError::Option(unknown(choice))));
            }
        }
    }

    /// A prompt, with a side question's context (`side`) when the node gives one
    /// (ADR 0045). The context goes first and only to the agent, as the ACP
    /// driver sends it: the timeline's `TurnStarted` shows the question
    /// alone, and nothing of the context is kept for a later prompt. The
    /// bound is the ACP driver's (`PROMPT_BYTES`), measured on what goes
    /// to the agent here, the whole stream-json line, before anything is
    /// sent; past it the prompt is refused (`ContextLimit`) and no turn
    /// starts.
    pub(super) async fn prompt(
        &mut self,
        content: Vec<ContentBlock>,
        side: Option<crate::PromptContext>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
        reply: Reply,
    ) {
        if self.turn != Turn::Idle {
            answer(reply, Err(InputError::TurnInProgress));
            return;
        }
        let mut blocks = match prompt_blocks(&content) {
            Ok(blocks) => blocks,
            Err(error) => {
                answer(reply, Err(error));
                return;
            }
        };
        let bounded = side.is_some();
        if let Some(side) = side {
            blocks.insert(
                0,
                OutgoingBlock::Text {
                    text: side.as_str().to_owned(),
                },
            );
        }
        let frame = Outgoing::User {
            message: OutgoingMessage {
                role: "user",
                content: blocks,
            },
            parent_tool_use_id: None,
            session_id: self.conversation.to_string(),
        };
        if bounded {
            let fits = serde_json::to_vec(&frame)
                .map(|line| line.len() <= vessel_proto::side_questions::PROMPT_BYTES);
            match fits {
                Ok(true) => {}
                Ok(false) => {
                    answer(reply, Err(InputError::ContextLimit));
                    return;
                }
                Err(error) => {
                    answer(reply, Err(InputError::InvalidContent(error)));
                    return;
                }
            }
        }
        if !self.out.send(&frame).await {
            answer(reply, Err(InputError::Closed));
            return;
        }
        self.turn = Turn::Running {
            cancelled: false,
            failure: None,
        };
        self.output(crate::AgentOutput::InputEcho(crate::InputEcho::started(
            content,
            client_message_id,
            provenance,
        )))
        .await;
        answer(reply, Ok(()));
    }

    /// The client stops the running turn (a no-op without one).
    pub(super) async fn cancel(&mut self) {
        let Turn::Running { cancelled, .. } = &mut self.turn else {
            return;
        };
        if *cancelled {
            return;
        }
        *cancelled = true;
        self.interrupt().await;
    }

    /// The driver stops the running turn, which then ends failed for
    /// `reason` (a no-op without one: the refusal was answered).
    pub(super) async fn stop_turn(&mut self, reason: String) {
        let Turn::Running { failure, cancelled } = &mut self.turn else {
            return;
        };
        if failure.is_some() {
            return;
        }
        *failure = Some(reason);
        if !*cancelled {
            *cancelled = true;
            self.interrupt().await;
        }
    }

    pub(super) async fn interrupt(&mut self) {
        if let Some(id) = self.request(HostRequest::Interrupt {}).await {
            let _new = self.interrupts.insert(id);
        }
    }

    /// Refuse the agent's request `request_id`.
    pub(super) async fn refuse(&mut self, request_id: String, why: &str) {
        let answer = HostAnswer::Error {
            request_id,
            error: why.to_owned(),
        };
        let _sent = self
            .out
            .send(&Outgoing::ControlResponse { response: answer })
            .await;
    }

    /// Send a request of the driver's; its id, or `None` once the agent's
    /// input is closed.
    pub(super) async fn request(&mut self, request: HostRequest) -> Option<String> {
        self.next = self.next.wrapping_add(1);
        let request_id = format!("vessel-{}", self.next);
        let frame = Outgoing::ControlRequest {
            request_id: request_id.clone(),
            request,
        };
        self.out.send(&frame).await.then_some(request_id)
    }
}
/// A prompt's content as Claude Code's message: text as text, a link as
/// its text.
///
/// # Errors
/// [`InputError::AttachmentNotTaken`] for an attachment (this driver takes
/// none yet), [`InputError::InvalidContent`] for an opaque block.
pub(super) fn prompt_blocks(content: &[ContentBlock]) -> Result<Vec<OutgoingBlock>, InputError> {
    content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => Ok(OutgoingBlock::Text { text: text.clone() }),
            ContentBlock::ResourceLink { uri, name } => Ok(OutgoingBlock::Text {
                text: format!("[{name}]({uri})"),
            }),
            ContentBlock::Attachment { attachment } => Err(InputError::AttachmentNotTaken {
                id: attachment.id,
                kind: attachment.kind,
            }),
            ContentBlock::Opaque { .. } => Err(InputError::InvalidContent(
                serde::de::Error::custom("Claude Code's driver takes text and links"),
            )),
        })
        .collect()
}

fn unknown(choice: OptionChoice) -> OptionError {
    OptionError::Unknown {
        id: choice.id,
        offered: Vec::new(),
    }
}

fn answer(reply: Reply, result: Result<(), InputError>) {
    if reply.send(result).is_err() {
        tracing::debug!("agent command reply dropped");
    }
}
