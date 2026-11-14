//! Usage scans have their own admission bound, including abandoned jobs.
//!
//! The generic reader's idle connection cache is not a concurrency bound.
//! Keep each permit inside the blocking closure: cancelling its async caller
//! must not admit another scan while the old one is running or still queued.
use crate::{Store, StoreError};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::usage::{QueryUsage, UsageReport};

pub(super) const MAX_READS: usize = 2;

impl Store {
    /// Reported usage as of `now`; no provider or credential I/O.
    ///
    /// # Errors
    /// At most two scans may be queued or running across this store's clones;
    /// an excess request returns [`StoreError::UsageBusy`] without queueing.
    /// The database read may also fail or a recorded report may be corrupt.
    pub async fn usage(
        &self,
        request: QueryUsage,
        now: DateTime<Utc>,
    ) -> Result<UsageReport, StoreError> {
        self.usage_read(move |reader| {
            reader.with(|connection| crate::usage::query(connection, &request, now))
        })
        .await
    }

    async fn usage_read<T: Send + 'static>(
        &self,
        read: impl FnOnce(&crate::reader::Reader) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let permit = Arc::clone(&self.usage_reads)
            .try_acquire_owned()
            .map_err(StoreError::UsageBusy)?;
        self.read(move |reader| {
            let _permit = permit;
            read(reader)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::{Future as _, poll_fn},
        sync::mpsc,
        task::Poll,
    };
    use tokio::sync::oneshot;
    use vessel_proto::{
        ErrorCode,
        usage::{UsagePeriod, UsageScope},
    };

    #[test]
    fn abandoned_running_and_queued_scans_keep_their_permits_until_the_closures_end() {
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
                        .usage_read(move |_reader| {
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
                let mut second = Box::pin(store.usage_read(move |_reader| {
                    queued_started.send(()).unwrap();
                    queued_blocked.recv().unwrap();
                    queued_ended.send(()).unwrap();
                    Ok(())
                }));
                // One poll admits the second closure into the blocked worker pool.
                assert!(poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await);
                first.abort();
                assert!(first.await.unwrap_err().is_cancelled());
                drop(second);
                assert_eq!(store.usage_reads.available_permits(), 0);
                let error = store.usage(query(), Utc::now()).await.unwrap_err();
                assert!(matches!(&error, StoreError::UsageBusy(_)));
                assert_eq!(error.code(), ErrorCode::Unavailable);
                release.send(()).unwrap();
                ending.await.unwrap();
                queued_starting.await.unwrap();
                assert_eq!(store.usage_reads.available_permits(), 1);
                queued_release.send(()).unwrap();
                queued_ending.await.unwrap();
                // A real scan queues behind completion of the abandoned closure.
                let report = store.usage(query(), Utc::now()).await.unwrap();
                assert_eq!(report.totals.reported.get(), 0);
                assert_eq!(store.usage_reads.available_permits(), MAX_READS);
                store.close().await;
            })
            .await
            .expect("the usage admission fixture stalled");
        });
    }

    fn query() -> QueryUsage {
        QueryUsage {
            scope: UsageScope::Machine,
            period: UsagePeriod::Day,
            after: None,
        }
    }
}
