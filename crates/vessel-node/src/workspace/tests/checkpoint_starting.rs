//! Blocking admission probes keep their bounded capacity after client loss.
use super::*;
use crate::workspace::metadata::ROOT_METADATA_WORKERS;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_cancelled_start_probes_retain_capacity_until_real_metadata_io_finishes() {
    let fixture = Fixture::new().await;
    let mut callers = Vec::new();
    let mut releases = Vec::new();
    for _worker in 0..ROOT_METADATA_WORKERS {
        let workspaces = Arc::clone(&fixture.workspaces);
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        callers.push(tokio::spawn(async move {
            workspaces
                .root_metadata(move || {
                    entered.send(()).unwrap();
                    held.recv().unwrap();
                    Ok(())
                })
                .await
        }));
        ready.await.unwrap();
        releases.push(release);
    }
    for caller in callers {
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
    }
    assert!(matches!(
        fixture.workspaces.root_metadata(|| Ok(())).await,
        Err(WorkspaceError::MetadataBusy)
    ));
    fixture.stop.cancel();
    assert!(matches!(
        fixture.workspaces.root_metadata(|| Ok(())).await,
        Err(WorkspaceError::Stopping)
    ));
    for release in releases {
        release.send(()).unwrap();
    }
    let _closed = fixture.tasks.close();
    fixture.tasks.wait().await;
    fixture.store.close().await;
}
