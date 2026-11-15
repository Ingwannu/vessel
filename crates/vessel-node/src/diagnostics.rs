//! Bounded content-free diagnostic assembly; source owners keep decoding.
//! Admission leases outlive caller cancellation and blocking jobs retain them.
//! The report contains no source paths, free messages or credentials (ADR 0074).
mod admission;
#[cfg(test)]
mod authority;
mod collect;
#[cfg(test)]
mod deadline;
mod inventory;
mod journal;
#[cfg(test)]
mod lifetime;
mod process;
mod sources;

use crate::{profiles::NodeProfiles, readiness::Readiness, relays::Relays};
use admission::Admission;
use chrono::{DateTime, Utc};
use journal::Journal;
use std::sync::{Arc, Weak};
use tokio::{sync::watch, time::Instant};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_harness::Lookup;
use vessel_proto::{
    Method, MethodShape, WireError,
    diagnostics::{DiagnosticBinding, DiagnosticListeners, DiagnosticLocalKind},
};
use vessel_session::SessionRuntime;
use vessel_store::Store;

/// All asynchronous sources share one response deadline. It does not kill I/O.
pub(crate) const RESPONSE_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

/// Startup owns clock, task lifetime and runtime; service owns remaining sources.
pub(crate) struct Setup {
    pub(crate) started: Instant,
    pub(crate) tasks: TaskTracker,
    pub(crate) stop: CancellationToken,
    pub(crate) runtime: Arc<SessionRuntime>,
    pub(crate) relays: Weak<Relays>,
}

#[derive(Debug)]
pub(crate) struct State {
    admission: Admission,
    journal: Journal,
    started: Instant,
    stop: CancellationToken,
    runtime: Arc<SessionRuntime>,
    relays: Weak<Relays>,
    listeners: watch::Sender<(DateTime<Utc>, DiagnosticListeners)>,
    lookup: Lookup,
    profiles: Arc<NodeProfiles>,
    readiness: Arc<Readiness>,
    store: Store,
}
impl State {
    pub(crate) fn new(
        setup: Setup,
        lookup: Lookup,
        profiles: Arc<NodeProfiles>,
        readiness: Arc<Readiness>,
        store: Store,
    ) -> Self {
        let initial = DiagnosticListeners {
            local_kind: if cfg!(windows) {
                DiagnosticLocalKind::Pipe
            } else {
                DiagnosticLocalKind::Socket
            },
            local: DiagnosticBinding::Starting,
            direct: DiagnosticBinding::Starting,
        };
        Self {
            admission: Admission::new(&setup.stop, &setup.tasks),
            journal: Journal::new(&setup.stop, &setup.tasks),
            started: setup.started,
            stop: setup.stop,
            runtime: setup.runtime,
            relays: setup.relays,
            listeners: watch::channel((Utc::now(), initial)).0,
            lookup,
            profiles,
            readiness,
            store,
        }
    }
    /// Borrow the actual successful bind result before serving. Neither the
    /// endpoint nor the listener is copied or retained in report state.
    pub(crate) fn bound(&self, direct: Option<&(tokio::net::TcpListener, std::net::SocketAddr)>) {
        self.listeners.send_modify(|(at, state)| {
            *at = Utc::now();
            state.local = DiagnosticBinding::Bound;
            state.direct = if direct.is_some() {
                DiagnosticBinding::Bound
            } else {
                DiagnosticBinding::Disabled
            };
        });
    }
    /// Observe a completed local call once, excluding forwarding wrappers.
    pub(crate) fn completed<T>(&self, method: Method, result: &Result<T, WireError>) {
        match method.shape() {
            MethodShape::Envelope => {}
            MethodShape::Unary | MethodShape::Stream => {
                if let Err(error) = result {
                    self.journal.observe(error);
                }
            }
        }
    }
}

#[cfg(test)]
mod machines;
#[cfg(test)]
mod tests;
