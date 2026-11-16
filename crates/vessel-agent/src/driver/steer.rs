//! Steering the running turn (protocol 10, ADR 0024): ACP's
//! `_session/steering` extension, which Claude's adapter, `codex-acp` and
//! `OpenCode` take.
//!
//! [Decision Log]
//! - Purpose: a message added to the turn that runs. ACP's
//!   `session/prompt` cannot do it: each agent queues, cancels or refuses
//!   a second prompt in its own way.
//! - The request asks `idleBehavior: promptRequired` in its `_meta`, as
//!   agent-orchestrator does: when the turn ended before the request
//!   arrived, the agent hands the message back rather than starting a
//!   turn of its own, whose end Vessel would never see. The session then
//!   starts a turn with it.
//! - The answer comes back through `inbound`, in order with the agent's
//!   updates (as an option change's does), so `steered` is logged where
//!   the agent took it. `startedNewTurn` means the agent ignored
//!   `promptRequired`: an error with a log line, never a turn Vessel did
//!   not start. "Method not found" means it does not steer after all.

use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::{Agent, Error, ErrorCode, UntypedMessage};
use serde::{Deserialize, Serialize};
use vessel_proto::ContentBlock;

use super::{Driver, Inbound, describe, forward};
use crate::attached::Attachments;
use crate::error::InputError;
use crate::input::{SteerOutcome, SteerReply};
use crate::mapper;

/// The extension's method.
const METHOD: &str = "_session/steering";

/// The request's parameters.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Steering<'a> {
    session_id: &'a acp::SessionId,
    prompt: Vec<acp::ContentBlock>,
    #[serde(rename = "_meta")]
    meta: Meta,
}

#[derive(Serialize)]
struct Meta {
    steering: IdleBehavior,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IdleBehavior {
    idle_behavior: &'static str,
}

/// The agent's answer.
#[derive(Deserialize)]
struct Answer {
    outcome: Outcome,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Outcome {
    Injected,
    PromptRequired,
    StartedNewTurn,
    #[serde(other)]
    Other,
}

impl Driver {
    /// Send `content` into the running turn; the answer comes back as
    /// [`Inbound::Steered`]. With no turn running, the caller is told to
    /// start one, and nothing is sent.
    pub(super) fn steer(
        &self,
        cx: &ConnectionTo<Agent>,
        content: Vec<ContentBlock>,
        mut attachments: Attachments,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
        reply: SteerReply,
    ) {
        if !self.abilities.steer {
            let _gone = reply.send(Err(InputError::CannotSteer));
            return;
        }
        let Some(session) = &self.session else {
            let _gone = reply.send(Err(InputError::Closed));
            return;
        };
        if !self.turn {
            let _gone = reply.send(Ok(SteerOutcome::PromptRequired));
            return;
        }
        let prompt = match content
            .iter()
            .cloned()
            .map(|block| mapper::prompt_block(block, &mut attachments, self.abilities))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(prompt) => prompt,
            Err(error) => {
                let _gone = reply.send(Err(error));
                return;
            }
        };
        let request = UntypedMessage::new(
            METHOD,
            Steering {
                session_id: session,
                prompt,
                meta: Meta {
                    steering: IdleBehavior {
                        idle_behavior: "promptRequired",
                    },
                },
            },
        );
        let request = match request {
            Ok(request) => request,
            Err(error) => {
                let _gone = reply.send(Err(InputError::Steering(describe(&error))));
                return;
            }
        };
        let sent = cx.send_request(request);
        let inbound = self.inbound.clone();
        let task = async move {
            let result = sent.block_task().await;
            forward(
                &inbound,
                Inbound::Steered {
                    provenance,
                    content,
                    client_message_id,
                    result,
                    reply,
                },
            )
            .await;
            Ok(())
        };
        if let Err(error) = cx.spawn(task) {
            // The reply went with the task; the caller sees the connection
            // closed when it is dropped.
            tracing::debug!(%error, "steering not sent: the connection is closing");
        }
    }

    /// The agent's answer to a steer.
    pub(super) async fn steered(
        &mut self,
        content: Vec<ContentBlock>,
        result: Result<serde_json::Value, Error>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
        reply: SteerReply,
    ) {
        let answer = match result.map(serde_json::from_value::<Answer>) {
            Ok(Ok(Answer {
                outcome: Outcome::Injected,
            })) => {
                self.echo(crate::InputEcho::steered(
                    content,
                    client_message_id,
                    provenance,
                ))
                .await;
                Ok(SteerOutcome::Injected)
            }
            Ok(Ok(Answer {
                outcome: Outcome::PromptRequired,
            })) => Ok(SteerOutcome::PromptRequired),
            Ok(Ok(Answer {
                outcome: Outcome::StartedNewTurn,
            })) => {
                tracing::warn!(
                    "the agent started a turn of its own for a steer that asked it not to"
                );
                Err(InputError::Steering(
                    "the agent started a turn of its own instead of taking the message".to_owned(),
                ))
            }
            Ok(Ok(Answer {
                outcome: Outcome::Other,
            })) => Err(InputError::Steering(
                "the agent answered with an outcome Vessel does not know".to_owned(),
            )),
            Ok(Err(error)) => Err(InputError::Steering(describe(&error))),
            Err(error) if error.code == ErrorCode::METHOD_NOT_FOUND => Err(InputError::CannotSteer),
            Err(error) => Err(InputError::SteerRefused {
                message: describe(&error),
            }),
        };
        // The caller may have stopped waiting; what the agent did stands.
        if reply.send(answer).is_err() {
            tracing::debug!("steer reply dropped");
        }
    }
}
