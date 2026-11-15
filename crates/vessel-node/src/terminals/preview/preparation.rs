//! A physical inspection owns its socket and permits even after caller timeout.

use std::sync::Arc;

use tokio::{net::TcpStream, sync::OwnedSemaphorePermit};
use tokio_util::task::task_tracker::TaskTrackerToken;

use super::{
    PreviewBudget, PreviewConnection, PreviewContext, PreviewError, PreviewLease, PreviewOrigin,
    budget, lease::Inner,
};

/// Socket fields drop before this token, so drain never precedes socket release.
pub(super) struct Attempt {
    pub(super) lease: Arc<Inner>,
    _connection: OwnedSemaphorePermit,
    _per_lease: OwnedSemaphorePermit,
    _drain: TaskTrackerToken,
}

struct Worker {
    socket: std::net::TcpStream,
    attempt: Attempt,
    _preparation: OwnedSemaphorePermit,
}

struct Prepared {
    socket: std::net::TcpStream,
    attempt: Attempt,
}

pub(super) async fn connect(
    lease: &PreviewLease,
    context: &PreviewContext,
    origin: PreviewOrigin,
) -> Result<PreviewConnection, PreviewError> {
    let (attempt, preparation) = register(lease, context, origin).await?;
    let socket = TcpStream::connect(origin.endpoint())
        .await
        .map_err(PreviewError::Socket)?;
    attempt.lease.validate()?;
    let worker = Worker {
        socket: socket.into_std().map_err(PreviewError::Socket)?,
        attempt,
        _preparation: preparation,
    };
    let task = lease
        .0
        .runtime
        .tasks
        .spawn_blocking(move || worker.inspect());
    // Dropping this JoinHandle does not drop the closure's actual owners.
    let prepared = task.await.map_err(PreviewError::Worker)??;
    prepared.attempt.lease.validate()?;
    let socket = TcpStream::from_std(prepared.socket).map_err(PreviewError::Socket)?;
    Ok(PreviewConnection::new(socket, prepared.attempt))
}

async fn register(
    lease: &PreviewLease,
    context: &PreviewContext,
    origin: PreviewOrigin,
) -> Result<(Attempt, OwnedSemaphorePermit), PreviewError> {
    let inner = &lease.0;
    let _gate = inner.gate.lock().await;
    inner.check_context(context, origin)?;
    inner.validate()?;
    let connection = budget::take(&inner.runtime.connections, PreviewBudget::Connections)?;
    let per_lease = budget::take(&inner.connections, PreviewBudget::LeaseConnections)?;
    let preparation = budget::take(&inner.runtime.preparations, PreviewBudget::Preparation)?;
    let attempt = Attempt {
        lease: Arc::clone(inner),
        _connection: connection,
        _per_lease: per_lease,
        _drain: inner.attempts.token(),
    };
    Ok((attempt, preparation))
}

impl Worker {
    fn inspect(self) -> Result<Prepared, PreviewError> {
        self.attempt.lease.validate()?;
        self.attempt
            .lease
            .source
            .inspect(&self.socket, &self.attempt.lease.binding)?;
        self.attempt.lease.validate()?;
        // Destruction order on refusal is socket, attempt, preparation. Success
        // moves the first two owners together; physical inspection is now over.
        Ok(Prepared {
            socket: self.socket,
            attempt: self.attempt,
        })
    }
}
