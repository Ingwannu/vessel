//! A disconnected caller cannot turn bounded scans into an unbounded worker queue.
use super::*;
use std::time::Duration;
use std::{
    future::{Future as _, poll_fn},
    sync::mpsc,
    task::Poll,
};
use tokio::sync::oneshot;

// Safety only: frames/channels establish ordering; expiry always fails.
const SAFETY: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);

async fn phase<T>(name: &str, wait: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, wait)
        .await
        .unwrap_or_else(|error| panic!("prompt admission {name} stalled: {error}"))
}

#[test]
fn abandoned_running_and_queued_scans_retain_admission_until_their_closures_end() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime
        .block_on(vessel_testkit::within(SAFETY, admission_lifetime()))
        .expect("the prompt-scan fixture stalled");
}

async fn admission_lifetime() {
    let temp = tempfile::tempdir().unwrap();
    let dir = crate::DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = phase("open", Store::open(&dir, crate::StoreConfig::default()))
        .await
        .unwrap()
        .store;
    let (started, starting) = oneshot::channel();
    let (release, blocked) = mpsc::channel();
    let (ended, ending) = oneshot::channel();
    let first_store = store.clone();
    let first = tokio::spawn(async move {
        first_store
            .prompt_read(move |_reader| {
                started.send(()).unwrap();
                blocked
                    .recv_timeout(PHASE)
                    .expect("running scan release stalled");
                ended.send(()).unwrap();
                Ok(())
            })
            .await
    });
    phase("running scan started", starting).await.unwrap();
    let (queued_started, queued_starting) = oneshot::channel();
    let (queued_release, queued_blocked) = mpsc::channel();
    let (queued_ended, queued_ending) = oneshot::channel();
    let mut second = Box::pin(store.prompt_read(move |_reader| {
        queued_started.send(()).unwrap();
        queued_blocked
            .recv_timeout(PHASE)
            .expect("queued scan release stalled");
        queued_ended.send(()).unwrap();
        Ok(())
    }));
    assert!(
        phase(
            "queued scan admitted",
            poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending()))
        )
        .await
    );
    first.abort();
    assert!(
        phase("cancelled waiter joined", first)
            .await
            .unwrap_err()
            .is_cancelled()
    );
    drop(second);
    assert_eq!(store.prompt_reads.available_permits(), 0);
    let error = phase("busy refusal", store.prompt_read(|_| Ok(())))
        .await
        .unwrap_err();
    assert!(matches!(&error, StoreError::PromptIndexBusy(_)));
    assert_eq!(error.code(), vessel_proto::ErrorCode::Unavailable);
    release.send(()).unwrap();
    phase("running closure ended", ending).await.unwrap();
    phase("queued closure started", queued_starting)
        .await
        .unwrap();
    assert_eq!(store.prompt_reads.available_permits(), 1);
    queued_release.send(()).unwrap();
    phase("queued closure ended", queued_ending).await.unwrap();
    phase("replacement scan", store.prompt_read(|_| Ok(())))
        .await
        .unwrap();
    assert_eq!(
        store.prompt_reads.available_permits(),
        crate::prompt_index::MAX_READS
    );
    phase("close", store.close()).await;
}
