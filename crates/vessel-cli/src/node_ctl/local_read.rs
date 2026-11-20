//! One-off local observation work must not keep this CLI alive past its deadline.
//! Own only thread/channel handoff; filesystem and identity rules stay in callers.

use std::io;
use tokio::sync::oneshot;

/// Closed operation names keep thread diagnostics free of external text.
pub(super) enum Kind {
    /// The node's stored identity binds local observations to its greeting.
    Identity,
    /// The existing OS lock supplies shutdown completion without polling.
    LockRelease,
}

impl Kind {
    const fn name(self) -> &'static str {
        match self {
            Self::Identity => "node-identity-reader",
            Self::LockRelease => "node-lock-release",
        }
    }
}

/// A borrowed receive survives a timeout while its dedicated worker stays blocked.
pub(super) struct Pending<T> {
    receive: oneshot::Receiver<T>,
}

impl<T: Send + 'static> Pending<T> {
    /// A dedicated thread ends with the process instead of delaying Tokio shutdown.
    ///
    /// # Errors
    /// The OS refuses to create the worker; the original I/O cause is preserved.
    pub(super) fn spawn(kind: Kind, work: impl FnOnce() -> T + Send + 'static) -> io::Result<Self> {
        let (send, receive) = oneshot::channel();
        let worker = std::thread::Builder::new()
            .name(kind.name().to_owned())
            .spawn(move || {
                let result = work();
                if send.send(result).is_err() {
                    tracing::debug!("local observation completed after its command ended");
                }
            })?;
        drop(worker);
        Ok(Self { receive })
    }

    /// Retry only cancellation; a successful receive consumes this one result.
    ///
    /// # Errors
    /// The worker ended without sending its result. Callers must stop on failure.
    pub(super) async fn wait(&mut self) -> Result<T, oneshot::error::RecvError> {
        (&mut self.receive).await
    }
}
