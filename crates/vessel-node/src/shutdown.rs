//! Stopping a node without losing anything.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_session::SessionRuntime;
use vessel_store::Store;

/// How long subscription forwarders get to queue their last frames (the
/// sessions' final records and `SubscriptionEnded`) before connections
/// close. They normally need far less; a forwarder stuck behind a client
/// that stopped reading is not waited for longer.
const FORWARDER_GRACE: Duration = Duration::from_secs(2);

/// Everything that must be stopped, in [`Teardown::run`]'s order.
pub(crate) struct Teardown {
    pub(crate) runtime: Arc<SessionRuntime>,
    /// Admitted settings writes must finish I/O and apply before stop returns.
    pub(crate) settings: Arc<crate::settings::Settings>,
    /// Session creations and persist tasks.
    pub(crate) tasks: TaskTracker,
    /// Subscription forwarders.
    pub(crate) forwarders: TaskTracker,
    /// Cancelling closes every RPC connection.
    pub(crate) closing: CancellationToken,
    pub(crate) connections: TaskTracker,
    pub(crate) store: Store,
}

impl Teardown {
    /// Called once the listener stopped accepting.
    ///
    /// 1. Seal and settle every accepted settings operation while runtime and
    ///    Store remain available.
    /// 2. End every session. Each publishes a final `Interrupted` status,
    ///    which live subscribers still receive.
    /// 3. Wait until every record is committed.
    /// 4. Give subscription forwarders a moment to queue the final records
    ///    and `SubscriptionEnded` (bounded by [`FORWARDER_GRACE`]).
    /// 5. Close connections; each flushes what it already queued (bounded by
    ///    the RPC close timeout).
    /// 6. Close the store, folding its WAL into the database file.
    pub(crate) async fn run(self) {
        self.settings.shutdown().await;
        tracing::info!("stopping sessions");
        self.runtime.shutdown().await;
        let _first_close = self.tasks.close();
        self.tasks.wait().await;
        tracing::debug!("session logs committed");
        let _first_close = self.forwarders.close();
        if tokio::time::timeout(FORWARDER_GRACE, self.forwarders.wait())
            .await
            .is_err()
        {
            tracing::debug!("some subscribers did not take their last records in time");
        }
        self.closing.cancel();
        let _first_close = self.connections.close();
        self.connections.wait().await;
        self.store.close().await;
        tracing::info!("node stopped");
    }
}

/// The process's stop requests (`Ctrl-C`, and `SIGTERM` on Unix, which
/// service managers send), listened for from the moment this is made.
///
/// [Decision Log] (2026-11-15, gate 45c)
/// - Purpose: a process that says it is ready (`relay listening`, `node
///   listening`, attach's readiness line) must already answer `SIGTERM` by
///   stopping cleanly; whoever read that line may send it at once.
/// - Before: `signal()` registered its handlers on its first poll, inside
///   the `select!` that followed the readiness line. A `SIGTERM` sent in
///   between met the default action and killed the process (the relay's
///   health-and-stop test, `signal: 15`, on a loaded build machine).
/// - Choice: [`StopSignals::listen`] registers the Unix handlers when it
///   is called; callers make it before they announce anything and await
///   [`StopSignals::stopped`] later. On other platforms `Ctrl-C` is still
///   listened for from the first poll, as before (no change there).
/// - Cost: the handlers replace the default action a little earlier, for
///   the whole start-up; a `SIGTERM` during start-up now stops the process
///   once it reaches the wait instead of killing it on the spot.
#[must_use = "the signals are only listened for while this is kept"]
#[derive(Debug)]
pub struct StopSignals {
    #[cfg(unix)]
    terminate: Option<tokio::signal::unix::Signal>,
    #[cfg(unix)]
    interrupt: Option<tokio::signal::unix::Signal>,
}

impl StopSignals {
    /// Start listening: the Unix handlers are in place when this returns.
    #[cfg(unix)]
    pub fn listen() -> Self {
        use tokio::signal::unix::{SignalKind, signal};
        let listen = |kind: SignalKind, name: &str| {
            signal(kind)
                .inspect_err(|error| tracing::warn!(%error, "cannot listen for {name}"))
                .ok()
        };
        Self {
            terminate: listen(SignalKind::terminate(), "SIGTERM"),
            interrupt: listen(SignalKind::interrupt(), "Ctrl-C"),
        }
    }

    /// Start listening (`Ctrl-C` is listened for from the first poll of
    /// [`stopped`](Self::stopped), as before).
    #[cfg(not(unix))]
    pub fn listen() -> Self {
        Self {}
    }

    /// Resolves when a stop is asked for.
    #[cfg(unix)]
    pub async fn stopped(mut self) {
        tokio::select! {
            () = received(self.terminate.as_mut()) => {}
            () = received(self.interrupt.as_mut()) => {}
        }
    }

    /// Resolves when a stop is asked for.
    #[cfg(not(unix))]
    pub async fn stopped(self) {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    }
}

/// One delivery of `signal`, or never when it could not be listened for.
#[cfg(unix)]
async fn received(signal: Option<&mut tokio::signal::unix::Signal>) {
    match signal {
        Some(signal) => {
            let _delivered = signal.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

/// Resolves when the process is asked to stop. Listens from its first
/// poll; a caller that announces readiness first uses [`StopSignals`].
pub async fn signal() {
    StopSignals::listen().stopped().await;
}
