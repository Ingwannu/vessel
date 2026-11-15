//! Pending data opens belong to one NodeLink run until accept or discard.
//!
//! [Decision Log]
//! - Purpose: stopping registration must not acknowledge before a late dial
//!   can hand another carrier to its accept callback.
//! - Constraint: accepted carriers already belong to the gateway and survive.
//! - Alternatives: detached tasks plus a token check cannot fence a callback
//!   already in progress; cancelling gateway tasks would close existing data.
//! - Choice: reserve capacity before spawn, cancel pending futures and join
//!   through their final callback on the same registration's shutdown.
//! - Why: run completion then proves no later handoff from this registration.
//! - Cost: excess announcements are not dialled and the relay's existing
//!   pending-client timeout handles them; the callback must remain nonblocking.
use super::STEP;
use crate::{
    ACCEPT_PATH,
    carrier::{Keepalive, WsCarrier},
    endpoint,
    reach::Reach,
    socket,
    tls::TlsConfig,
};
use std::{future::Future, sync::Arc};
use tokio::task::{JoinError, JoinSet};
use tokio_util::sync::CancellationToken;
use vessel_proto::{ConnId, RouteUrl};

/// Per-registration in-flight dials, including the final accept decision.
/// This is separate from the gateway's established-connection admission.
const MAX_PENDING_DATA_OPENS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Admission {
    Accepted,
    Full,
    Stopping,
}

pub(super) struct DataOpens {
    tasks: JoinSet<()>,
    stop: CancellationToken,
}
impl DataOpens {
    pub(super) fn new(registration: &CancellationToken) -> Self {
        Self {
            tasks: JoinSet::new(),
            stop: registration.child_token(),
        }
    }

    fn start<F, Work>(&mut self, work: F) -> Admission
    where
        F: FnOnce(CancellationToken) -> Work + Send + 'static,
        Work: Future<Output = ()> + Send + 'static,
    {
        while let Some(result) = self.tasks.try_join_next() {
            finished(result);
        }
        if self.stop.is_cancelled() {
            return Admission::Stopping;
        }
        if self.tasks.len() >= MAX_PENDING_DATA_OPENS {
            return Admission::Full;
        }
        let stop = self.stop.clone();
        // Admission precedes invoking the factory as well as spawning its work.
        drop(self.tasks.spawn(async move {
            tokio::select! {
                biased;
                () = stop.cancelled() => {},
                () = work(stop.clone()) => {},
            }
        }));
        Admission::Accepted
    }

    pub(super) fn open<F>(
        &mut self,
        relay: &RouteUrl,
        tls: &TlsConfig,
        reach: Reach,
        conn: ConnId,
        accept: Arc<F>,
    ) where
        F: Fn(WsCarrier) + Send + Sync + 'static,
    {
        let url = endpoint(relay, &format!("{ACCEPT_PATH}?conn={conn}"));
        let tls = tls.clone();
        let admitted = self.start(move |stop| async move {
            match tokio::time::timeout(STEP, socket::open_reaching(&url, &tls, reach)).await {
                Ok(Ok(socket)) => {
                    if !stop.is_cancelled() {
                        accept(WsCarrier::new(socket, Some(Keepalive::DEFAULT.silence)));
                    }
                }
                Ok(Err(error)) => tracing::debug!(%error, "could not accept a relayed client"),
                Err(_elapsed) => tracing::debug!("accepting a relayed client timed out"),
            }
        });
        match admitted {
            Admission::Accepted => {}
            Admission::Full => tracing::debug!("pending relay data-open capacity is full"),
            Admission::Stopping => {}
        }
    }

    /// Completion includes a callback already in progress when stop won.
    pub(super) async fn stop(&mut self) {
        self.stop.cancel();
        self.tasks.abort_all();
        while let Some(result) = self.tasks.join_next().await {
            finished(result);
        }
    }
}
impl Drop for DataOpens {
    fn drop(&mut self) {
        // Dropping run cannot acknowledge a normal stop, but still prevents
        // unstarted callbacks; JoinSet's drop aborts every remaining child.
        self.stop.cancel();
    }
}
fn finished(result: Result<(), JoinError>) {
    if let Err(error) = result {
        // A callback panic's text is not trusted log content. Keep closed
        // categories, never the panic payload or an unbounded source string.
        tracing::debug!(
            cancelled = error.is_cancelled(),
            panicked = error.is_panic(),
            "pending relay data open ended"
        );
    }
}

#[cfg(test)]
mod tests;
