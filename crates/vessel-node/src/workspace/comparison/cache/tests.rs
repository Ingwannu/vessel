//! Barrier tests exercise actual tracked work, cancellation and shared delivery.
use super::core::{ForceReason, Subject};
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_proto::ProjectId;
use vessel_proto::workspace::{
    CompareGit, GitComparison, GitReadReason, GitWhitespace, WorkspaceRoot,
};
use vessel_workspace::paths::Root;

struct Fixture {
    _temp: tempfile::TempDir,
    key: Key,
    reader: Reader,
    tasks: TaskTracker,
    stop: CancellationToken,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        let project = ProjectId::random();
        let query = CompareGit {
            root: WorkspaceRoot::Project { project },
            comparison: GitComparison::Uncommitted,
            whitespace: GitWhitespace::Show,
            reason: GitReadReason::View,
        };
        Self {
            key: Key::new(project, root, query, None, Subject::Summary),
            _temp: temp,
            reader: Reader::default(),
            tasks: TaskTracker::new(),
            stop: CancellationToken::new(),
        }
    }
    fn owner(&self) -> Owner<'_> {
        Owner {
            tasks: &self.tasks,
            stop: &self.stop,
        }
    }
    async fn finish(&self) {
        assert!(self.tasks.close());
        self.tasks.wait().await;
    }
}

fn value(letter: char) -> Snapshot {
    Snapshot::Summary(GitCompared::Ready {
        data: DiffSummary {
            from: Some(letter.to_string().repeat(40).parse().unwrap()),
            files: vec![],
            more: false,
        },
    })
}

fn waiting(delivery: Delivery, after: bool) -> Receiver {
    match delivery {
        Delivery::Wait { reply, mode } => {
            assert_eq!(matches!(mode, WaitMode::After), after);
            reply
        }
        Delivery::Complete(_) => panic!("expected tracked work"),
    }
}

async fn reply(receiver: &mut Receiver) -> Reply {
    loop {
        if let Some(value) = receiver.borrow_and_update().clone() {
            return value;
        }
        receiver.changed().await.unwrap();
    }
}

#[tokio::test]
async fn a_forced_build_waits_for_cancellation_cleanup_and_fresh_waiters_share_the_next_one() {
    let fixture = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let (started, start) = tokio::sync::oneshot::channel();
    let (cleaned, cleanup) = tokio::sync::oneshot::channel();
    let old_calls = Arc::clone(&calls);
    let mut original = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                move |cancel| async move {
                    let _count = old_calls.fetch_add(1, Ordering::SeqCst);
                    started.send(cancel.clone()).unwrap();
                    cancel.cancelled().await;
                    cleanup.await.unwrap();
                    Ok(value('a'))
                },
            )
            .unwrap(),
        false,
    );
    let cancel = start.await.unwrap();
    let mut forced = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::Force(ForceReason::Mutation),
                fixture.owner(),
                |_| async { panic!("the old job still owns cleanup") },
            )
            .unwrap(),
        true,
    );
    let mut second = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::Force(ForceReason::Person),
                fixture.owner(),
                |_| async { panic!("the old job still owns cleanup") },
            )
            .unwrap(),
        true,
    );
    assert!(cancel.is_cancelled());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(original.borrow().is_none());
    cleaned.send(()).unwrap();
    let old = reply(&mut original).await.unwrap();
    assert!(
        matches!(old.as_ref(), Snapshot::Summary(GitCompared::Ready { data }) if data.from.as_ref().unwrap().as_str() == "a".repeat(40))
    );
    let _old_forced = reply(&mut forced).await.unwrap();
    let _old_second = reply(&mut second).await.unwrap();
    let (ready, pending) = tokio::sync::oneshot::channel();
    let fresh_calls = Arc::clone(&calls);
    let mut fresh = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                move |_| async move {
                    let _count = fresh_calls.fetch_add(1, Ordering::SeqCst);
                    pending.await.unwrap();
                    Ok(value('b'))
                },
            )
            .unwrap(),
        false,
    );
    let mut joined = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                |_| async { panic!("identical readers share") },
            )
            .unwrap(),
        false,
    );
    ready.send(()).unwrap();
    let fresh = reply(&mut fresh).await.unwrap();
    let joined = reply(&mut joined).await.unwrap();
    assert!(Arc::ptr_eq(&fresh, &joined));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    fixture.finish().await;
}

#[tokio::test]
async fn dropping_every_external_receiver_does_not_abandon_the_owned_build() {
    let fixture = Fixture::new();
    let (started, start) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let pending = fixture
        .reader
        .prepare(
            fixture.key.clone(),
            ReadReason::View,
            fixture.owner(),
            move |_| async move {
                started.send(()).unwrap();
                released.await.unwrap();
                Ok(value('a'))
            },
        )
        .unwrap();
    start.await.unwrap();
    drop(pending);
    assert!(!fixture.tasks.is_empty());
    release.send(()).unwrap();
    fixture.finish().await;
    let result = fixture
        .reader
        .read(
            fixture.key.clone(),
            ReadReason::View,
            fixture.owner(),
            |_| async { panic!("completed value is retained") },
        )
        .await
        .unwrap();
    assert!(matches!(result, ReadStep::Complete(Ok(_))));
    assert!(fixture.reader.state.lock().unwrap().active.is_empty());
}

#[tokio::test]
async fn shared_failures_keep_one_typed_source_and_do_not_poison_a_retry() {
    let fixture = Fixture::new();
    let (release, released) = tokio::sync::oneshot::channel();
    let mut first = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                move |_| async move {
                    released.await.unwrap();
                    Err(WorkspaceError::NotGit)
                },
            )
            .unwrap(),
        false,
    );
    let mut other = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                |_| async { panic!("shared failure") },
            )
            .unwrap(),
        false,
    );
    release.send(()).unwrap();
    let first = reply(&mut first).await.unwrap_err();
    let other = reply(&mut other).await.unwrap_err();
    assert!(Arc::ptr_eq(&first, &other));
    assert!(matches!(first.as_ref(), WorkspaceError::NotGit));
    let retried = fixture
        .reader
        .read(
            fixture.key.clone(),
            ReadReason::View,
            fixture.owner(),
            |_| async { Ok(value('b')) },
        )
        .await
        .unwrap();
    assert!(matches!(retried, ReadStep::Complete(Ok(_))));
    fixture.finish().await;
}

#[tokio::test]
async fn the_write_guard_retires_a_read_that_started_between_its_two_boundaries() {
    let fixture = Fixture::new();
    let guard =
        fixture
            .reader
            .mutating(fixture.key.query.root, fixture.key.project, Change::Working);
    let (started, start) = tokio::sync::oneshot::channel();
    let mut during = waiting(
        fixture
            .reader
            .prepare(
                fixture.key.clone(),
                ReadReason::View,
                fixture.owner(),
                move |cancel| async move {
                    started.send(cancel.clone()).unwrap();
                    cancel.cancelled().await;
                    Ok(value('a'))
                },
            )
            .unwrap(),
        false,
    );
    let token = start.await.unwrap();
    drop(guard);
    assert!(token.is_cancelled());
    let _outdated = reply(&mut during).await.unwrap();
    let fresh = fixture
        .reader
        .read(
            fixture.key.clone(),
            ReadReason::View,
            fixture.owner(),
            |_| async { Ok(value('b')) },
        )
        .await
        .unwrap();
    assert!(matches!(fresh, ReadStep::Complete(Ok(_))));
    fixture.finish().await;
}

#[tokio::test]
async fn the_public_read_requests_revalidation_only_after_the_old_cleanup_finishes() {
    let fixture = Fixture::new();
    let (started, start) = tokio::sync::oneshot::channel();
    let (cleaned, cleanup) = tokio::sync::oneshot::channel();
    let original = fixture.reader.read(
        fixture.key.clone(),
        ReadReason::View,
        fixture.owner(),
        move |cancel| async move {
            started.send(cancel.clone()).unwrap();
            cancel.cancelled().await;
            cleanup.await.unwrap();
            Ok(value('a'))
        },
    );
    tokio::pin!(original);
    let token = tokio::select! {
        biased;
        result = &mut original => panic!("the barrier has not released: {result:?}"),
        token = start => token.unwrap(),
    };
    let forced = fixture.reader.read(
        fixture.key.clone(),
        ReadReason::Force(ForceReason::Person),
        fixture.owner(),
        |_| async { panic!("fresh work waits for retry") },
    );
    tokio::pin!(forced);
    tokio::select! {
        biased;
        result = &mut forced => panic!("cleanup has not finished: {result:?}"),
        () = token.cancelled() => {}
    }
    cleaned.send(()).unwrap();
    let (original, forced) = tokio::join!(original, forced);
    assert!(matches!(original.unwrap(), ReadStep::Complete(Ok(_))));
    assert!(matches!(forced.unwrap(), ReadStep::Retry));
    fixture.finish().await;
}
