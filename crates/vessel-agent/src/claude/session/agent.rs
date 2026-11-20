//! The agent's traffic: its frames read, its requests answered, its
//! answers to the driver's own requests (the session loop's other half is
//! `commands`).

use vessel_proto::AgentEvent;

use super::super::io::Read;
use super::super::map::Translator;
use super::super::wire::{
    AgentRequest, Answer, ControlRequest, ControlResponse, Frame, System, TurnResult,
};
use super::{ABILITIES, Session, Turn, handshake_failed};
use crate::error::Disconnect;
use crate::introduction::Introduction;
use crate::output::AgentOutput;

impl Session {
    pub(super) async fn read(&mut self, read: Read) -> Option<Disconnect> {
        match read {
            Read::Frame(frame) => return self.frame(*frame).await,
            Read::UnreadableRequest { request_id, error } => {
                tracing::warn!(%error, "a Claude Code control request that does not read, refused");
                if let Some(request_id) = request_id {
                    self.refuse(request_id, "Vessel could not read this request")
                        .await;
                }
                self.stop_turn(format!(
                    "Claude Code asked Vessel something it could not read ({error}); it was \
                     refused and the turn stopped"
                ))
                .await;
            }
            Read::UnreadableResult(error) => {
                if self.turn != Turn::Idle {
                    self.turn = Turn::Idle;
                    self.emit(AgentEvent::TurnFailed {
                        message: format!("Claude Code's end of the turn did not read: {error}"),
                    })
                    .await;
                }
            }
        }
        None
    }

    pub(super) async fn frame(&mut self, frame: Frame) -> Option<Disconnect> {
        match frame {
            Frame::System(System::Init(init)) => {
                if init.session_id != self.conversation.as_str() {
                    tracing::warn!(
                        launched = %self.conversation,
                        reported = %init.session_id,
                        "Claude Code reports another conversation than the one it was launched with"
                    );
                }
            }
            Frame::StreamEvent(event) => {
                let events = self.translator.stream(&event);
                self.emit_all(events).await;
            }
            Frame::Assistant(message) => {
                let events = self.translator.assistant(&message);
                self.emit_all(events).await;
            }
            Frame::User(message) => {
                let events = self.translator.user(&message);
                self.emit_all(events).await;
            }
            Frame::Result(result) => self.ended(&result).await,
            Frame::ControlRequest(request) => self.asked(request).await,
            Frame::ControlResponse(ControlResponse { response }) => {
                return self.answered(response).await;
            }
            Frame::ControlCancelRequest(cancel) => self.agent_withdrew(&cancel.request_id).await,
            Frame::System(_) | Frame::KeepAlive | Frame::Unknown => {}
        }
        None
    }

    /// A turn's `result`: whatever the turn left asked is withdrawn first,
    /// so its resolutions come before the turn's end.
    pub(super) async fn ended(&mut self, result: &TurnResult) {
        self.withdraw_all().await;
        let Turn::Running { cancelled, failure } = std::mem::replace(&mut self.turn, Turn::Idle)
        else {
            tracing::debug!("a result outside a turn of the session's");
            return;
        };
        let event = match failure {
            Some(message) => AgentEvent::TurnFailed { message },
            None => Translator::end(result, cancelled),
        };
        self.emit(event).await;
    }

    /// A request of the agent's: a tool request is asked of a person
    /// (`asking`); anything else is refused and stops the turn.
    pub(super) async fn asked(&mut self, request: ControlRequest) {
        let ControlRequest {
            request_id,
            request,
        } = request;
        match request {
            AgentRequest::CanUseTool(asked) => self.ask(request_id, asked).await,
            AgentRequest::Unknown => {
                self.refuse(request_id, "Vessel does not serve this request")
                    .await;
                self.stop_turn(
                    "Claude Code asked Vessel for something this driver does not serve; it was \
                     refused and the turn stopped"
                        .to_owned(),
                )
                .await;
            }
        }
    }

    /// The answer to one of the driver's requests.
    pub(super) async fn answered(&mut self, answer: Answer) -> Option<Disconnect> {
        let (request_id, refusal) = match answer {
            Answer::Success { request_id, .. } => (request_id, None),
            Answer::Error { request_id, error } => (request_id, Some(error)),
        };
        if self.initialize.as_deref() == Some(request_id.as_str()) {
            self.initialize = None;
            if let Some(error) = refusal {
                return Some(handshake_failed(format!(
                    "Claude Code refused the control protocol's initialize: {error}"
                )));
            }
            self.ready().await;
        } else if self.interrupts.remove(&request_id) {
            if let Some(error) = refusal {
                tracing::info!(%error, "Claude Code refused to interrupt the turn");
            }
        } else {
            tracing::debug!(request_id, "an answer to no request of the driver's");
        }
        None
    }

    /// The handshake is done: say who the agent is, its options, ready.
    pub(super) async fn ready(&mut self) {
        self.ready = true;
        let introduction = Introduction {
            version: None,
            methods: Vec::new(),
        };
        self.output(AgentOutput::Introduced(Box::new(introduction)))
            .await;
        // No options yet: they are slice 3's.
        self.output(AgentOutput::Options(Vec::new())).await;
        self.emit(AgentEvent::Ready {
            harness_session: self.conversation.clone(),
            abilities: Some(ABILITIES),
        })
        .await;
    }
}
