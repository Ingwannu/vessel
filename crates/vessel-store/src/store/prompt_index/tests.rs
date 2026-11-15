//! A disconnected caller cannot turn bounded scans into an unbounded worker queue.
use super::*;
use std::{
    future::{Future as _, poll_fn},
    sync::mpsc,
    task::Poll,
};
use tokio::sync::oneshot;

#[test]
fn abandoned_running_and_queued_scans_retain_admission_until_their_closures_end() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let temp = tempfile::tempdir().unwrap();
            let dir = crate::DataDir::new(temp.path());
            dir.prepare().unwrap();
            let store = Store::open(&dir, crate::StoreConfig::default())
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
                        blocked.recv().unwrap();
                        ended.send(()).unwrap();
                        Ok(())
                    })
                    .await
            });
            starting.await.unwrap();
            let (queued_started, queued_starting) = oneshot::channel();
            let (queued_release, queued_blocked) = mpsc::channel();
            let (queued_ended, queued_ending) = oneshot::channel();
            let mut second = Box::pin(store.prompt_read(move |_reader| {
                queued_started.send(()).unwrap();
                queued_blocked.recv().unwrap();
                queued_ended.send(()).unwrap();
                Ok(())
            }));
            assert!(poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await);
            first.abort();
            assert!(first.await.unwrap_err().is_cancelled());
            drop(second);
            assert_eq!(store.prompt_reads.available_permits(), 0);
            let error = store.prompt_read(|_| Ok(())).await.unwrap_err();
            assert!(matches!(&error, StoreError::PromptIndexBusy(_)));
            assert_eq!(error.code(), vessel_proto::ErrorCode::Unavailable);
            release.send(()).unwrap();
            ending.await.unwrap();
            queued_starting.await.unwrap();
            assert_eq!(store.prompt_reads.available_permits(), 1);
            queued_release.send(()).unwrap();
            queued_ending.await.unwrap();
            store.prompt_read(|_| Ok(())).await.unwrap();
            assert_eq!(
                store.prompt_reads.available_permits(),
                crate::prompt_index::MAX_READS
            );
            store.close().await;
        })
        .await
        .expect("the prompt-scan fixture stalled");
    });
}
