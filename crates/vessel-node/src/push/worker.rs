//! One coalescing delivery loop per node. No polling, durable notification
//! replay or unbounded delivery queue. The inbox remains the durable truth.
use super::{Delivered, Push, PushError};
use chrono::Utc;
use futures_util::{StreamExt as _, stream};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::inbox::InboxCursor;
use vessel_store::{PushAttention, StoredPush};

pub(crate) async fn start(
    push: Arc<Push>,
    tasks: &TaskTracker,
    stop: CancellationToken,
) -> Result<(), vessel_store::StoreError> {
    // Subscribe before reading the watermark: an attention committed during
    // startup is either in the initial snapshot or wakes this receiver.
    let changes = push.store.inbox_changes();
    let head = push.store.inbox_head().await?;
    drop(tasks.spawn(run(push, changes, head, stop)));
    Ok(())
}
async fn run(
    push: Arc<Push>,
    mut changes: tokio::sync::watch::Receiver<()>,
    mut head: InboxCursor,
    stop: CancellationToken,
) {
    loop {
        tokio::select! { () = stop.cancelled() => break, wake = changes.changed() => { if wake.is_err() { break; } } }
        #[cfg(test)]
        self_pause(&push, &stop).await;
        tokio::select! { () = stop.cancelled() => break, () = tokio::time::sleep(Duration::from_millis(250)) => {} }
        let before = head;
        let signal = async {
            head = push.store.inbox_head().await?;
            push.pick(before, head).await
        }
        .await;
        match signal {
            Ok(Some(_attention)) => {}
            Ok(None) => continue,
            Err(_store) => {
                tracing::warn!(reason = "store", "push attention could not be read");
                continue;
            }
        }
        let subscriptions = match push.store.push_subscriptions(Utc::now()).await {
            Ok(subscriptions) => subscriptions,
            Err(_store) => {
                tracing::warn!(reason = "store", "push subscriptions could not be read");
                continue;
            }
        };
        let sent = stream::iter(subscriptions).for_each_concurrent(4, |subscription| {
            let push = Arc::clone(&push);
            async move {
                let result = push
                    .send_one(
                        subscription,
                        Burst {
                            after: before,
                            through: head,
                        },
                    )
                    .await;
                if let Err(error) = &result {
                    tracing::warn!(
                        reason = error.label(),
                        "push delivery failed; the inbox is still available"
                    );
                }
                #[cfg(test)]
                if let Some(hook) = &push.preparation
                    && hook.finished.send(result.ok().flatten()).await.is_err()
                {
                    tracing::debug!("push preparation observer closed");
                }
            }
        });
        tokio::select! { () = stop.cancelled() => break, () = sent => {} }
    }
}
impl Push {
    pub(super) async fn pick(
        &self,
        mut after: InboxCursor,
        through: InboxCursor,
    ) -> Result<Option<PushAttention>, vessel_store::StoreError> {
        loop {
            let candidates = self.store.pending_push(after, through).await?;
            for candidate in &candidates {
                if self.should_push(candidate.key.session) {
                    return Ok(Some(*candidate));
                }
            }
            let Some(last) = candidates.last() else {
                return Ok(None);
            };
            after = last.cursor;
        }
    }
    /// Select a current source in the original burst. If A was read/answered
    /// during preparation, B must still have its generic push opportunity.
    /// Every candidate is admitted by exact key/cursor and current recipient.
    async fn admission(
        &self,
        stored: &StoredPush,
        mut burst: Burst,
    ) -> Result<Option<PushAttention>, PushError> {
        while let Some(attention) = self.pick(burst.after, burst.through).await? {
            if self
                .store
                .push_admitted(stored, attention, Utc::now())
                .await?
                && self.should_push(attention.key.session)
            {
                return Ok(Some(attention));
            }
            burst.after = attention.cursor;
        }
        Ok(None)
    }
    /// Admission follows asynchronous preparation; an already admitted
    /// ciphertext cannot be recalled. The whole attempt remains time bounded.
    async fn send_one(
        &self,
        stored: StoredPush,
        burst: Burst,
    ) -> Result<Option<PushAttention>, PushError> {
        tokio::time::timeout(Duration::from_secs(10), self.send_within(stored, burst))
            .await
            .map_err(|_elapsed| super::delivery::DeliveryError::Timeout)?
    }
    async fn send_within(
        &self,
        stored: StoredPush,
        burst: Burst,
    ) -> Result<Option<PushAttention>, PushError> {
        if self.admission(&stored, burst).await?.is_none() {
            return Ok(None);
        }
        let crypto = self.crypto().await?;
        let subscription = stored.subscription.clone();
        let key = Arc::clone(&crypto);
        let request = tokio::task::spawn_blocking(move || key.encrypt(&subscription))
            .await
            .map_err(PushError::Task)??;
        #[cfg(test)]
        self.prepared(super::PreparedStage::Crypto).await;
        let prepared = crypto
            .delivery
            .prepare(&stored.subscription.endpoint, request)
            .await?;
        #[cfg(test)]
        self.prepared(super::PreparedStage::Transport).await;
        // Last admission check follows every asynchronous preparation step,
        // including DNS. The following send begins with no preparatory await.
        let Some(attention) = self.admission(&stored, burst).await? else {
            return Ok(None);
        };
        match prepared.send().await? {
            Delivered::Accepted => {}
            Delivered::Gone => {
                self.store.expire_push(stored).await?;
            }
            Delivered::Refused => tracing::warn!(
                reason = "service_refused",
                "push service did not accept the notification"
            ),
        }
        Ok(Some(attention))
    }
}

/// Inclusive upper bound captured after coalescing, exclusive lower bound.
#[derive(Debug, Clone, Copy)]
struct Burst {
    after: InboxCursor,
    through: InboxCursor,
}
#[cfg(test)]
async fn self_pause(push: &Push, stop: &CancellationToken) {
    tokio::select! { () = stop.cancelled() => {}, () = push.prepared(super::PreparedStage::Observed) => {} }
}
