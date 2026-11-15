//! One tracked boot coordinator; transient attempts never become a job registry.
use super::{ADMISSION, settled};
use crate::guard::Guarded;
use crate::recovery::{Attempt, Claim, Recovery};
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::SessionId;
use vessel_store::{RestartCandidate, StoreError};

pub(crate) fn start(service: &Arc<Guarded>, tasks: &TaskTracker, stop: CancellationToken) {
    drop(tasks.spawn(coordinate(Arc::clone(service), stop)));
}
async fn coordinate(service: Arc<Guarded>, stop: CancellationToken) {
    let mut running = JoinSet::new();
    loop {
        if stop.is_cancelled() {
            break;
        }
        if running.len() >= Recovery::MAX_ATTEMPTS {
            settled(running.join_next().await);
            continue;
        }
        let candidate = match service.0.sessions.store().next_restart().await {
            Ok(Some(candidate)) => candidate,
            Ok(None) => break,
            Err(error) => {
                tracing::error!(%error, "cannot read startup recovery intents");
                break;
            }
        };
        let Some(attempt) = wait_claim(&service, candidate.session, &stop).await else {
            if stop.is_cancelled() {
                break;
            }
            tracing::warn!(session = %candidate.session, "startup recovery yielded to an ordinary operation that still owns this session");
            if let Err(error) = release_skipped(&service, candidate).await {
                tracing::error!(%error, "cannot consume a skipped startup intent");
                break;
            }
            continue;
        };
        match service.0.sessions.store().claim_restart(candidate).await {
            Ok(true) => {
                let service = Arc::clone(&service);
                let stopping = stop.clone();
                drop(running.spawn(async move {
                    service.0.recover(candidate, attempt, stopping).await;
                }));
            }
            Ok(false) => drop(attempt),
            Err(error) => {
                tracing::error!(%error, "cannot consume startup recovery intent");
                break;
            }
        }
    }
    // Cancellation stops new phases, never ownership of admitted effects.
    while let Some(result) = running.join_next().await {
        settled(Some(result));
    }
    #[cfg(test)]
    service.0.recovery.finish_boot();
}
async fn wait_claim(
    service: &Guarded,
    session: SessionId,
    stop: &CancellationToken,
) -> Option<Attempt> {
    let deadline = Instant::now()
        .checked_add(ADMISSION)
        .unwrap_or_else(Instant::now);
    loop {
        match service.0.recovery.claim(session) {
            Claim::Owned(attempt) => return Some(attempt),
            Claim::Wait(mut changed) => {
                tokio::select! {
                    biased;
                    () = stop.cancelled() => return None,
                    () = tokio::time::sleep_until(deadline) => return None,
                    _changed = changed.changed() => {},
                }
            }
        }
    }
}
async fn release_skipped(service: &Guarded, candidate: RestartCandidate) -> Result<(), StoreError> {
    let store = service.0.sessions.store();
    if store.claim_restart(candidate).await? {
        store.finish_restart(candidate.session).await?;
    }
    Ok(())
}
