//! The dedicated-thread/oneshot handoff for one local lifecycle observation.
//! Filesystem rules stay in callers; a command never joins a blocked worker.

use std::io;
use tokio::sync::oneshot;

/// A closed operation name keeps thread diagnostics free of external text.
pub(super) enum Kind {
    /// Bind observations to the selected node identity.
    Identity,
    /// Only an authenticated directory may supply a service observation.
    ServiceFile,
    /// Opening a lock or its owner must not delay runtime shutdown.
    Preflight,
    /// Preserve the existing OS-lock waiter's thread name.
    LockRelease,
}

impl Kind {
    const fn name(self) -> &'static str {
        match self {
            Self::ServiceFile => "node-service-reader",
            Self::Identity => "node-identity-reader",
            Self::Preflight => "node-preflight-reader",
            Self::LockRelease => "node-lock-release",
        }
    }
}

/// Borrowing the receive preserves the existing waiter's cancellation rule.
pub(super) struct Pending<T> {
    receive: oneshot::Receiver<T>,
}

impl<T: Send + 'static> Pending<T> {
    /// Keep the existing detached worker policy separate from its file operation.
    ///
    /// # Errors
    /// The OS refuses to create the worker; the original I/O cause is preserved.
    pub(super) fn spawn(
        kind: Kind,
        work: impl FnOnce() -> T + Send + 'static,
        on_closed: impl FnOnce() + Send + 'static,
    ) -> io::Result<Self> {
        let (send, receive) = oneshot::channel();
        let waiter = std::thread::Builder::new()
            .name(kind.name().to_owned())
            .spawn(move || {
                let result = work();
                // The caller retains its diagnostic and tracing target when
                // the command intentionally ends before this worker does.
                if send.send(result).is_err() {
                    on_closed();
                }
            })?;
        drop(waiter);
        Ok(Self { receive })
    }

    /// The same receiver remains available after cancellation of this borrow.
    ///
    /// # Errors
    /// The worker ended without sending its result.
    pub(super) async fn wait(&mut self) -> Result<T, oneshot::error::RecvError> {
        (&mut self.receive).await
    }
}
