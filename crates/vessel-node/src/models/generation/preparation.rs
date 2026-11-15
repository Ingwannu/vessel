//! Keep the original admission and hard drain until actual file work settles.

use super::super::{KeyError, Models, config::Provider};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::OwnedSemaphorePermit, task::JoinError};
use tokio_util::task::TaskTrackerToken;
use vessel_llm::client::ApiKey;
use vessel_session::SessionHandle;
use vessel_workspace::{Git, project_file::metadata::ProjectStyles};

pub(super) struct Context {
    pub(super) cwd: PathBuf,
    pub(super) git: Option<Git>,
    pub(super) tracked: TaskTrackerToken,
}

struct Held {
    _slots: (OwnedSemaphorePermit, OwnedSemaphorePermit),
    tracked: TaskTrackerToken,
    #[cfg(test)]
    style_gate: Option<Arc<super::super::blocking_tests::Gate>>,
}

pub(super) struct Lease(Arc<Held>);
impl Lease {
    pub(super) fn acquire(
        models: &Models,
        handle: &SessionHandle,
        tracked: TaskTrackerToken,
    ) -> Option<Self> {
        Some(Self(Arc::new(Held {
            _slots: models.slot(handle)?,
            tracked,
            #[cfg(test)]
            style_gate: models
                .blocked_reads
                .gate(super::super::blocking_tests::Stage::Style),
        })))
    }

    pub(super) async fn styles(&self, folder: PathBuf) -> Result<ProjectStyles, JoinError> {
        let held = Arc::clone(&self.0);
        self.0
            .tracked
            .task_tracker()
            .spawn_blocking(move || {
                #[cfg(test)]
                if let Some(gate) = &held.style_gate {
                    gate.enter();
                }
                let _lease = held;
                ProjectStyles::read(&folder)
            })
            .await
    }

    pub(super) async fn key(
        &self,
        models: Arc<Models>,
        provider: Provider,
    ) -> Result<Option<ApiKey>, KeyError> {
        if provider.key.is_none() {
            return models.key(&provider).await;
        }
        let held = Arc::clone(&self.0);
        // Dropping this join handle detaches, never aborts: the sole key reader
        // must join its physical closure before releasing the original slots.
        self.0
            .tracked
            .task_tracker()
            .spawn(async move {
                let _lease = held;
                models.key(&provider).await
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        blocking_tests::Stage,
        tests::{Fixture, within},
    };
    use tokio_util::task::TaskTracker;

    #[tokio::test]
    async fn a_keyless_preparation_never_schedules_credential_work() {
        within(async {
            let fixture = Fixture::new().await;
            let (handle, records) = fixture.structured().await;
            let tasks = TaskTracker::new();
            let lease = Lease::acquire(&fixture.models, &handle, tasks.token()).unwrap();
            let held = fixture.models.blocked_reads.hold(Stage::Secret);
            let mut provider = fixture.models.route(&"sonnet".parse().unwrap()).unwrap().1;
            provider.key = None;
            {
                let key = lease.key(Arc::clone(&fixture.models), provider);
                tokio::pin!(key);
                assert!(matches!(
                    futures_util::poll!(key.as_mut()),
                    std::task::Poll::Ready(Ok(None))
                ));
            }
            assert_eq!(held.entries(), 0);
            drop(lease);
            handle.kill();
            let _ended = handle.ended().await;
            let _records = records.await.unwrap();
            fixture.stop().await;
        })
        .await;
    }
}
