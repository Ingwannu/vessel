//! Explicit scripted commit backend: timing fixtures, never product persistence.

use std::collections::VecDeque;
use std::sync::Mutex;

use vessel_proto::{
    WorkspaceId,
    workspace_labels::{
        Assignment, CatalogDelta, CatalogMode, LabelColor, LabelCount, WorkspaceLabelFailure,
    },
};

use super::*;

const SHUTDOWN_TEST_LIMIT: Duration = Duration::from_secs(30);

struct Gate {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

struct Step<T> {
    result: Result<T, StoreError>,
    gate: Option<Gate>,
}

impl<T> Step<T> {
    async fn finish(self) -> Result<T, StoreError> {
        if let Some(gate) = self.gate {
            gate.entered.send(()).unwrap();
            gate.release.await.unwrap();
        }
        self.result
    }
}

enum Reply {
    Assign(Step<Committed<WorkspaceLabelAssigned>>),
    Update(Step<Committed<WorkspaceLabelUpdated>>),
    Inspect(Step<WorkspaceLabelDeleted>),
    Delete(Step<Committed<WorkspaceLabelDeleted>>),
}

struct Script(Mutex<VecDeque<Reply>>);

impl Script {
    fn new(replies: impl IntoIterator<Item = Reply>) -> Self {
        Self(Mutex::new(replies.into_iter().collect()))
    }
    fn next(&self) -> Reply {
        self.0.lock().unwrap().pop_front().unwrap()
    }
}

impl Backend for Script {
    async fn catalog(&self) -> Result<Vec<Label>, StoreError> {
        Ok(Vec::new())
    }
    async fn assign(
        &self,
        _request: AssignWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelAssigned>, StoreError> {
        let Reply::Assign(step) = self.next() else {
            panic!("expected assignment");
        };
        step.finish().await
    }
    async fn update(
        &self,
        _request: UpdateWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelUpdated>, StoreError> {
        let Reply::Update(step) = self.next() else {
            panic!("expected update");
        };
        step.finish().await
    }
    async fn inspect(
        &self,
        _request: InspectWorkspaceLabelDelete,
    ) -> Result<WorkspaceLabelDeleted, StoreError> {
        let Reply::Inspect(step) = self.next() else {
            panic!("expected inspection");
        };
        step.finish().await
    }
    async fn delete(
        &self,
        _request: DeleteWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelDeleted>, StoreError> {
        let Reply::Delete(step) = self.next() else {
            panic!("expected deletion");
        };
        step.finish().await
    }
}

fn label(name: &str) -> Label {
    Label {
        name: name.parse().unwrap(),
        color: LabelColor::Sky,
    }
}
fn assign() -> AssignWorkspaceLabel {
    AssignWorkspaceLabel {
        workspace: WorkspaceId::random(),
        label: label("Ready"),
        assignment: Assignment::Assigned,
    }
}
fn assigned() -> Committed<WorkspaceLabelAssigned> {
    Committed {
        result: WorkspaceLabelAssigned {
            label: label("Ready"),
            labels: vec!["Ready".parse().unwrap()],
        },
        delta: Some(CatalogDelta::Upsert {
            label: label("Ready"),
            previous: None,
        }),
    }
}
fn ready<T>(result: Result<T, StoreError>) -> Step<T> {
    Step { result, gate: None }
}
fn observe(cursor: CatalogCursor) -> ListWorkspaceLabels {
    ListWorkspaceLabels {
        cursor: Some(cursor),
        read: CatalogRead::Observe,
    }
}

#[tokio::test]
async fn caller_cancellation_after_acceptance_cannot_skip_committed_publication() {
    let (entered, started) = oneshot::channel();
    let (release, waiting) = oneshot::channel();
    let service = Labels::start(Script::new([Reply::Assign(Step {
        result: Ok(assigned()),
        gate: Some(Gate {
            entered,
            release: waiting,
        }),
    })]))
    .await
    .unwrap();
    let initial = service.list(ListWorkspaceLabels::default()).await.unwrap();
    let caller = service.clone();
    let pending = tokio::spawn(async move { caller.assign(assign()).await });
    started.await.unwrap();
    // The request is gone while the durable backend still owns its commit.
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    assert_eq!(*service.changed.borrow(), initial.head);
    release.send(()).unwrap();
    let current = service
        .list(ListWorkspaceLabels {
            cursor: Some(initial.head),
            read: CatalogRead::Now,
        })
        .await
        .unwrap();
    assert_eq!(initial.head.after.next(), Some(current.head.after));
    assert!(matches!(current.data, CatalogMode::Changes { updates, .. } if updates.len() == 1));
    assert_eq!(
        service
            .list(ListWorkspaceLabels::default())
            .await
            .unwrap()
            .data,
        CatalogMode::Snapshot {
            labels: vec![label("Ready")]
        }
    );
}

#[tokio::test(start_paused = true)]
async fn observation_has_no_lost_wake_and_no_repeated_immediate_poll_after_a_commit() {
    let service = Labels::start(Script::new([Reply::Assign(ready(Ok(assigned())))]))
        .await
        .unwrap();
    let initial = service.list(ListWorkspaceLabels::default()).await.unwrap();
    let caller = service.clone();
    let pending = tokio::spawn(async move { caller.list(observe(initial.head)).await });
    tokio::task::yield_now().await;
    let _assigned = service.assign(assign()).await.unwrap();
    let current = pending.await.unwrap().unwrap();
    assert_eq!(initial.head.after.next(), Some(current.head.after));
    let started = tokio::time::Instant::now();
    let unchanged = service.list(observe(current.head)).await.unwrap();
    assert_eq!(
        tokio::time::Instant::now().duration_since(started),
        OBSERVE_LIMIT
    );
    assert_eq!(unchanged.head, current.head);
    assert!(matches!(unchanged.data, CatalogMode::Changes { updates, .. } if updates.is_empty()));
}

#[tokio::test]
async fn failed_and_noop_writes_publish_nothing_and_inspection_does_not_mutate() {
    let failure = StoreError::WorkspaceLabelRefused(WorkspaceLabelFailure::NameTaken);
    let count = WorkspaceLabelDeleted {
        affected: LabelCount::new(0),
    };
    let service = Labels::start(Script::new([
        Reply::Update(ready(Err(failure))),
        Reply::Inspect(ready(Ok(count))),
        Reply::Delete(ready(Ok(Committed {
            result: count,
            delta: None,
        }))),
    ]))
    .await
    .unwrap();
    let initial = service.list(ListWorkspaceLabels::default()).await.unwrap();
    assert!(matches!(
        service
            .update(UpdateWorkspaceLabel {
                name: "Ready".parse().unwrap(),
                new_name: None,
                color: Some(LabelColor::Red),
            })
            .await,
        Err(StoreError::WorkspaceLabelRefused(
            WorkspaceLabelFailure::NameTaken
        ))
    ));
    assert_eq!(
        service
            .inspect(InspectWorkspaceLabelDelete {
                name: "Ready".parse().unwrap()
            })
            .await
            .unwrap(),
        count
    );
    assert_eq!(
        service
            .delete(DeleteWorkspaceLabel {
                name: "Ready".parse().unwrap()
            })
            .await
            .unwrap(),
        count
    );
    assert_eq!(
        service.list(ListWorkspaceLabels::default()).await.unwrap(),
        initial
    );
}

#[tokio::test]
async fn cancelled_observer_releases_its_watch_receiver() {
    let journal = Journal::new(Vec::new());
    let (changed, receiver) = watch::channel(journal.cursor());
    let (commands, incoming) = mpsc::channel(COMMANDS);
    let service = Labels {
        commands,
        changed: receiver,
    };
    let actor = tokio::spawn(run(Script::new([]), journal, changed.clone(), incoming));
    let initial = service.list(ListWorkspaceLabels::default()).await.unwrap();
    let baseline = changed.receiver_count();
    {
        // Poll once explicitly: the pending snapshot owns one cloned watch.
        let pending = service.list(observe(initial.head));
        tokio::pin!(pending);
        tokio::select! {
            biased;
            _ = &mut pending => panic!("an unchanged catalog waits"),
            () = tokio::task::yield_now() => {}
        }
        assert_eq!(Some(changed.receiver_count()), baseline.checked_add(1));
    }
    assert_eq!(changed.receiver_count(), baseline);
    drop(service);
    actor.await.unwrap();
    assert_eq!(changed.receiver_count(), 0);
}

#[tokio::test]
async fn normal_shutdown_drains_a_cancelled_accepted_commit_and_refuses_commands_after_its_marker()
{
    let check = async {
        let (entered, started) = oneshot::channel();
        let (release, waiting) = oneshot::channel();
        let service = Labels::start(Script::new([Reply::Assign(Step {
            result: Ok(assigned()),
            gate: Some(Gate {
                entered,
                release: waiting,
            }),
        })]))
        .await
        .unwrap();
        let initial = service.list(ListWorkspaceLabels::default()).await.unwrap();
        let caller = service.clone();
        let pending = tokio::spawn(async move { caller.assign(assign()).await });
        started.await.unwrap();
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());

        // Poll close once while the writer is held: the marker is queued after
        // the accepted mutation, but has not claimed that it finished draining.
        let closing = service.close();
        tokio::pin!(closing);
        tokio::select! {
            biased;
            _ = &mut closing => panic!("the durable write is still held"),
            () = tokio::task::yield_now() => {}
        }
        let caller = service.clone();
        let later = tokio::spawn(async move { caller.assign(assign()).await });
        tokio::task::yield_now().await;
        release.send(()).unwrap();
        closing.await.unwrap();
        assert_eq!(
            initial.head.after.next(),
            Some(service.changed.borrow().after)
        );
        assert!(matches!(later.await.unwrap(), Err(StoreError::Closed)));
        assert!(matches!(
            service.list(ListWorkspaceLabels::default()).await,
            Err(StoreError::Closed)
        ));
    };
    tokio::time::timeout(SHUTDOWN_TEST_LIMIT, check)
        .await
        .unwrap();
}
