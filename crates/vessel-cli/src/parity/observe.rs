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

#[derive(Debug, Default)]
pub(super) struct Observed {
    pub(super) turn: Option<EventSeq>,
    permissions: BTreeSet<PermissionId>,
    questions: BTreeSet<QuestionId>,
    failure: Option<String>,
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
        if let SessionEvent::Agent { event } = &record.event {
            match event {
                AgentEvent::TurnStarted { .. } => {
                    self.turn = Some(record.seq);
                    self.failure = None;
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
                    self.permissions.insert(request.id);
                }
                AgentEvent::PermissionResolved { id, .. } => {
                    self.permissions.remove(id);
                }
                AgentEvent::QuestionAsked(question) => {
                    self.questions.insert(question.id);
                }
                AgentEvent::QuestionResolved { id, .. } => {
                    self.questions.remove(id);
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
