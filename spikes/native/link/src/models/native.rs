//! Bound dispatched model reads without growing worker count.
//! The deadline starts at dispatch; the shared actor owns its startup and
//! fleet-list waits, which this worker does not independently time out.
use super::{Answer, Ask, ModelsError};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::models::page::{Listed, Snapshot};
use vessel_client::{CallError, NodeClient, Unary};
use vessel_proto::{ListModels, ListProfiles, agents::ListAgents};

const MAX_READS: usize = 4;
const READ_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_READS)))
    }
}
impl Hub {
    pub(crate) async fn ask(
        &self,
        client: Option<NodeClient>,
        ask: Ask,
        events: mpsc::Sender<crate::Event>,
    ) {
        let Some(client) = client else {
            emit(&events, ask.refused(ModelsError::Offline)).await;
            return;
        };
        let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
            emit(&events, ask.refused(ModelsError::Busy)).await;
            return;
        };
        drop(tokio::spawn(async move {
            let _permit = permit;
            let result = tokio::time::timeout(READ_TIMEOUT, read(&client))
                .await
                .unwrap_or(Err(ModelsError::Timeout));
            emit(
                &events,
                Answer {
                    machine: ask.machine,
                    ticket: ask.ticket,
                    result,
                },
            )
            .await;
        }));
    }
}
async fn emit(events: &mpsc::Sender<crate::Event>, answer: Answer) {
    let _closed = events.send(crate::Event::Models(answer)).await;
}
async fn read(client: &NodeClient) -> Result<Snapshot, ModelsError> {
    let routes = listed(client, ListModels).await?;
    let profiles = listed(client, ListProfiles { harness: None }).await?;
    let agents = match listed(client, ListAgents {}).await? {
        Listed::Available(agents) => agents,
        Listed::OlderNode => Vec::new(),
    };
    Ok(Snapshot {
        node: client.node().id,
        routes,
        profiles,
        agents,
    })
}

// The typed client owns admission before any frame/id is sent. Only its
// definite older-protocol refusal is an absent list; pending/unassigned
// contracts and actual failures retain their typed error instead of emptiness.
async fn listed<R, T>(client: &NodeClient, request: R) -> Result<Listed<T>, ModelsError>
where
    R: Unary<Response = Vec<T>>,
{
    match client.call(request).await {
        Ok(rows) => Ok(Listed::Available(rows)),
        Err(CallError::NotInProtocol { .. }) => Ok(Listed::OlderNode),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
