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
/// feature adapters still retain cloned `RuntimeHandle` values. Explicit shutdown joins
/// cleanup; a failed reap cannot be reported as successful disable/remove.
#[derive(Debug)]
pub struct Supervisor {
    handle: RuntimeHandle,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<Result<vessel_proc::OwnedGroupExit, RuntimeError>>,
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
        // Ordinary proc on the shared node head has no release-runtime guard.
        // Refuse here before Tokio's process API can panic or spawn a child.
        let runtime = tokio::runtime::Handle::try_current().map_err(RuntimeError::Runtime)?;
        let spec = command.into_spec();
        let spawned = vessel_proc::spawn(&spec).map_err(RuntimeError::Spawn)?;
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
        let task = runtime.spawn(actor.run());
        Ok(Self { handle, stop, task })
    }

    /// Typed callbacks/catalog observation; no install/settings authority.
    #[must_use]
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    /// Withdraw, cancel and join pipe workers after checked owned cleanup.
    /// The receipt proves leader reap and accepted first cleanup request, not
    /// observed death of every descendant or escaped process.
    ///
    /// # Errors
    /// Reports process/protocol/cleanup failures; never acknowledges an unproved
    /// reap as a successful lifecycle change.
    pub async fn shutdown(self) -> Result<vessel_proc::OwnedGroupExit, RuntimeError> {
        self.stop.cancel();
        // Borrowing the task preserves cancellation-on-owner-drop while await
        // remains cancel safe; no handle is moved out of this Drop owner.
        let mut owner = self;
        (&mut owner.task)
            .await
            .map_err(|cause| RuntimeError::Task(TaskFailure::new(cause)))?
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
    async fn run(self) -> Result<vessel_proc::OwnedGroupExit, RuntimeError> {
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
        let Wired {
            mut child,
            writer,
            mut messages,
            mut workers,
        } = Wired::new(spawned, &logs);
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
        finish(
            &mut core,
            &mut child,
            &mut requests,
            &mut workers,
            &writer,
            result,
        )
        .await
    }
}

/// Pipe ownership is prepared once; dropping it aborts workers and retains
/// Child's ordinary group cleanup guard even if initialization refuses.
struct Wired {
    child: vessel_proc::Child,
    writer: mpsc::Sender<NodeToHelper>,
    messages: mpsc::Receiver<Option<vessel_proto::plugins::HelperToNode>>,
    workers: Workers,
}

impl Wired {
    fn new(spawned: vessel_proc::Spawned, logs: &SharedLogs) -> Self {
        let vessel_proc::Spawned {
            child,
            stdin,
            stdout,
            stderr,
        } = spawned;
        let (writer, outgoing) = mpsc::channel(PIPE_QUEUE);
        let (incoming, messages) = mpsc::channel(PIPE_QUEUE);
        let mut workers = Workers::new();
        workers::reader(&mut workers, stdout, incoming);
        workers::writer(&mut workers, stdin, outgoing);
        workers::stderr(&mut workers, stderr, logs.clone());
        Self {
            child,
            writer,
            messages,
            workers,
        }
    }
}

async fn finish(
    core: &mut Core,
    child: &mut vessel_proc::Child,
    requests: &mut Requests,
    workers: &mut Workers,
    writer: &mpsc::Sender<NodeToHelper>,
    result: Result<(), RuntimeError>,
) -> Result<vessel_proc::OwnedGroupExit, RuntimeError> {
    core.retire(
        result
            .as_ref()
            .err()
            .map_or(CallError::Closed, RuntimeError::call_error),
    );
    requests.close();
    // Unissued work can release permits now; Core keeps issued admission
    // until the permanent actor retirement and checked cleanup below.
    while let Ok(request) = requests.try_recv() {
        drop(request.reply.send(Err(CallError::Closed)));
    }
    let cleanup = shutdown::reap(child, writer).await;
    let drained = workers.drain().await;
    let receipt = cleanup?;
    drained?;
    result.map(|()| receipt)
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
        let ready_by = tokio::time::Instant::now()
            .checked_add(REQUEST_DEADLINE)
            .ok_or(RuntimeError::ReadyTimeout)?;
        loop {
            let deadline = if self.core.is_ready() {
                self.core.deadline()
            } else {
                Some(ready_by)
            };
            if self.stop.is_cancelled() {
                return Ok(());
            }
            tokio::select! {
                () = self.stop.cancelled() => return Ok(()),
                request = self.requests.recv() => match request {
                    Some(request) => self.core.admit(request)?, None => return Ok(()),
                },
                message = self.messages.recv() => {
                    let message = message.flatten().ok_or(RuntimeError::EndOfStream)?;
                    accept_message(self.core, self.logs, message, ready_by)?;
                },
                completed = self.workers.join_next() => match completed {
                    Some(Ok(Ok(WorkerExit::Reader | WorkerExit::Stderr))) => {},
                    Some(Ok(Ok(WorkerExit::Writer))) | None => return Err(RuntimeError::EndOfStream),
                    Some(Ok(Err(error))) => return Err(error),
                    Some(Err(cause)) => return Err(RuntimeError::Task(TaskFailure::new(cause))),
                },
                code = self.child.wait() => return Err(RuntimeError::Exited { code: code.map_err(RuntimeError::Wait)? }),
                () = self.cancelled.notified() => self.core.cancellations()?,
                () = sleep_until(deadline) => return Err(if self.core.is_ready() { RuntimeError::CallTimeout } else { RuntimeError::ReadyTimeout }),
            }
        }
    }
}

fn accept_message(
    core: &mut Core,
    logs: &SharedLogs,
    message: vessel_proto::plugins::HelperToNode,
    ready_by: tokio::time::Instant,
) -> Result<(), RuntimeError> {
    // An elapsed timer and a completed pipe can be ready in the same select
    // turn. Initial capability must never publish merely because the pipe won.
    if !core.is_ready() && tokio::time::Instant::now() >= ready_by {
        return Err(RuntimeError::ReadyTimeout);
    }
    logs.helper_diagnostic(&message)?;
    core.receive(message)
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

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
