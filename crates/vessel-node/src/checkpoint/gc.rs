//! One serial collector shares the actual capture/restore worker budget.
//! Previous-process worker rows are distinct from live captures. Every release
//! follows actual cleanup/verification and the existing writer's durable ack.
use super::{
    backend::reason,
    workers::{WorkerLease, Workers},
};
use crate::workspace::Workspaces;
use std::{sync::Arc, time::Duration};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::{
    WorktreeId,
    checkpoint::{CaptureId, CaptureKey, CheckpointError, FileImage},
};
use vessel_store::{
    Store,
    checkpoint::{CheckpointWrite, GarbageObject, OrphanedCapture},
};
const GC_INTERVAL: Duration = Duration::from_secs(60);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Object {
    Orphan(CaptureId),
    Garbage(GarbageObject),
}
enum Work {
    Clean {
        worktree: WorktreeId,
        capture: CaptureId,
    },
    Verify {
        key: CaptureKey,
        image: Box<FileImage>,
    },
}
enum Settlement {
    Capture(CaptureKey),
    Garbage(GarbageObject),
}
struct Collector {
    workers: Workers,
    after: Option<CaptureId>,
    held: Vec<(Object, Arc<WorkerLease>)>,
}
impl Collector {
    fn new(workers: Workers) -> Self {
        Self {
            workers,
            after: None,
            held: Vec::new(),
        }
    }
    async fn pass(
        &mut self,
        store: &Store,
        workspaces: &Arc<Workspaces>,
        stop: &CancellationToken,
    ) -> Result<(), CheckpointError> {
        let page = store
            .checkpoint_orphaned_page(self.after)
            .await
            .map_err(reason)?;
        for orphan in page.captures {
            if stop.is_cancelled() {
                return Ok(());
            }
            let key = orphan.key();
            let object = Object::Orphan(key.capture);
            if self.held.iter().any(|(held, _)| *held == object) {
                continue;
            }
            let work = match orphan {
                OrphanedCapture::Discard { key } => Work::Clean {
                    worktree: key.worktree,
                    capture: key.capture,
                },
                OrphanedCapture::Retained { key, checkpoint } => {
                    let selected = store
                        .checkpoint_selected(key.session, checkpoint)
                        .await
                        .map_err(reason)?;
                    if selected.key != key {
                        return Err(CheckpointError::StaleCapture);
                    }
                    Work::Verify {
                        key,
                        image: Box::new(selected.snapshot.image().clone()),
                    }
                }
            };
            self.execute(
                store,
                workspaces,
                stop,
                object,
                work,
                Settlement::Capture(key),
            )
            .await?;
        }
        self.after = page.next;
        for garbage in store.checkpoint_garbage().await.map_err(reason)? {
            if stop.is_cancelled() {
                return Ok(());
            }
            let work = match garbage.object {
                GarbageObject::Capture { worktree, capture } => Work::Clean { worktree, capture },
            };
            self.execute(
                store,
                workspaces,
                stop,
                Object::Garbage(garbage.object),
                work,
                Settlement::Garbage(garbage.object),
            )
            .await?;
        }
        Ok(())
    }
    async fn execute(
        &mut self,
        store: &Store,
        workspaces: &Arc<Workspaces>,
        stop: &CancellationToken,
        object: Object,
        work: Work,
        settlement: Settlement,
    ) -> Result<(), CheckpointError> {
        if self.held.iter().any(|(held, _)| *held == object) {
            return Ok(());
        }
        let lease = self.workers.claim()?;
        let actual = Arc::clone(&lease);
        let workspaces = Arc::clone(workspaces);
        let cancel = stop.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _lease = actual;
            match work {
                Work::Clean { worktree, capture } => {
                    workspaces.clean_checkpoint_storage(worktree, capture)
                }
                Work::Verify { key, image } => {
                    workspaces.verify_checkpoint_archive(key, &image, &cancel)
                }
            }
        })
        .await
        .unwrap_or(Err(CheckpointError::Unknown));
        if let Err(failure) = result {
            // Returned failure is not actual cleanup. Bound retained failures by
            // the same two leases; never repeatedly spend slots on one object.
            self.held.push((object, lease));
            tracing::warn!(reason=%failure,"checkpoint collector retained failed cleanup admission");
            return Ok(());
        }
        let write = match settlement {
            Settlement::Capture(key) => CheckpointWrite::CaptureSettled { key },
            Settlement::Garbage(object) => CheckpointWrite::GarbageCleaned { object },
        };
        let result = store.checkpoint_write(write).await.map_err(reason);
        drop(lease);
        result
    }
}
pub(super) fn start(
    store: Store,
    workspaces: Arc<Workspaces>,
    workers: Workers,
    tasks: &TaskTracker,
    stop: CancellationToken,
) {
    let _collector=tasks.spawn(async move {
        let mut collector=Collector::new(workers);
        let mut tick=tokio::time::interval(GC_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {biased;
                ()=stop.cancelled()=>break,
                _=tick.tick()=>{},
            }
            if let Err(failure)=collector.pass(&store,&workspaces,&stop).await {
                tracing::warn!(reason=%failure,"checkpoint collection deferred with accounting retained");
            }
        }
    });
}

#[cfg(test)]
pub(crate) async fn test_pass(
    store: &Store,
    workspaces: &Arc<Workspaces>,
) -> Result<(), CheckpointError> {
    Collector::new(Workers::default())
        .pass(store, workspaces, &CancellationToken::new())
        .await
}
