//! Admission follows owned work, not the caller future or async join handle.
use super::*;
use futures_util::poll;
use tokio::sync::{Semaphore, mpsc, oneshot};
use vessel_proto::workspace::WorkspaceRoot;

#[tokio::test]
async fn cancelling_queued_callers_cannot_make_an_unbounded_command_queue() {
    let (commands, mut received) = mpsc::channel(MAX_ACTIVE);
    let client = ProjectPulls {
        commands,
        admission: Arc::new(Semaphore::new(MAX_ACTIVE)),
        stop: CancellationToken::new(),
    };
    let project = ProjectId::new();
    for _ in 0..MAX_ACTIVE {
        let mut request = Box::pin(client.read(project, None, Mode::Cached));
        assert!(poll!(&mut request).is_pending());
        drop(request);
    }
    assert_eq!(received.len(), MAX_ACTIVE);
    assert_eq!(client.admission.available_permits(), 0);
    assert!(matches!(
        &*client.read(project, None, Mode::Cached).await.unwrap_err(),
        Error::Busy
    ));
    while let Ok(request) = received.try_recv() {
        request.answer(Err(Arc::new(Error::Stopped)));
    }
    assert_eq!(client.admission.available_permits(), MAX_ACTIVE);
    client.stop.cancel();
    assert!(matches!(
        &*client.read(project, None, Mode::Cached).await.unwrap_err(),
        Error::Stopped
    ));
}

struct RootGuard {
    permit: Admission,
    ended: Option<oneshot::Sender<()>>,
}
impl Drop for RootGuard {
    fn drop(&mut self) {
        assert_eq!(Arc::strong_count(&self.permit), 1);
        if let Some(ended) = self.ended.take() {
            let _gone = ended.send(());
        }
    }
}

#[test]
fn cancelling_a_queued_blocking_root_open_keeps_its_permit_until_actual_exit() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let fixture = Fixture::new().await;
        let project = fixture.project("app", true).await;
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
        let permit = Arc::new(Arc::clone(&permits).try_acquire_owned().unwrap());
        let (release, wait) = std::sync::mpsc::channel();
        let (started, running) = oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            // A channel barrier, not a timing sleep. Dropping the sender
            // on an assertion failure also releases the blocking thread.
            let _released = wait.recv();
        });
        running.await.unwrap();
        let (ended, finished) = oneshot::channel();
        let keep = RootGuard {
            permit,
            ended: Some(ended),
        };
        let mut resolving = Box::pin(fixture.workspaces.resolve_git_guarded(
            WorkspaceRoot::Project {
                project: project.id,
            },
            keep,
        ));
        assert!(poll!(&mut resolving).is_pending());
        drop(resolving);
        assert_eq!(permits.available_permits(), MAX_ACTIVE - 1);
        release.send(()).unwrap();
        blocker.await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), finished)
            .await
            .expect("queued root open must actually exit")
            .unwrap();
        assert_eq!(permits.available_permits(), MAX_ACTIVE);
        fixture.finish().await;
    });
}

#[tokio::test]
async fn stop_refuses_new_calls_and_drains_the_owned_actor() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    assert!(fixture.read(project.id, Mode::Cached).await.is_ok());
    fixture.stop.cancel();
    assert!(matches!(
        &*fixture.read(project.id, Mode::Cached).await.unwrap_err(),
        Error::Stopped
    ));
    fixture.tasks.close();
    fixture.tasks.wait().await;
    assert!(fixture.client.admission.is_closed());
    assert_eq!(fixture.client.admission.available_permits(), MAX_ACTIVE);
}
