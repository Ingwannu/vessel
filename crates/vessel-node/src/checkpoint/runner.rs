//! A single nonblocking owner fences candidates; admitted I/O jobs retain capacity.

mod availability;

use super::{
    capture::{Captures, RootState},
    quota::ReservedBytes,
    workers::{WorkerLease, Workers},
};
use std::{future::Future, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::{Instant, sleep_until},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_QUEUE, CHECKPOINT_WORKERS, CaptureKey, CheckpointError,
    ObservedSnapshot, PhysicalRoot,
};
use vessel_session::CheckpointAdvance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Disposition {
    Keep,
    Discard(CheckpointError),
    /// ReserveCapture refused before its INSERT; no filesystem read was started.
    NoReservation,
    /// Ready activation may have committed; keep bytes until reconciliation.
    Recovery,
}
#[derive(Debug)]
pub(super) enum Publication {
    /// Durable global admission precedes every filesystem read or allocation.
    Reserve(ReservedBytes),
    Candidate(ObservedSnapshot),
    /// Only sent after the coordinator accepted the durable candidate in time.
    Activate,
}
#[derive(Debug)]
struct Request {
    key: CaptureKey,
    physical: PhysicalRoot,
    bytes: ReservedBytes,
    deadline: Instant,
    reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
    advancement: oneshot::Receiver<CheckpointAdvance>,
}
#[derive(Debug)]
struct Work {
    key: CaptureKey,
    physical: PhysicalRoot,
    bytes: ReservedBytes,
    deadline: Instant,
    cancellation: CancellationToken,
    advancement: oneshot::Receiver<CheckpointAdvance>,
    worker: Arc<WorkerLease>,
}
#[derive(Debug)]
enum Event {
    CleanupFailed {
        key: CaptureKey,
        reason: CheckpointError,
    },
    Candidate {
        key: CaptureKey,
        permit: oneshot::Sender<Result<(), CheckpointError>>,
    },
    Published {
        key: CaptureKey,
        result: Result<(), CheckpointError>,
        accepted: oneshot::Sender<Disposition>,
    },
    Failed {
        key: CaptureKey,
        reason: CheckpointError,
    },
    Cleaned(CaptureKey),
}
#[derive(Debug, Clone)]
pub(crate) struct Client {
    requests: mpsc::Sender<Request>,
    #[cfg(test)]
    lifecycle: watch::Receiver<Lifecycle>,
    cleanup_faults: watch::Receiver<Option<CleanupFault>>,
    roots: watch::Receiver<std::collections::BTreeMap<PhysicalRoot, RootState>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CleanupFault {
    pub(crate) key: CaptureKey,
    pub(super) reason: CheckpointError,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lifecycle {
    Accepting,
    Draining,
}
/// Durable candidate acknowledgement, still gated on the actual actor receipt.
#[derive(Debug)]
pub(crate) struct AcceptedCapture {
    pub(super) key: CaptureKey,
    advancement: oneshot::Sender<CheckpointAdvance>,
}
impl AcceptedCapture {
    pub(crate) fn complete(self, outcome: CheckpointAdvance) {
        let _closed = self.advancement.send(outcome);
    }
}
impl Client {
    pub(super) fn cleanup_faults(&self) -> watch::Receiver<Option<CleanupFault>> {
        self.cleanup_faults.clone()
    }
    #[cfg(test)]
    pub(super) fn lifecycle(&self) -> watch::Receiver<Lifecycle> {
        self.lifecycle.clone()
    }
    #[cfg(test)]
    pub(super) async fn capture(
        &self,
        key: CaptureKey,
        bytes: ReservedBytes,
    ) -> Result<CaptureKey, CheckpointError> {
        self.capture_at_root(
            super::capture::model_root(key.worktree),
            key,
            bytes,
            Instant::now()
                .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
                .ok_or(CheckpointError::CaptureTimedOut)?,
        )
        .await
    }
    #[cfg(test)]
    pub(super) async fn capture_at_root(
        &self,
        physical: PhysicalRoot,
        key: CaptureKey,
        bytes: ReservedBytes,
        deadline: Instant,
    ) -> Result<CaptureKey, CheckpointError> {
        let accepted = self
            .capture_reserved(physical, key, bytes, deadline)
            .await?;
        let key = accepted.key;
        accepted.complete(CheckpointAdvance::TurnComplete);
        Ok(key)
    }
    /// Bridges preserve the actor's original deadline across their own queues.
    /// Dropping the returned receipt cannot activate a durable Ready image.
    pub(crate) async fn capture_candidate(
        &self,
        physical: PhysicalRoot,
        key: CaptureKey,
        deadline: Instant,
    ) -> Result<AcceptedCapture, CheckpointError> {
        self.wait_for_root(physical, deadline).await?;
        self.capture_reserved(physical, key, ReservedBytes::capture(), deadline)
            .await
    }
    pub(super) async fn capture_reserved(
        &self,
        physical: PhysicalRoot,
        key: CaptureKey,
        bytes: ReservedBytes,
        deadline: Instant,
    ) -> Result<AcceptedCapture, CheckpointError> {
        if Instant::now() >= deadline {
            return Err(CheckpointError::CaptureTimedOut);
        }
        let (reply, received) = oneshot::channel();
        let (advancement, advanced) = oneshot::channel();
        self.requests
            .try_send(Request {
                key,
                physical,
                bytes,
                deadline,
                reply,
                advancement: advanced,
            })
            .map_err(|_full_or_closed| CheckpointError::Busy)?;
        let key = tokio::time::timeout_at(deadline, received)
            .await
            .map_err(|_deadline| CheckpointError::CaptureTimedOut)?
            .map_err(|_closed| CheckpointError::StaleCapture)??;
        if Instant::now() >= deadline {
            return Err(CheckpointError::CaptureTimedOut);
        }
        Ok(AcceptedCapture { key, advancement })
    }
}

#[cfg(test)]
pub(super) fn start<Read, Persist, Cleanup, Stored>(
    read: Read,
    persist: Persist,
    cleanup: Cleanup,
) -> (Client, tokio::task::JoinHandle<()>)
where
    Read: Fn(CaptureKey, PhysicalRoot, CancellationToken) -> Result<ObservedSnapshot, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, Publication) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
    Cleanup: Fn(CaptureKey, Disposition) -> Result<(), CheckpointError> + Send + Sync + 'static,
{
    start_shared(Workers::default(), read, persist, cleanup)
}

/// Concrete closures are the node's I/O/store assembly seam, never a dynamic registry.
pub(super) fn start_shared<Read, Persist, Cleanup, Stored>(
    workers: Workers,
    read: Read,
    persist: Persist,
    cleanup: Cleanup,
) -> (Client, tokio::task::JoinHandle<()>)
where
    Read: Fn(CaptureKey, PhysicalRoot, CancellationToken) -> Result<ObservedSnapshot, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, Publication) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
    Cleanup: Fn(CaptureKey, Disposition) -> Result<(), CheckpointError> + Send + Sync + 'static,
{
    let (requests, mut receive) = mpsc::channel::<Request>(CHECKPOINT_QUEUE);
    let (events, mut completed) = mpsc::channel::<Event>(CHECKPOINT_WORKERS);
    let (state, _lifecycle) = watch::channel(Lifecycle::Accepting);
    let (faults, cleanup_faults) = watch::channel(None);
    let (root_state, roots) = watch::channel(std::collections::BTreeMap::new());
    let read = Arc::new(read);
    let persist = Arc::new(persist);
    let cleanup = Arc::new(cleanup);
    let task = tokio::spawn(async move {
        let mut captures = Captures::new(workers);
        let mut requests_open = true;
        loop {
            let _previous = root_state.send_replace(captures.roots());
            if !requests_open && captures.drained() {
                break;
            }
            let deadline = captures.deadline();
            tokio::select! {biased;
                ()=wait_deadline(deadline)=>captures.expire(Instant::now()),
                event=completed.recv()=>{
                    let Some(event)=event else {break;};
                    completed_event(&mut captures,event,&faults);
                }
                request=receive.recv(), if requests_open=>{
                    let Some(request)=request else {
                        // Closing callers does not make blocked I/O disappear. Drain
                        // admitted workers before dropping their guards/reservations.
                        requests_open=false;
                        let _previous=state.send_replace(Lifecycle::Draining);
                        continue;
                    };
                    if Instant::now() >= request.deadline {
                        let _closed=request.reply.send(Err(CheckpointError::CaptureTimedOut));
                        continue;
                    }
                    let key=request.key;
                    if captures.admit_at_root(key,request.physical,request.deadline,request.bytes,request.reply).is_ok() {
                        let cancellation=captures.cancellation(key);
                        if let Some((cancellation,worker))=cancellation.zip(captures.worker(key)) {
                            let job=Work {key,physical:request.physical,bytes:request.bytes,deadline:request.deadline,cancellation,advancement:request.advancement,worker};
                            let _worker=tokio::spawn(work(job,Arc::clone(&read),Arc::clone(&persist),Arc::clone(&cleanup),events.clone()));
                        }
                    }
                }
            }
        }
    });
    (
        Client {
            requests,
            #[cfg(test)]
            lifecycle: _lifecycle,
            cleanup_faults,
            roots,
        },
        task,
    )
}

fn completed_event(
    captures: &mut Captures,
    event: Event,
    faults: &watch::Sender<Option<CleanupFault>>,
) {
    match event {
        Event::CleanupFailed { key, reason } => {
            captures.cleanup_failed(key, reason);
            let _previous = faults.send_replace(Some(CleanupFault { key, reason }));
        }
        Event::Candidate { key, permit } => {
            let _closed = permit.send(captures.candidate(key, Instant::now()));
        }
        Event::Published {
            key,
            result,
            accepted,
        } => {
            let disposition = match result {
                Ok(()) => match captures.published(key, Instant::now()) {
                    Ok(()) => Disposition::Keep,
                    Err(reason) => Disposition::Discard(reason),
                },
                Err(reason) => {
                    captures.refuse(key, reason);
                    Disposition::Discard(reason)
                }
            };
            let _closed = accepted.send(disposition);
        }
        Event::Failed { key, reason } => captures.refuse(key, reason),
        Event::Cleaned(key) => {
            let _stale = captures.cleaned(key);
        }
    }
}

async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn work<Read, Persist, Cleanup, Stored>(
    job: Work,
    read: Arc<Read>,
    persist: Arc<Persist>,
    cleanup: Arc<Cleanup>,
    events: mpsc::Sender<Event>,
) where
    Read: Fn(CaptureKey, PhysicalRoot, CancellationToken) -> Result<ObservedSnapshot, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, Publication) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
    Cleanup: Fn(CaptureKey, Disposition) -> Result<(), CheckpointError> + Send + Sync + 'static,
{
    let key = job.key;
    let worker = Arc::clone(&job.worker);
    let disposition = match persist(key, Publication::Reserve(job.bytes)).await {
        Ok(()) if Instant::now() < job.deadline && !job.cancellation.is_cancelled() => {
            read_candidate(job, read, persist, &events).await
        }
        Ok(()) => {
            let _closed = events
                .send(Event::Failed {
                    key,
                    reason: CheckpointError::CaptureTimedOut,
                })
                .await;
            Disposition::Discard(CheckpointError::CaptureTimedOut)
        }
        Err(reason) => {
            let _closed = events.send(Event::Failed { key, reason }).await;
            // These ReserveCapture refusals occur before INSERT and before any
            // filesystem callback. Other storage failures remain uncertain.
            if matches!(
                reason,
                CheckpointError::StorageFull | CheckpointError::StaleCapture
            ) {
                Disposition::NoReservation
            } else {
                Disposition::Recovery
            }
        }
    };
    // Cleanup is deliberately awaited here, never by the session/deadline owner.
    let cleaned = tokio::task::spawn_blocking(move || {
        let _worker = worker;
        cleanup(key, disposition)
    })
    .await;
    let event = match cleaned {
        Ok(Ok(())) => Event::Cleaned(key),
        Ok(Err(reason)) => Event::CleanupFailed { key, reason },
        Err(_lost) => Event::CleanupFailed {
            key,
            reason: CheckpointError::StaleCapture,
        },
    };
    let _closed = events.send(event).await;
}
async fn read_candidate<Read, Persist, Stored>(
    job: Work,
    read: Arc<Read>,
    persist: Arc<Persist>,
    events: &mpsc::Sender<Event>,
) -> Disposition
where
    Read: Fn(CaptureKey, PhysicalRoot, CancellationToken) -> Result<ObservedSnapshot, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, Publication) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
{
    let key = job.key;
    let physical = job.physical;
    let cancel = job.cancellation;
    let worker = Arc::clone(&job.worker);
    let candidate = tokio::task::spawn_blocking(move || {
        let _worker = worker;
        read(key, physical, cancel)
    })
    .await;
    match candidate {
        Ok(Ok(image)) => publish(key, image, job.advancement, persist, events).await,
        Ok(Err(reason)) => {
            let _closed = events.send(Event::Failed { key, reason }).await;
            Disposition::Discard(reason)
        }
        Err(_lost) => {
            let _closed = events
                .send(Event::Failed {
                    key,
                    reason: CheckpointError::StaleCapture,
                })
                .await;
            Disposition::Discard(CheckpointError::StaleCapture)
        }
    }
}
async fn publish<Persist, Stored>(
    key: CaptureKey,
    image: ObservedSnapshot,
    advancement: oneshot::Receiver<CheckpointAdvance>,
    persist: Arc<Persist>,
    events: &mpsc::Sender<Event>,
) -> Disposition
where
    Persist: Fn(CaptureKey, Publication) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
{
    let (permit, allowed) = oneshot::channel();
    if events.send(Event::Candidate { key, permit }).await.is_err() {
        return Disposition::Discard(CheckpointError::StaleCapture);
    }
    match allowed.await {
        Ok(Ok(())) => {}
        Ok(Err(reason)) => return Disposition::Discard(reason),
        Err(_closed) => return Disposition::Discard(CheckpointError::StaleCapture),
    }
    let result = persist(key, Publication::Candidate(image)).await;
    let (accepted, outcome) = oneshot::channel();
    if events
        .send(Event::Published {
            key,
            result,
            accepted,
        })
        .await
        .is_err()
    {
        return Disposition::Discard(CheckpointError::StaleCapture);
    }
    match outcome
        .await
        .unwrap_or(Disposition::Discard(CheckpointError::StaleCapture))
    {
        Disposition::Keep => match advancement.await {
            Ok(CheckpointAdvance::PromptSent | CheckpointAdvance::TurnComplete) => {
                match persist(key, Publication::Activate).await {
                    Ok(()) => Disposition::Keep,
                    Err(_uncertain) => Disposition::Recovery,
                }
            }
            Ok(CheckpointAdvance::PromptRefused) | Err(_) => {
                Disposition::Discard(CheckpointError::StaleCapture)
            }
        },
        Disposition::Discard(reason) => Disposition::Discard(reason),
        Disposition::NoReservation => Disposition::NoReservation,
        Disposition::Recovery => Disposition::Recovery,
    }
}
