//! The node's [`Runner`]: runs made through the service
//! (`service::scheduled`), which the scheduler holds weakly so the
//! service (which holds the scheduler's handle) is not kept alive by its
//! own scheduler.

use std::sync::{Arc, Weak};

use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::SessionId;
use vessel_proto::automation::{RunError, RunErrorCode, RunOutcome, ScheduleRun};

use vessel_store::Store;

use super::{Command, FollowSender, Job, Made, Parts, Runner, SystemClock, start};
use crate::guard::Guarded;

/// Reconcile exact receipts before the scheduler reads its book or
/// becomes able to dispatch another due run.
pub(crate) async fn start_node(
    service: &Arc<Guarded>,
    commands: mpsc::Receiver<Command>,
    store: &Store,
    tasks: &TaskTracker,
    stop: &CancellationToken,
    ended: broadcast::Receiver<SessionId>,
) {
    service.0.recover_schedule_workspaces().await;
    let schedule_cleanup = Arc::downgrade(service);
    drop(tasks.spawn(async move {
        if let Some(service) = schedule_cleanup.upgrade() {
            service.0.watch_deferred_schedule_workspaces().await;
        }
    }));
    start(
        commands,
        Parts {
            runner: Arc::new(NodeRunner::new(service)),
            clock: SystemClock,
            store: store.clone(),
            tasks: tasks.clone(),
            stop: stop.clone(),
            ended,
        },
    )
    .await;
}

/// Runs made through the node's service.
#[derive(Debug)]
pub(crate) struct NodeRunner {
    service: Weak<Guarded>,
}

impl NodeRunner {
    pub(crate) fn new(service: &Arc<Guarded>) -> Self {
        Self {
            service: Arc::downgrade(service),
        }
    }
}

impl Runner for NodeRunner {
    async fn make(&self, job: Job) -> Made {
        let Some(service) = self.service.upgrade() else {
            return Made::outcome(RunOutcome::Failed {
                error: RunError {
                    code: RunErrorCode::Unavailable,
                    message: "the node is stopping".to_owned(),
                },
                session: None,
            });
        };
        service.0.make_run(&job).await
    }

    async fn follow(&self, job: Job, session: SessionId, follow: FollowSender) {
        if let Some(service) = self.service.upgrade() {
            service.0.follow_run(job, session, follow).await;
        }
    }

    async fn settle(&self, run: &ScheduleRun) -> Option<RunOutcome> {
        let service = self.service.upgrade()?;
        service.0.settle_run(run).await
    }

    fn agent_alive(&self, session: SessionId) -> bool {
        self.service
            .upgrade()
            .is_some_and(|service| service.0.agents.scopes(session).is_some())
    }
}
