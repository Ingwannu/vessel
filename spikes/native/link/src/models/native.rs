//! Bound dispatched model reads without growing worker count.
//! The deadline starts at dispatch; the shared actor owns its startup and
//! fleet-list waits, which this worker does not independently time out.
use super::{Answer, Ask, ModelsError};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::models::page::{Listed, Snapshot};
use vessel_client::NodeClient;
use vessel_proto::{ListModels, ListProfiles, Method, agents::ListAgents};

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
    let protocol = client.protocol();
    let routes = if protocol >= Method::ListModels.since() {
        Listed::Available(client.call(ListModels).await?)
    } else {
        Listed::OlderNode
    };
    let profiles = if protocol >= Method::ListProfiles.since() {
        Listed::Available(client.call(ListProfiles { harness: None }).await?)
    } else {
        Listed::OlderNode
    };
    let agents = if protocol >= Method::ListAgents.since() {
        client.call(ListAgents {}).await?
    } else {
        Vec::new()
    };
    Ok(Snapshot {
        routes,
        profiles,
        agents,
    })
}

#[cfg(test)]
mod tests;
