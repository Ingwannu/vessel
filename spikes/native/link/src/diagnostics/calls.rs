//! The bounded read never asks agent.status, which can spawn discovery.
use super::{Answer, Ask, MAX_PARALLEL_HOSTS};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use vessel_app_view::diagnostics::collection::{HostResult, NodeFacts, Problem};
use vessel_client::{CallError, NodeClient};

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
        let Ok(client) = self.client(ask.machine.as_str()).cloned() else {
            self.emit(crate::Event::Diagnostics(ask.refused(Problem::Offline)))
                .await;
            return;
        };
        let Ok(permit) = Arc::clone(&self.diagnostic_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Diagnostics(ask.refused(Problem::Busy)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let _permit = permit;
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result: Ok(collect(&client).await),
            };
            let _closed = events.send(crate::Event::Diagnostics(answer)).await;
        }));
    }
}
async fn collect(client: &NodeClient) -> HostResult {
    let admitted = client.node().id;
    let facts = match tokio::time::timeout(NODE_INFO_DEADLINE, client.node_info()).await {
        Ok(Ok(info)) if info.id == admitted => Ok(NodeFacts {
            node: info.id,
            protocol: client.protocol(),
            state: info.state,
        }),
        Ok(Ok(_)) => Err(Problem::InvalidReply),
        Ok(Err(error)) => Err(problem(error)),
        Err(_elapsed) => Err(Problem::TimedOut),
    };
    HostResult { admitted, facts }
}
fn problem(error: CallError) -> Problem {
    match error {
        CallError::Disconnected => Problem::Offline,
        CallError::Remote(error) => match error.code {
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
        },
        CallError::NotInProtocol { .. } => Problem::Unsupported,
        CallError::UnexpectedReply(_) => Problem::InvalidReply,
        CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}
#[cfg(test)]
mod tests;
