//! CLI result shapes are projections of typed acknowledgements, not native snapshots.
//! A finished run/send says completed; the separate wait command still says idle.
use crate::{
    args::parity::Presentation,
    error::CliError,
    parity::{
        observe::{State, WaitResult},
        output::{self, Row},
    },
};
use serde::Serialize;
use std::path::Path;
use vessel_proto::{Delivered, HarnessRef, SessionId, SessionInfo, SessionTitle};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum RunStatus {
    Created,
    Running,
    Completed,
    Timeout,
    Permission,
    Error,
}
impl RunStatus {
    const fn finished(state: State) -> Self {
        match state {
            State::Idle => Self::Completed,
            State::Running => Self::Running,
            State::Timeout => Self::Timeout,
            State::Permission => Self::Permission,
            State::Error => Self::Error,
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Timeout => "timeout",
            Self::Permission => "permission",
            Self::Error => "error",
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunReceipt<'a> {
    agent_id: SessionId,
    status: RunStatus,
    provider: &'a HarnessRef,
    cwd: Option<&'a Path>,
    title: Option<&'a SessionTitle>,
}
impl<'a> RunReceipt<'a> {
    fn new(info: &'a SessionInfo, status: RunStatus) -> Self {
        Self {
            agent_id: info.id,
            status,
            provider: &info.spec.harness,
            cwd: info.spec.cwd.as_deref(),
            title: info.title.as_ref(),
        }
    }
}
impl Row for RunReceipt<'_> {
    fn id(&self) -> String {
        self.agent_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT ID", "STATUS", "PROVIDER", "CWD", "TITLE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.agent_id.to_string(),
            self.status.label().to_owned(),
            self.provider.to_string(),
            self.cwd
                .map_or_else(|| "-".to_owned(), |cwd| cwd.display().to_string()),
            self.title
                .map_or_else(|| "-".to_owned(), ToString::to_string),
        ]
    }
}
pub(super) async fn run_started(
    presentation: Presentation,
    info: &SessionInfo,
    delivery: Delivered,
) -> Result<(), CliError> {
    let status = match delivery {
        Delivered::Started | Delivered::Steered => RunStatus::Running,
        Delivered::Queued { .. } => RunStatus::Created,
    };
    emit(presentation, &RunReceipt::new(info, status)).await
}
/// Import creates an idle session without acknowledging a new prompt turn.
pub(super) async fn imported(
    presentation: Presentation,
    info: &SessionInfo,
) -> Result<(), CliError> {
    emit(presentation, &RunReceipt::new(info, RunStatus::Created)).await
}
pub(super) async fn run_finished(
    presentation: Presentation,
    info: &SessionInfo,
    state: State,
) -> Result<(), CliError> {
    emit(
        presentation,
        &RunReceipt::new(info, RunStatus::finished(state)),
    )
    .await
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum SendStatus {
    Queued,
    Sent,
    Completed,
    Timeout,
    Permission,
    Error,
}
impl SendStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Sent => "sent",
            Self::Completed => "completed",
            Self::Timeout => "timeout",
            Self::Permission => "permission",
            Self::Error => "error",
        }
    }
    const fn message(self) -> &'static str {
        match self {
            Self::Queued => "Message queued for delivery",
            Self::Sent => "Message sent, not waiting for completion",
            Self::Completed => "Agent completed processing the message",
            Self::Timeout => "Timed out waiting for agent to finish",
            Self::Permission => "Agent is waiting for permission or an answer",
            Self::Error => "Agent finished with error",
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendReceipt {
    agent_id: SessionId,
    status: SendStatus,
    message: &'static str,
}
impl Row for SendReceipt {
    fn id(&self) -> String {
        self.agent_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT ID", "STATUS", "MESSAGE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.agent_id.to_string(),
            self.status.label().to_owned(),
            self.message.to_owned(),
        ]
    }
}
pub(super) async fn send_started(
    presentation: Presentation,
    agent_id: SessionId,
    delivery: Delivered,
) -> Result<(), CliError> {
    let status = match delivery {
        Delivered::Started | Delivered::Steered => SendStatus::Sent,
        Delivered::Queued { .. } => SendStatus::Queued,
    };
    emit(
        presentation,
        &SendReceipt {
            agent_id,
            status,
            message: status.message(),
        },
    )
    .await
}
pub(super) async fn send_finished(
    presentation: Presentation,
    result: WaitResult,
) -> Result<(), CliError> {
    let status = match result.status {
        State::Idle => SendStatus::Completed,
        State::Running => SendStatus::Sent,
        State::Timeout => SendStatus::Timeout,
        State::Permission => SendStatus::Permission,
        State::Error => SendStatus::Error,
    };
    emit(
        presentation,
        &SendReceipt {
            agent_id: result.agent_id,
            status,
            message: status.message(),
        },
    )
    .await
}
async fn emit<T: Row>(presentation: Presentation, result: &T) -> Result<(), CliError> {
    output::record(presentation, result).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_run_and_send_are_distinct_from_idle_wait() {
        assert_eq!(RunStatus::finished(State::Idle).label(), "completed");
        assert_eq!(
            serde_json::to_string(&RunStatus::finished(State::Idle)).unwrap(),
            "\"completed\""
        );
        assert_eq!(State::Idle.label(), "idle");
        assert_eq!(
            SendStatus::Completed.message(),
            "Agent completed processing the message"
        );
    }
}
