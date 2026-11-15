//! A single nonblocking owner fences candidates; admitted I/O jobs retain capacity.

use super::{capture::Captures, quota::ReservedBytes};
use std::{future::Future, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Instant, sleep_until},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_QUEUE, CHECKPOINT_WORKERS, CaptureKey, CheckpointError,
    FileImage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Disposition {
    Keep,
    Discard,
}
#[derive(Debug)]
struct Request {
    key: CaptureKey,
    bytes: ReservedBytes,
    deadline: Instant,
    reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
}
#[derive(Debug)]
enum Event {
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
pub(super) struct Client {
    requests: mpsc::Sender<Request>,
}
impl Client {
    pub(super) async fn capture(
        &self,
        key: CaptureKey,
        bytes: ReservedBytes,
    ) -> Result<CaptureKey, CheckpointError> {
        let (reply, received) = oneshot::channel();
        self.requests
            .try_send(Request {
                key,
                bytes,
                deadline: Instant::now()
                    .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
                    .ok_or(CheckpointError::CaptureTimedOut)?,
                reply,
            })
            .map_err(|_full_or_closed| CheckpointError::Busy)?;
        received
            .await
            .map_err(|_closed| CheckpointError::StaleCapture)?
    }
}

/// Concrete closures are the node's I/O/store assembly seam, never a dynamic registry.
pub(super) fn start<Read, Persist, Cleanup, Stored>(
    read: Read,
    persist: Persist,
    cleanup: Cleanup,
) -> (Client, tokio::task::JoinHandle<()>)
where
    Read: Fn(CaptureKey, CancellationToken) -> Result<FileImage, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, FileImage) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
    Cleanup: Fn(CaptureKey, Disposition) + Send + Sync + 'static,
{
    let (requests, mut receive) = mpsc::channel::<Request>(CHECKPOINT_QUEUE);
    let (events, mut completed) = mpsc::channel::<Event>(CHECKPOINT_WORKERS);
    let read = Arc::new(read);
    let persist = Arc::new(persist);
    let cleanup = Arc::new(cleanup);
    let task = tokio::spawn(async move {
        let mut captures = Captures::default();
        loop {
            let deadline = captures.deadline();
            tokio::select! {
                request=receive.recv()=>{
                    let Some(request)=request else {break;};
                    let key=request.key;
                    if captures.admit(key,request.deadline,request.bytes,request.reply).is_ok() {
                        let cancellation=captures.cancellation(key);
                        if let Some(cancellation)=cancellation {
                            let _worker=tokio::spawn(work(key,cancellation,Arc::clone(&read),Arc::clone(&persist),Arc::clone(&cleanup),events.clone()));
                        }
                    }
                }
                event=completed.recv()=>{
                    let Some(event)=event else {break;};
                    match event {
                        Event::Candidate{key,permit}=>{
                            let _closed=permit.send(captures.candidate(key,Instant::now()));
                        }
                        Event::Published{key,result,accepted}=>{
                            let disposition=match result {
                                Ok(())=>match captures.published(key,Instant::now()) {Ok(())=>Disposition::Keep,Err(_)=>Disposition::Discard},
                                Err(reason)=>{captures.refuse(key,reason);Disposition::Discard},
                            };
                            let _closed=accepted.send(disposition);
                        }
                        Event::Failed{key,reason}=>captures.refuse(key,reason),
                        Event::Cleaned(key)=>{let _stale=captures.cleaned(key);},
                    }
                }
                ()=wait_deadline(deadline)=>captures.expire(Instant::now()),
            }
        }
    });
    (Client { requests }, task)
}

async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn work<Read, Persist, Cleanup, Stored>(
    key: CaptureKey,
    cancellation: CancellationToken,
    read: Arc<Read>,
    persist: Arc<Persist>,
    cleanup: Arc<Cleanup>,
    events: mpsc::Sender<Event>,
) where
    Read: Fn(CaptureKey, CancellationToken) -> Result<FileImage, CheckpointError>
        + Send
        + Sync
        + 'static,
    Persist: Fn(CaptureKey, FileImage) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
    Cleanup: Fn(CaptureKey, Disposition) + Send + Sync + 'static,
{
    let read_key = key;
    let candidate = tokio::task::spawn_blocking(move || read(read_key, cancellation)).await;
    let disposition = match candidate {
        Ok(Ok(image)) => publish(key, image, persist, &events).await,
        Ok(Err(reason)) => {
            let _closed = events.send(Event::Failed { key, reason }).await;
            Disposition::Discard
        }
        Err(_lost) => {
            let _closed = events
                .send(Event::Failed {
                    key,
                    reason: CheckpointError::StaleCapture,
                })
                .await;
            Disposition::Discard
        }
    };
    // Cleanup is deliberately awaited here, never by the session/deadline owner.
    let cleaned = tokio::task::spawn_blocking(move || cleanup(key, disposition)).await;
    if cleaned.is_ok() {
        let _closed = events.send(Event::Cleaned(key)).await;
    }
}
async fn publish<Persist, Stored>(
    key: CaptureKey,
    image: FileImage,
    persist: Arc<Persist>,
    events: &mpsc::Sender<Event>,
) -> Disposition
where
    Persist: Fn(CaptureKey, FileImage) -> Stored + Send + Sync + 'static,
    Stored: Future<Output = Result<(), CheckpointError>> + Send + 'static,
{
    let (permit, allowed) = oneshot::channel();
    if events.send(Event::Candidate { key, permit }).await.is_err() {
        return Disposition::Discard;
    }
    if !matches!(allowed.await, Ok(Ok(()))) {
        return Disposition::Discard;
    }
    let result = persist(key, image).await;
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
        return Disposition::Discard;
    }
    outcome.await.unwrap_or(Disposition::Discard)
}
