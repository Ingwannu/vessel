//! One actor owns a helper, all pending replies and its process-group lifetime.

use std::{future, sync::Arc};

use tokio::sync::{Notify, Semaphore, mpsc, watch};
use tokio_util::sync::CancellationToken;
use vessel_proto::plugins::{HelperProtocol, NodeToHelper, PluginAddress};

use crate::{
    actor_core::Core,
    handle::RuntimeHandle,
    launch::PreparedCommand,
    runtime_error::{RuntimeError, TaskFailure},
    runtime_types::{
        CallError, MAX_ADMITTED_BYTES, MAX_PENDING_CALLS, PIPE_QUEUE, REQUEST_DEADLINE, Requests,
        RuntimeState,
    },
    shared_logs::SharedLogs,
    shutdown,
    workers::{self, WorkerExit, Workers},
};

/// Node-owned supervision. Dropping this owner requests cleanup even if stale
/// feature adapters still retain cloned RuntimeHandles. Explicit shutdown joins
/// cleanup; a failed reap cannot be reported as successful disable/remove.
#[derive(Debug)]
pub struct Supervisor {
    handle: RuntimeHandle,
    stop: CancellationToken,
    task: Option<tokio::task::JoinHandle<Result<(), RuntimeError>>>,
}

impl Supervisor {
    /// Spawn only a node-prepared trusted command under vessel-proc's group/job
    /// owner. This library does not install, download or choose a runtime.
    ///
    /// # Errors
    /// Returns the concrete spawn cause. Ready is a later atomic state change;
    /// initialization failure never publishes an empty successful catalog.
    pub fn spawn(
        command: PreparedCommand,
        address: PluginAddress,
        logs: SharedLogs,
        parent: &CancellationToken,
    ) -> Result<Self, RuntimeError> {
        if parent.is_cancelled() {
            return Err(RuntimeError::Stopping);
        }
        let spawned = vessel_proc::spawn(command.spec()).map_err(RuntimeError::Spawn)?;
        let (requests, receive) = mpsc::channel(MAX_PENDING_CALLS);
        let (published, state) = watch::channel(RuntimeState::Starting);
        let cancelled = Arc::new(Notify::new());
        let stop = parent.child_token();
        let handle = RuntimeHandle {
            requests,
            state,
            cancelled: Arc::clone(&cancelled),
            bytes: Arc::new(Semaphore::new(MAX_ADMITTED_BYTES)),
            calls: Arc::new(Semaphore::new(MAX_PENDING_CALLS)),
        };
        let actor = Actor {
            spawned,
            requests: receive,
            address,
            logs,
            cancelled,
            stop: stop.clone(),
            published,
        };
        let task = tokio::spawn(actor.run());
        Ok(Self {
            handle,
            stop,
            task: Some(task),
        })
    }

    /// Typed callbacks/catalog observation; no install/settings authority.
    #[must_use]
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    /// Withdraw, cancel and reap the entire owned group, then join pipe workers.
    ///
    /// # Errors
    /// Reports process/protocol/cleanup failures; never acknowledges an unproved
    /// reap as a successful lifecycle change.
    pub async fn shutdown(mut self) -> Result<(), RuntimeError> {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.await
                .map_err(|cause| RuntimeError::Task(TaskFailure::new(cause)))?
        } else {
            Ok(())
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

struct Actor {
    spawned: vessel_proc::Spawned,
    requests: Requests,
    address: PluginAddress,
    logs: SharedLogs,
    cancelled: Arc<Notify>,
    stop: CancellationToken,
    published: watch::Sender<RuntimeState>,
}

impl Actor {
    async fn run(self) -> Result<(), RuntimeError> {
        let Self {
            spawned,
            mut requests,
            address,
            logs,
            cancelled,
            stop,
            published,
        } = self;
        let _withdraw = WithdrawOnDrop(published.clone());
        let vessel_proc::Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        } = spawned;
        let (writer, outgoing) = mpsc::channel(PIPE_QUEUE);
        let (incoming, mut messages) = mpsc::channel(PIPE_QUEUE);
        let mut workers = Workers::new();
        workers::reader(&mut workers, stdout, incoming);
        workers::writer(&mut workers, stdin, outgoing);
        workers::stderr(&mut workers, stderr, logs.clone());
        let mut core = Core::new(address.clone(), writer.clone(), published);
        writer
            .try_send(NodeToHelper::Initialize {
                protocol: HelperProtocol::V1,
                installation: address,
            })
            .map_err(|_closed_or_full| RuntimeError::EndOfStream)?;
        let result = Live {
            core: &mut core,
            child: &mut child,
            requests: &mut requests,
            messages: &mut messages,
            workers: &mut workers,
            logs: &logs,
            cancelled: &cancelled,
            stop: &stop,
        }
        .run()
        .await;
        core.retire(
            result
                .as_ref()
                .err()
                .map_or(CallError::Closed, RuntimeError::call_error),
        );
        requests.close();
        // Queued, unissued work may release its byte permits immediately; issued
        // callbacks keep theirs in Core until the owned child is reaped below.
        while let Ok(request) = requests.try_recv() {
            drop(request.reply.send(Err(CallError::Closed)));
        }
        let cleanup = shutdown::reap(&mut child, &writer).await;
        workers.abort_all();
        while workers.join_next().await.is_some() {}
        cleanup?;
        result
    }
}

struct Live<'a> {
    core: &'a mut Core,
    child: &'a mut vessel_proc::Child,
    requests: &'a mut Requests,
    messages: &'a mut mpsc::Receiver<Option<vessel_proto::plugins::HelperToNode>>,
    workers: &'a mut Workers,
    logs: &'a SharedLogs,
    cancelled: &'a Notify,
    stop: &'a CancellationToken,
}

impl Live<'_> {
    async fn run(self) -> Result<(), RuntimeError> {
        let Self {
            core,
            child,
            requests,
            messages,
            workers,
            logs,
            cancelled,
            stop,
        } = self;
        let ready_by = tokio::time::Instant::now()
            .checked_add(REQUEST_DEADLINE)
            .ok_or(RuntimeError::ReadyTimeout)?;
        loop {
            let deadline = if core.is_ready() {
                core.deadline()
            } else {
                Some(ready_by)
            };
            if stop.is_cancelled() {
                return Ok(());
            }
            tokio::select! {
                () = stop.cancelled() => return Ok(()),
                request = requests.recv() => match request {
                    Some(request) => core.admit(request)?, None => return Ok(()),
                },
                message = messages.recv() => {
                    let message = message.flatten().ok_or(RuntimeError::EndOfStream)?;
                    logs.helper_diagnostic(&message)?;
                    core.receive(message)?;
                },
                completed = workers.join_next() => match completed {
                    Some(Ok(Ok(WorkerExit::Reader | WorkerExit::Stderr))) => {},
                    Some(Ok(Ok(WorkerExit::Writer))) | None => return Err(RuntimeError::EndOfStream),
                    Some(Ok(Err(error))) => return Err(error),
                    Some(Err(cause)) => return Err(RuntimeError::Task(TaskFailure::new(cause))),
                },
                code = child.wait() => return Err(RuntimeError::Exited { code: code.map_err(RuntimeError::Wait)? }),
                () = cancelled.notified() => core.cancellations()?,
                () = sleep_until(deadline) => return Err(if core.is_ready() { RuntimeError::CallTimeout } else { RuntimeError::ReadyTimeout }),
            }
        }
    }
}

async fn sleep_until(deadline: Option<tokio::time::Instant>) {
    if let Some(deadline) = deadline {
        tokio::time::sleep_until(deadline).await;
    } else {
        future::pending::<()>().await;
    }
}

struct WithdrawOnDrop(watch::Sender<RuntimeState>);

impl Drop for WithdrawOnDrop {
    fn drop(&mut self) {
        drop(self.0.send_replace(RuntimeState::Stopped));
    }
}
