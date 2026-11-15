//! Replay to the subscription snapshot before classifying an agent as idle.
//! Process state Running alone is not evidence of a running agent turn.

use crate::{args::parity::WaitBudget, error::CliError};
use serde::Serialize;
use std::collections::BTreeSet;
use std::time::Duration;
use vessel_client::{NodeClient, SessionStream};
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, PermissionId, QuestionId, SessionEvent, SessionId,
    SessionStatus,
};

/// Bounded history reconstruction does not silently skip older turn boundaries.
const SNAPSHOT_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum State {
    Running,
    Idle,
    Permission,
    Error,
    Timeout,
}

impl State {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Idle => "idle",
            Self::Permission => "permission",
            Self::Error => "error",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Observed {
    pub(super) turn: Option<EventSeq>,
    permissions: BTreeSet<PermissionId>,
    questions: BTreeSet<QuestionId>,
    failure: Option<String>,
    pub(super) options: Vec<vessel_proto::SessionOption>,
}
impl Observed {
    pub(super) fn state(&self) -> State {
        if self.failure.is_some() {
            State::Error
        } else if !self.permissions.is_empty() || !self.questions.is_empty() {
            State::Permission
        } else if self.turn.is_some() {
            State::Running
        } else {
            State::Idle
        }
    }
    fn event(&mut self, record: &EventRecord) {
        if let SessionEvent::OptionsChanged { options } = &record.event {
            self.options.clone_from(options);
        }
        if let SessionEvent::Agent { event } = &record.event {
            match event {
                AgentEvent::TurnStarted { .. } => {
                    self.turn = Some(record.seq);
                    self.failure = None;
                    self.permissions.clear();
                    self.questions.clear();
                }
                AgentEvent::TurnEnded { .. } => {
                    self.turn = None;
                    self.permissions.clear();
                    self.questions.clear();
                }
                AgentEvent::TurnFailed { message } => {
                    self.turn = None;
                    self.failure = Some(message.clone());
                    self.permissions.clear();
                    self.questions.clear();
                }
                AgentEvent::PermissionRequested(request) => {
                    let _new = self.permissions.insert(request.id);
                }
                AgentEvent::PermissionResolved { id, .. } => {
                    let _present = self.permissions.remove(id);
                }
                AgentEvent::QuestionAsked(question) => {
                    let _new = self.questions.insert(question.id);
                }
                AgentEvent::QuestionResolved { id, .. } => {
                    let _present = self.questions.remove(id);
                }
                AgentEvent::Ready { .. }
                | AgentEvent::UserMessageChunk { .. }
                | AgentEvent::MessageChunk { .. }
                | AgentEvent::ThoughtChunk { .. }
                | AgentEvent::ToolCall(_)
                | AgentEvent::ToolCallUpdate(_)
                | AgentEvent::Plan { .. }
                | AgentEvent::HarnessLog { .. }
                | AgentEvent::Steered { .. } => {}
            }
        }
        if let SessionEvent::Status { status } = &record.event {
            match status {
                SessionStatus::Starting => {
                    self.failure = None;
                    self.turn = None;
                    self.permissions.clear();
                    self.questions.clear();
                }
                SessionStatus::Running => {}
                SessionStatus::Failed { message, .. } => {
                    self.failure = Some(message.clone());
                    self.turn = None;
                }
                SessionStatus::Exited {
                    code: None | Some(0),
                }
                | SessionStatus::Interrupted => {
                    self.turn = None;
                    self.permissions.clear();
                    self.questions.clear();
                }
                SessionStatus::Exited { code: Some(code) } => {
                    self.failure = Some(format!("agent exited with code {code}"));
                    self.turn = None;
                }
            }
        }
    }
}

pub(super) async fn snapshot(client: &NodeClient, id: SessionId) -> Result<Observed, CliError> {
    tokio::time::timeout(SNAPSHOT_DEADLINE, async {
        let mut stream = client.follow(id, EventSeq::ZERO).await?;
        history(&mut stream).await
    })
    .await
    .map_err(CliError::ParitySnapshotTimeout)?
}

async fn history(stream: &mut SessionStream) -> Result<Observed, CliError> {
    let boundary = stream.info().next_seq;
    let mut observed = Observed::default();
    while stream.cursor() < boundary {
        let record = stream.next().await.ok_or(CliError::Usage(
            "agent history ended before its snapshot boundary",
        ))??;
        observed.event(&record);
    }
    Ok(observed)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WaitResult {
    pub(super) agent_id: SessionId,
    pub(super) status: State,
    pub(super) message: String,
}

/// The caller chooses the cursor for a newly delivered prompt, avoiding an old idle boundary.
pub(super) async fn wait(
    client: &NodeClient,
    id: SessionId,
    budget: WaitBudget,
) -> Result<WaitResult, CliError> {
    let state = tokio::time::timeout(budget.duration(), async {
        let mut stream = client.follow(id, EventSeq::ZERO).await?;
        let mut observed = history(&mut stream).await?;
        loop {
            let state = observed.state();
            if state != State::Running {
                return Ok::<State, CliError>(state);
            }
            let record = stream
                .next()
                .await
                .ok_or(CliError::Usage("agent stream ended without a final state"))??;
            observed.event(&record);
        }
    })
    .await;
    let status = match state {
        Ok(state) => state?,
        Err(_) => State::Timeout,
    };
    Ok(WaitResult {
        agent_id: id,
        status,
        message: match status {
            State::Idle => "Agent is idle.",
            State::Permission => "Agent is waiting for permission or an answer.",
            State::Error => "Agent finished with error.",
            State::Timeout => "Agent did not finish before the wait deadline.",
            State::Running => "Agent is running.",
        }
        .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(seq: u64, event: SessionEvent) -> EventRecord {
        EventRecord {
            seq: EventSeq::new(seq),
            at: chrono::Utc::now(),
            event,
        }
    }
    fn agent(seq: u64, event: AgentEvent) -> EventRecord {
        record(seq, SessionEvent::Agent { event })
    }
    #[test]
    fn process_running_is_idle_and_a_new_turn_does_not_inherit_old_permissions() {
        let mut observed = Observed::default();
        observed.event(&record(
            0,
            SessionEvent::Status {
                status: SessionStatus::Running,
            },
        ));
        assert_eq!(observed.state(), State::Idle);
        observed.event(&agent(
            1,
            AgentEvent::TurnStarted {
                content: Vec::new(),
            },
        ));
        observed.event(&agent(
            2,
            AgentEvent::PermissionRequested(vessel_proto::PermissionRequest {
                id: PermissionId::random(),
                tool_call: vessel_proto::ToolCallId::new("tool"),
                options: Vec::new(),
            }),
        ));
        assert_eq!(observed.state(), State::Permission);
        observed.event(&agent(
            3,
            AgentEvent::TurnStarted {
                content: Vec::new(),
            },
        ));
        assert_eq!(observed.state(), State::Running);
        assert_eq!(observed.turn, Some(EventSeq::new(3)));
        observed.event(&agent(
            4,
            AgentEvent::TurnEnded {
                reason: vessel_proto::StopReason::EndTurn,
                usage: None,
                point: None,
            },
        ));
        assert_eq!(observed.state(), State::Idle);
    }
    #[test]
    fn reported_options_survive_process_exit_and_failure_is_reset_on_restart() {
        let mut observed = Observed::default();
        let option = vessel_proto::SessionOption {
            id: vessel_proto::OptionId::new("effort"),
            name: "Effort".to_owned(),
            description: None,
            category: vessel_proto::OptionCategory::ThoughtLevel,
            kind: vessel_proto::OptionKind::Select {
                current: vessel_proto::OptionValueId::new("high"),
                values: Vec::new(),
            },
        };
        observed.event(&record(
            0,
            SessionEvent::OptionsChanged {
                options: vec![option.clone()],
            },
        ));
        observed.event(&record(
            1,
            SessionEvent::Status {
                status: SessionStatus::Exited { code: Some(1) },
            },
        ));
        assert_eq!(observed.state(), State::Error);
        assert_eq!(observed.options, [option]);
        observed.event(&record(
            2,
            SessionEvent::Status {
                status: SessionStatus::Starting,
            },
        ));
        assert_eq!(observed.state(), State::Idle);
    }
}
