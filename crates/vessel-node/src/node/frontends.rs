//! Assemble the shared connection/shutdown context from the already started
//! core. Keeping these handles together prevents one frontend from receiving
//! different stop/admission ownership and keeps node start focused on ordering.

use super::Core;
use crate::{
    ProviderRegistrations,
    agents::Agents,
    config::Config,
    guard::Guarded,
    lifecycle::Lifecycle,
    relays::Relays,
    secure::{MAX_UNADMITTED, SecureGateway},
    shutdown::Teardown,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_rpc::Server;
use vessel_secure::NodeKeypair;
use vessel_store::Store;

pub(super) struct Frontends {
    pub(super) rpc: Arc<Server<Guarded>>,
    pub(super) secure: SecureGateway,
    pub(super) teardown: Teardown,
    pub(super) connections: TaskTracker,
    pub(super) agents: Arc<Agents>,
    pub(super) relays: Arc<Relays>,
    pub(super) lifecycle: Lifecycle,
    pub(super) stop: CancellationToken,
    pub(super) provider_registrations: ProviderRegistrations,
}

/// Move the same core handles into the frontends and their ordered teardown.
/// This performs no I/O or task start; bootstrap's async ordering is unchanged.
pub(super) fn prepare(
    config: &Config,
    core: Core,
    keypair: Arc<NodeKeypair>,
    store: Store,
) -> Frontends {
    let Core {
        runtime,
        lifecycle,
        stop,
        tasks,
        forwarders,
        devices,
        agents,
        service,
        relays,
    } = core;
    let closing = CancellationToken::new();
    let connections = TaskTracker::new();
    let rpc = Arc::new(Server::new(
        Arc::clone(&service),
        lifecycle.watch_info(),
        config.rpc.clone(),
        closing.clone(),
    ));
    let provider_registrations = service.0.providers.registrations();
    let secure = SecureGateway {
        keypair,
        channel: vessel_secure::Config::default(),
        service,
        lifecycle: lifecycle.clone(),
        devices,
        config: config.rpc.clone(),
        closing: closing.clone(),
        connections: connections.clone(),
        unadmitted: Arc::new(Semaphore::new(MAX_UNADMITTED)),
        unadmitted_per_address: Arc::default(),
    };
    let teardown = Teardown {
        runtime,
        tasks,
        forwarders,
        closing,
        connections: connections.clone(),
        store,
    };
    Frontends {
        rpc,
        secure,
        teardown,
        connections,
        agents,
        relays,
        lifecycle,
        stop,
        provider_registrations,
    }
}
