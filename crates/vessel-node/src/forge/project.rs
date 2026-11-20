//! Project repository reads staged under cfg(test) for ADR 0061. The
//! transport change must remove that attribute only after shared protocol
//! admission and the connection budget are ready, then gate product wiring.
//!
//! One owner holds pages and flights. Every admitted caller retains its
//! permit through queued work, blocking root resolution and completion;
//! cancelling its receiver neither frees running work nor strands peers.

mod cache;
mod resolve;
#[cfg(test)]
mod tests;

use super::hosts::Hosts;
use crate::workspace::{WorkspaceError, Workspaces};
use cache::{Cache, Key, LoadId};
use resolve::Target;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::{
    ProjectId,
    forge::{ForgeAccess, OpenPullPage, ProjectPullCursor},
};

/// Bounds count every caller, including coalesced and cancelled waiters.
const MAX_ACTIVE: usize = 4;
/// Idle keys are evicted before accepting another; active keys never are.
const MAX_KEYS: usize = 16;
/// Paseo's lifetime; reusing a page preserves its original read_at.
const FRESH: Duration = Duration::from_secs(30);
type Admission = Arc<OwnedSemaphorePermit>;
type Outcome = Result<Arc<OpenPullPage>, Arc<Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Cached,
    Refresh,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("the project listing is busy")]
    Busy,
    #[error("the project listing stopped")]
    Stopped,
    #[error("the project folder cannot be used")]
    Workspace(#[source] WorkspaceError),
    #[error("the repository is unavailable")]
    Access(ForgeAccess),
    #[error("the cursor belongs to another repository")]
    CursorRepository,
    #[error("the selected credential changed during the read")]
    CredentialChanged,
    #[error("the provider refused the whole repository page")]
    Provider {
        access: ForgeAccess,
        #[source]
        source: vessel_forge::ForgeError,
    },
    #[error("the provider returned another repository or did not advance")]
    InconsistentPage,
}

#[derive(Debug)]
struct Request {
    project: ProjectId,
    cursor: Option<ProjectPullCursor>,
    mode: Mode,
    reply: oneshot::Sender<Outcome>,
    keep: Admission,
}

impl Request {
    fn answer(self, result: Outcome) {
        if self.reply.send(result).is_err() {
            tracing::debug!("a project read's caller went away");
        }
    }
}

#[derive(Debug)]
struct ProjectPulls {
    commands: mpsc::Sender<Request>,
    admission: Arc<Semaphore>,
    stop: CancellationToken,
}

impl ProjectPulls {
    fn start(
        workspaces: Arc<Workspaces>,
        hosts: Arc<Hosts>,
        tasks: &TaskTracker,
        stop: CancellationToken,
    ) -> Self {
        let (commands, received) = mpsc::channel(MAX_ACTIVE);
        let admission = Arc::new(Semaphore::new(MAX_ACTIVE));
        let actor = Actor {
            workspaces,
            hosts,
            received,
            jobs: JoinSet::new(),
            cache: Cache::new(),
        };
        drop(tasks.spawn(actor.run(stop.clone(), Arc::clone(&admission))));
        Self {
            commands,
            admission,
            stop,
        }
    }

    async fn read(
        &self,
        project: ProjectId,
        cursor: Option<ProjectPullCursor>,
        mode: Mode,
    ) -> Outcome {
        if self.stop.is_cancelled() {
            return Err(Arc::new(Error::Stopped));
        }
        let keep = Arc::new(
            Arc::clone(&self.admission)
                .try_acquire_owned()
                .map_err(|_| Arc::new(Error::Busy))?,
        );
        let (reply, answer) = oneshot::channel();
        let request = Request {
            project,
            cursor,
            mode,
            reply,
            keep,
        };
        self.commands
            .try_send(request)
            .map_err(|_| Arc::new(Error::Busy))?;
        answer
            .await
            .unwrap_or_else(|_| Err(Arc::new(Error::Stopped)))
    }
}

#[derive(Debug)]
enum Completion {
    Prepared(Request, Result<Target, Error>),
    Loaded {
        key: Key,
        id: LoadId,
        result: Outcome,
        finished: tokio::time::Instant,
        // Completed results retain admission until the actor handles them.
        keep: Admission,
    },
}

struct Actor {
    workspaces: Arc<Workspaces>,
    hosts: Arc<Hosts>,
    received: mpsc::Receiver<Request>,
    jobs: JoinSet<Completion>,
    cache: Cache,
}

impl Actor {
    async fn run(mut self, stop: CancellationToken, admission: Arc<Semaphore>) {
        loop {
            tokio::select! {
                () = stop.cancelled() => break,
                request = self.received.recv() => {
                    let Some(request) = request else { break; };
                    if stop.is_cancelled() { request.answer(Err(Arc::new(Error::Stopped))); break; }
                    let workspaces = Arc::clone(&self.workspaces);
                    let hosts = Arc::clone(&self.hosts);
                    drop(self.jobs.spawn(async move {
                        let target = resolve::prepare(&workspaces, &hosts, &request).await;
                        Completion::Prepared(request, target)
                    }));
                }
                completed = self.jobs.join_next(), if !self.jobs.is_empty() => {
                    match completed {
                        Some(Ok(completion)) if !stop.is_cancelled() => self.completed(completion),
                        Some(Ok(_)) | None => {},
                        Some(Err(error)) => tracing::warn!(%error, "a project read job stopped"),
                    }
                }
            }
        }
        admission.close();
        self.received.close();
        self.jobs.abort_all();
        self.cache.stop();
        while let Ok(request) = self.received.try_recv() {
            request.answer(Err(Arc::new(Error::Stopped)));
        }
        // Aborted async jobs release their requests. A queued/running
        // spawn_blocking root open still owns its independent guard.
        while let Some(completed) = self.jobs.join_next().await {
            if let Err(error) = completed
                && !error.is_cancelled()
            {
                tracing::warn!(%error, "a project read job failed during shutdown");
            }
        }
    }

    fn completed(&mut self, completion: Completion) {
        match completion {
            Completion::Prepared(request, Ok(target)) => self.accept(request, target),
            Completion::Prepared(request, Err(error)) => request.answer(Err(Arc::new(error))),
            Completion::Loaded {
                key,
                id,
                result,
                finished,
                keep,
            } => {
                self.cache.finish(&key, &id, result, finished);
                drop(keep);
            }
        }
    }

    fn accept(&mut self, request: Request, target: Target) {
        let Some(slot) = self.cache.slot(&target.key) else {
            request.answer(Err(Arc::new(Error::Busy)));
            return;
        };
        if let Some(page) = slot.reuse(request.mode) {
            request.answer(Ok(page));
            return;
        }
        if let Some(flight) = slot.joinable(request.mode) {
            flight.waiters.push(request);
            return;
        }
        let id = LoadId::new();
        let keep = Arc::clone(&request.keep);
        slot.begin(id.clone(), request);
        let hosts = Arc::clone(&self.hosts);
        drop(self.jobs.spawn(async move {
            let result = resolve::load(&hosts, &target)
                .await
                .map(Arc::new)
                .map_err(Arc::new);
            Completion::Loaded {
                key: target.key,
                id,
                result,
                finished: tokio::time::Instant::now(),
                keep,
            }
        }));
    }
}
