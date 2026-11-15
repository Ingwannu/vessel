//! Asynchronous catalogue owner. One task owns the cache and waiters;
//! bounded producer futures support native transports and plugin connections.

use super::cache::{Admission, Cache, CacheError, Observation, Refresh, Ticket, Work, WorkBudget};
use super::lease::WorkLease;
use futures_util::{StreamExt, stream::FuturesUnordered};
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};
#[cfg(test)]
use tokio::sync::broadcast;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_harness::providers::ProviderRegistry;
use vessel_proto::providers::{
    CatalogueScope, DiscoveryFailure, DiscoveryPhase, ProviderDiagnostic, ProviderId,
    ProviderSnapshotEntry,
};

/// Reference discovery budget; queued work shares this finite budget here.
pub(super) const REFRESH_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const PER_PROVIDER: usize = 4;
const MAX_JOBS: usize = 32;
const MAX_WAITERS: usize = 64;
const MAILBOX: usize = 32;
/// Slow subscribers receive Lagged and must reread a cached snapshot.
#[cfg(test)]
const CHANGE_CAPACITY: usize = 32;

type Answer = oneshot::Sender<Result<ProviderDiagnostic, OwnerError>>;

/// Notifications are invalidation hints. Consumers reread after lag and never
/// confuse an observation for one workspace with another workspace's state.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    /// One scoped observation became available.
    Observed {
        /// Canonical cache scope of the provider family.
        scope: CatalogueScope,
        /// Reader-safe projection, without launch or secret values.
        diagnostic: Arc<ProviderDiagnostic>,
    },
    /// A complete definition generation changed; reread the desired scope.
    Registry,
    /// The owning node task stopped; no more results will be published.
    Stopped,
}

/// A discovery request could not be admitted or completed by the bounded owner.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OwnerError {
    /// A checked identity or bounded cache admission failed.
    #[error(transparent)]
    Cache(#[from] CacheError),
    /// Owner/channel shutdown.
    #[error("provider discovery owner is stopped")]
    Stopped,
    /// A bounded discovery/waiter queue is full.
    #[error("provider discovery admission capacity is full")]
    Capacity,
    /// Configuration changed before the current observation could publish.
    #[error("provider discovery was superseded")]
    Superseded,
    /// The whole snapshot request's finite discovery budget elapsed.
    #[error("provider snapshot discovery deadline elapsed")]
    Deadline,
}

#[derive(Debug)]
enum Mail {
    Snapshot {
        scope: CatalogueScope,
        reply: oneshot::Sender<Result<Vec<ProviderSnapshotEntry>, OwnerError>>,
    },
    Cached {
        provider: ProviderId,
        scope: CatalogueScope,
        reply: Answer,
    },
    Refresh {
        provider: ProviderId,
        scope: CatalogueScope,
        refresh: Refresh,
        reply: Answer,
    },
    Replace {
        registry: ProviderRegistry,
        reply: oneshot::Sender<()>,
    },
    Stop(oneshot::Sender<()>),
}

#[derive(Debug)]
struct Pending {
    cancel: CancellationToken,
    waiters: Vec<Answer>,
}

/// Internal service seam shared by pending RPC handlers and trusted consumers.
#[derive(Debug, Clone)]
pub(super) struct Owner {
    mail: mpsc::Sender<Mail>,
    #[cfg(test)]
    changes: broadcast::Sender<Change>,
}

impl Owner {
    pub(super) async fn snapshot(
        &self,
        scope: CatalogueScope,
    ) -> Result<Vec<ProviderSnapshotEntry>, OwnerError> {
        let (reply, response) = oneshot::channel();
        self.mail
            .send(Mail::Snapshot { scope, reply })
            .await
            .map_err(|_closed| OwnerError::Stopped)?;
        response.await.map_err(|_closed| OwnerError::Stopped)?
    }
    /// Generic futures are the open provider/plugin producer seam. The actor
    /// does not invent a single-implementation trait merely for its tests.
    pub(super) fn start<P, F>(
        registry: ProviderRegistry,
        producer: P,
    ) -> (Self, tokio::task::JoinHandle<()>)
    where
        P: Fn(Work, CancellationToken, WorkLease) -> F + Send + Sync + 'static,
        F: Future<Output = Observation> + Send + 'static,
    {
        let (mail, commands) = mpsc::channel(MAILBOX);
        #[cfg(test)]
        let (changes, _initial) = broadcast::channel(CHANGE_CAPACITY);
        let owner = Self {
            mail,
            #[cfg(test)]
            changes: changes.clone(),
        };
        let task = tokio::spawn(run(
            Cache::new(registry),
            commands,
            #[cfg(test)]
            changes,
            Arc::new(producer),
        ));
        (owner, task)
    }

    pub(super) async fn cached(
        &self,
        provider: ProviderId,
        scope: CatalogueScope,
    ) -> Result<ProviderDiagnostic, OwnerError> {
        let (reply, response) = oneshot::channel();
        self.mail
            .send(Mail::Cached {
                provider,
                scope,
                reply,
            })
            .await
            .map_err(|_closed| OwnerError::Stopped)?;
        response.await.map_err(|_closed| OwnerError::Stopped)?
    }

    pub(super) async fn refresh(
        &self,
        provider: ProviderId,
        scope: CatalogueScope,
        refresh: Refresh,
    ) -> Result<ProviderDiagnostic, OwnerError> {
        let (reply, response) = oneshot::channel();
        self.mail
            .send(Mail::Refresh {
                provider,
                scope,
                refresh,
                reply,
            })
            .await
            .map_err(|_closed| OwnerError::Stopped)?;
        response.await.map_err(|_closed| OwnerError::Stopped)?
    }

    pub(super) async fn replace(&self, registry: ProviderRegistry) -> Result<(), OwnerError> {
        let (reply, response) = oneshot::channel();
        self.mail
            .send(Mail::Replace { registry, reply })
            .await
            .map_err(|_closed| OwnerError::Stopped)?;
        response.await.map_err(|_closed| OwnerError::Stopped)
    }

    #[cfg(test)]
    pub(super) fn changes(&self) -> broadcast::Receiver<Change> {
        let subscription = self.changes.subscribe();
        if self.mail.is_closed() {
            let (closed, receiver) = broadcast::channel(CHANGE_CAPACITY);
            drop(closed);
            receiver
        } else {
            subscription
        }
    }

    pub(super) async fn stop(&self) -> Result<(), OwnerError> {
        let (reply, response) = oneshot::channel();
        self.mail
            .send(Mail::Stop(reply))
            .await
            .map_err(|_closed| OwnerError::Stopped)?;
        response.await.map_err(|_closed| OwnerError::Stopped)
    }
}

async fn run<P, F>(
    mut cache: Cache,
    mut commands: mpsc::Receiver<Mail>,
    #[cfg(test)] changes: broadcast::Sender<Change>,
    producer: Arc<P>,
) where
    P: Fn(Work, CancellationToken, WorkLease) -> F + Send + Sync + 'static,
    F: Future<Output = Observation> + Send + 'static,
{
    let mut pending = BTreeMap::<Ticket, Pending>::new();
    let mut permits = BTreeMap::<ProviderId, Arc<Semaphore>>::new();
    let mut jobs = FuturesUnordered::new();
    let global = Arc::new(Semaphore::new(MAX_JOBS));
    let workers = TaskTracker::new();
    let stopped = loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(Mail::Snapshot { scope, reply }) => {
                    let _gone = reply.send(cache.snapshot(&scope).map_err(OwnerError::from));
                }
                Some(Mail::Cached { provider, scope, reply }) => {
                    let _gone = reply.send(cache.cached(&provider, &scope).map_err(OwnerError::from));
                }
                Some(Mail::Refresh { provider, scope, refresh, reply }) => {
                    // The reducer checks existing/disabled states before work
                    // admission, so full execution cannot block cached answers.
                    let budget = if jobs.len() == MAX_JOBS { WorkBudget::Full } else { WorkBudget::Available };
                    match cache.admit(&provider, &scope, refresh, budget) {
                        Ok(Admission::Ready(value)) => { let _gone = reply.send(Ok(value)); }
                        Ok(Admission::Joined(ticket)) => join(&mut pending, ticket, reply),
                        Ok(Admission::Start(work)) => {
                            if let Some(old) = work.supersedes { cancel(&mut pending, old, &OwnerError::Superseded); }
                            let cancel = CancellationToken::new();
                            let _old = pending.insert(work.ticket, Pending { cancel: cancel.clone(), waiters: vec![reply] });
                            let semaphore = Arc::clone(permits.entry(provider).or_insert_with(|| Arc::new(Semaphore::new(PER_PROVIDER))));
                            jobs.push(execute(work, Arc::clone(&producer), semaphore, cancel, Arc::clone(&global), workers.clone()));
                        }
                        Err(error) => { let _gone = reply.send(Err(error.into())); }
                    }
                }
                Some(Mail::Replace { registry, reply }) => {
                    permits.retain(|id, semaphore| registry.get(id).is_some() || semaphore.available_permits() != PER_PROVIDER);
                    for ticket in cache.replace(registry) { cancel(&mut pending, ticket, &OwnerError::Superseded); }
                    #[cfg(test)]
                    let _unobserved = changes.send(Change::Registry);
                    let _gone = reply.send(());
                }
                Some(Mail::Stop(reply)) => break Some(reply),
                None => break None,
            },
            Some((work, observation)) = jobs.next(), if !jobs.is_empty() => {
                let waiters = pending.remove(&work.ticket);
                if let Some(value) = cache.complete(&work, observation, chrono::Utc::now()) {
                    if let Some(pending) = waiters {
                        for waiter in pending.waiters { let _gone = waiter.send(Ok(value.clone())); }
                    }
                    #[cfg(test)]
                    let _unobserved = changes.send(Change::Observed { scope: work.scope, diagnostic: Arc::new(value) });
                } else if let Some(pending) = waiters {
                    for waiter in pending.waiters { let _gone = waiter.send(Err(OwnerError::Superseded)); }
                }
            }
        }
    };
    // Cancel async work first, then drain the actual blocking preparations.
    // Process-guard drop is not proof of descendant termination.
    for (_, pending) in pending {
        pending.cancel.cancel();
        for reply in pending.waiters {
            let _gone = reply.send(Err(OwnerError::Stopped));
        }
    }
    drop(jobs);
    let _was_open = workers.close();
    workers.wait().await;
    if let Some(reply) = stopped {
        let _gone = reply.send(());
    }
    #[cfg(test)]
    let _unobserved = changes.send(Change::Stopped);
}

fn join(pending: &mut BTreeMap<Ticket, Pending>, ticket: Ticket, reply: Answer) {
    if let Some(job) = pending.get_mut(&ticket) {
        if job.waiters.len() == MAX_WAITERS {
            let _gone = reply.send(Err(OwnerError::Capacity));
        } else {
            job.waiters.push(reply);
        }
    } else {
        let _gone = reply.send(Err(OwnerError::Superseded));
    }
}

fn cancel(pending: &mut BTreeMap<Ticket, Pending>, ticket: Ticket, reason: &OwnerError) {
    if let Some(old) = pending.remove(&ticket) {
        old.cancel.cancel();
        for reply in old.waiters {
            let _gone = reply.send(Err(reason.clone()));
        }
    }
}

async fn execute<P, F>(
    work: Work,
    producer: Arc<P>,
    semaphore: Arc<Semaphore>,
    cancel: CancellationToken,
    global: Arc<Semaphore>,
    workers: TaskTracker,
) -> (Work, Observation)
where
    P: Fn(Work, CancellationToken, WorkLease) -> F + Send + Sync,
    F: Future<Output = Observation> + Send,
{
    let operation = async {
        let Ok(global) = global.acquire_owned().await else {
            return Observation::Failed(DiscoveryFailure::Stopped);
        };
        let Ok(provider) = semaphore.acquire_owned().await else {
            return Observation::Failed(DiscoveryFailure::Stopped);
        };
        producer(
            work.clone(),
            cancel.clone(),
            WorkLease::new(global, provider, workers),
        )
        .await
    };
    let observation = tokio::select! {
        biased;
        () = cancel.cancelled() => Observation::Failed(DiscoveryFailure::Superseded),
        result = tokio::time::timeout(REFRESH_TIMEOUT, operation) => result.unwrap_or(Observation::Failed(DiscoveryFailure::TimedOut { phase: DiscoveryPhase::Catalogue })),
    };
    (work, observation)
}

#[cfg(test)]
mod tests;
