//! The bounded read never asks agent.status, which can spawn discovery.
use super::{Answer, Ask, MAX_PARALLEL_HOSTS, Message};
use crate::connections::{RequestFailure, SelectedConnection};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_app_view::diagnostics::DaemonSnapshot;
use vessel_app_view::diagnostics::collection::{HostResult, NodeFacts, Problem};
use vessel_client::{CallError, DiagnosticError};

/// A basic node-info round trip cannot hold a diagnostic row indefinitely.
const NODE_INFO_DEADLINE: Duration = Duration::from_secs(10);

pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_PARALLEL_HOSTS)))
    }
}
impl crate::State {
    pub(crate) async fn diagnostic(&self, ask: Ask) {
        let selected = match self.selected_connection(&ask.context) {
            Ok(selected) => selected,
            Err(why) => {
                self.emit(crate::Event::Diagnostics(Message::Answered(
                    ask.refused(refusal(why)),
                )))
                .await;
                return;
            }
        };
        let Ok(permit) = Arc::clone(&self.diagnostic_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Diagnostics(Message::Answered(
                ask.refused(Problem::Busy),
            )))
            .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let _permit = permit;
            let result = collect(&selected, &ask.context).await;
            publish(&selected, &events, ask, result).await;
        }));
    }
}
/// A retired/closed guarded send still settles the original ticket with a
/// closed refusal. Keep the bounded host permit through that fallback, like
/// history delivery: dropping it could strand the visible row or grow tasks.
async fn publish(
    selected: &SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    ask: Ask,
    result: Result<HostResult, Problem>,
) {
    use crate::connections::SendFailure;
    let answer = Answer {
        context: ask.context.clone(),
        ticket: ask.ticket,
        result,
        guard: Some(selected.delivery_guard()),
    };
    match selected
        .send_guarded(events, crate::Event::Diagnostics(Message::Answered(answer)))
        .await
    {
        Ok(()) | Err(SendFailure::ReceiverClosed) => {}
        Err(SendFailure::Refused(why)) => {
            let _closed = events
                .send(crate::Event::Diagnostics(Message::Answered(
                    ask.refused(refusal(why)),
                )))
                .await;
        }
    }
}
async fn collect(
    selected: &SelectedConnection,
    context: &ConnectionContext,
) -> Result<HostResult, Problem> {
    let (facts, daemon) = tokio::join!(basic(selected, context), daemon(selected));
    for failure in [
        facts.as_ref().err().copied(),
        match &daemon {
            DaemonSnapshot::Unavailable(why) => Some(*why),
            DaemonSnapshot::Report(_) => None,
        },
    ]
    .into_iter()
    .flatten()
    {
        if matches!(failure, Problem::ConnectionChanged | Problem::Offline) {
            return Err(failure);
        }
    }
    Ok(HostResult {
        admitted: context.node(),
        facts,
        daemon,
    })
}
async fn basic(
    selected: &SelectedConnection,
    context: &ConnectionContext,
) -> Result<NodeFacts, Problem> {
    match tokio::time::timeout(NODE_INFO_DEADLINE, selected.call(vessel_proto::GetNode {})).await {
        Ok(Ok(info)) if info.id == context.node() => Ok(NodeFacts {
            node: info.id,
            protocol: selected.protocol(),
            state: info.state,
        }),
        Ok(Ok(_)) => Err(Problem::InvalidReply),
        Ok(Err(error)) => Err(selected_problem(error)),
        Err(_elapsed) => Err(Problem::TimedOut),
    }
}
async fn daemon(selected: &SelectedConnection) -> DaemonSnapshot {
    match selected.diagnostics().await {
        Ok(Ok(report)) => DaemonSnapshot::Report(Arc::new(report)),
        Ok(Err(error)) => DaemonSnapshot::Unavailable(diagnostic_problem(error)),
        Err(error) => DaemonSnapshot::Unavailable(selected_problem(error)),
    }
}
fn diagnostic_problem(error: DiagnosticError) -> Problem {
    match error {
        DiagnosticError::Call(error) => problem(error),
        DiagnosticError::Identity { .. } => Problem::InvalidReply,
        DiagnosticError::Timeout(_) => Problem::TimedOut,
    }
}
fn selected_problem(error: RequestFailure) -> Problem {
    match error {
        RequestFailure::Refused(why) => refusal(why),
        RequestFailure::Call(error) => problem(error),
    }
}
fn refusal(why: Refusal) -> Problem {
    match why {
        Refusal::Changed => Problem::ConnectionChanged,
        Refusal::Closed => Problem::Offline,
    }
}
fn problem(error: CallError) -> Problem {
    match error {
        CallError::Disconnected => Problem::Offline,
        CallError::Remote(error) => {
            if error.code == vessel_proto::ErrorCode::Unavailable
                && let Some(vessel_proto::ErrorDetail::Diagnostics { reason }) = error.detail
            {
                return match reason {
                    vessel_proto::diagnostics::DiagnosticRefusal::Busy => Problem::Busy,
                    vessel_proto::diagnostics::DiagnosticRefusal::Stopping => Problem::Stopping,
                };
            }
            match error.code {
                vessel_proto::ErrorCode::Forbidden => Problem::Forbidden,
                vessel_proto::ErrorCode::Unsupported => Problem::Unsupported,
                vessel_proto::ErrorCode::InvalidRequest
                | vessel_proto::ErrorCode::Unauthorized
                | vessel_proto::ErrorCode::ProtocolMismatch
                | vessel_proto::ErrorCode::NotFound
                | vessel_proto::ErrorCode::Conflict
                | vessel_proto::ErrorCode::HarnessNotInstalled
                | vessel_proto::ErrorCode::SpawnFailed
                | vessel_proto::ErrorCode::Unavailable
                | vessel_proto::ErrorCode::Internal => Problem::Failed,
            }
        }
        CallError::NotInProtocol { .. } => Problem::Unsupported,
        CallError::UnassignedProtocol { .. } => Problem::Unavailable,
        CallError::UnexpectedReply(_) => Problem::InvalidReply,
        CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}
#[cfg(test)]
mod tests;
