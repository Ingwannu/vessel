//! One coalescing delivery loop per node. No polling, durable notification
//! replay or unbounded delivery queue. The inbox remains the durable truth.
use super::{Delivered, Push, PushError};
use chrono::Utc;
use futures_util::{StreamExt as _, stream};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::inbox::InboxCursor;
use vessel_proto::notifications::NotificationSource;
use vessel_store::{PushAttention, StoredPush};

const COALESCE: Duration = Duration::from_millis(250);
const SEND_DEADLINE: Duration = Duration::from_secs(10);
const DELIVERY_CONCURRENCY: usize = 4;

pub(crate) async fn start(
    push: Arc<Push>,
    terminals: Arc<crate::terminals::Terminals>,
    tasks: &TaskTracker,
    stop: CancellationToken,
) -> Result<(), vessel_store::StoreError> {
    // Subscribe before reading the watermark: an attention committed during
    // startup is either in the initial snapshot or wakes this receiver.
    let changes = push.store.inbox_changes();
    let (terminals, terminal_changes) = super::terminal::Observer::new(terminals);
    let head = push.store.inbox_head().await?;
    let observed = Observed {
        changes,
        terminal_changes,
        head,
        terminals,
    };
    drop(tasks.spawn(run(push, observed, stop)));
    Ok(())
}
async fn run(push: Arc<Push>, mut observed: Observed, stop: CancellationToken) {
    loop {
        tokio::select! {
            () = stop.cancelled() => break,
            wake = observed.changes.changed() => { if wake.is_err() { break; } },
            wake = observed.terminal_changes.changed() => { if wake.is_err() { break; } },
        }
        #[cfg(test)]
        self_pause(&push, &stop).await;
        tokio::select! { () = stop.cancelled() => break, () = tokio::time::sleep(COALESCE) => {} }
        let before = observed.head;
        let terminal = observed.terminals.capture();
        let signal = async {
            observed.head = push.store.inbox_head().await?;
            Ok::<_, vessel_store::StoreError>(
                push.pick(before, observed.head).await?.is_some()
                    || terminal.current(&push).is_some(),
            )
        }
        .await;
        match signal {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_store) => {
                tracing::warn!(reason = "store", "push attention could not be read");
                continue;
            }
        }
        let burst = Burst {
            after: before,
            through: observed.head,
            terminal,
        };
        let subscriptions = match push.store.push_subscriptions(Utc::now()).await {
            Ok(subscriptions) => subscriptions,
            Err(_store) => {
                tracing::warn!(reason = "store", "push subscriptions could not be read");
                continue;
            }
        };
        let sent =
            stream::iter(subscriptions).for_each_concurrent(DELIVERY_CONCURRENCY, |subscription| {
                let push = Arc::clone(&push);
                let burst = burst.clone();
                async move {
                    let result = push.send_one(subscription, burst).await;
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
        burst: &Burst,
    ) -> Result<Option<NotificationSource>, PushError> {
        let mut after = burst.after;
        while let Some(attention) = self.pick(after, burst.through).await? {
            if self
                .store
                .push_admitted(stored, attention, Utc::now())
                .await?
                && self.should_push(attention.key.session)
            {
                return Ok(Some(NotificationSource::Session {
                    key: attention.key,
                    cursor: attention.cursor,
                }));
            }
            after = attention.cursor;
        }
        if self.store.push_lease_admitted(stored, Utc::now()).await? {
            return Ok(burst.terminal.current(self));
        }
        Ok(None)
    }
    /// Admission follows asynchronous preparation; an already admitted
    /// ciphertext cannot be recalled. The whole attempt remains time bounded.
    async fn send_one(
        &self,
        stored: StoredPush,
        burst: Burst,
    ) -> Result<Option<NotificationSource>, PushError> {
        tokio::time::timeout(SEND_DEADLINE, self.send_within(stored, burst))
            .await
            .map_err(|_elapsed| super::delivery::DeliveryError::Timeout)?
    }
    async fn send_within(
        &self,
        stored: StoredPush,
        burst: Burst,
    ) -> Result<Option<NotificationSource>, PushError> {
        if self.admission(&stored, &burst).await?.is_none() {
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
        let Some(attention) = self.admission(&stored, &burst).await? else {
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

struct Observed {
    changes: tokio::sync::watch::Receiver<()>,
    terminal_changes: tokio::sync::watch::Receiver<()>,
    head: InboxCursor,
    terminals: super::terminal::Observer,
}

/// Inclusive upper bound captured after coalescing, exclusive lower bound.
#[derive(Debug, Clone)]
struct Burst {
    after: InboxCursor,
    through: InboxCursor,
    terminal: super::terminal::Burst,
}
#[cfg(test)]
async fn self_pause(push: &Push, stop: &CancellationToken) {
    tokio::select! { () = stop.cancelled() => {}, () = push.prepared(super::PreparedStage::Observed) => {} }
}
