//! The protocol's `Handler`: routes each call to the module that owns it
//! and maps its typed error to the wire, once. No logic of its own. Every
//! transport serves it behind the scope check (`guard::Guarded`).

use std::sync::Arc;

use tokio_util::task::TaskTracker;
use vessel_harness::Lookup;
use vessel_proto::{
    Ack, DeviceInfo, GetNode, Harness, Invitation, InviteDevice, ListDevices, ListHarnesses,
    NodeInfo, ProtocolVersion, RevokeDevice, StopNode, WireError,
};
use vessel_rpc::{Outbound, Peer};

use crate::agents::Agents;
use crate::attachments::Attachments;
use crate::devices::Devices;
use crate::error::ServiceError;
use crate::fleet::{Home, Links};
use crate::lifecycle::Lifecycle;
use crate::models::Models;
use crate::principal::Principal;
use crate::probes::Probes;
use crate::profiles::NodeProfiles;
use crate::profiles::launch::Launcher;
use crate::sessions::Sessions;
use crate::tidying::Tidying;
use crate::transfers::Transfers;
use crate::waits::{ConnectionWaits, Share, WaitSlot, Waits};
use crate::workspace::Workspaces;

// The calls of protocols 2 to 8, by area.
mod fleet;
mod profiles;
mod sessions;
mod transfers;
// Protocol 9's calls: what a session runs with, and titles.
mod options;
// Protocol 10's calls: the conversation, and starting an ended session
// again.
mod again;
mod conversation;
// Protocol 11's calls: agents beyond the four (ADR 0032).
mod agents;
// Protocol 10's calls: projects, worktrees, git and files (`workspace`).
mod files;
mod git;
mod projects;
mod worktrees;

/// The node's protocol implementation.
#[derive(Debug)]
pub(crate) struct NodeService {
    pub(crate) lsp: vessel_lsp::Manager,
    pub(crate) lsp_roots: Vec<crate::LspRoot>,
    pub(crate) lifecycle: Lifecycle,
    lookup: Lookup,
    sessions: Arc<Sessions>,
    devices: Arc<Devices>,
    /// Subscription forwarders, so shutdown can let them queue their last
    /// frames.
    forwarders: TaskTracker,
    /// `profiles.toml` (ADR 0011).
    profiles: Arc<NodeProfiles>,
    /// `models.toml` and the model proxy's tokens (ADR 0015).
    models: Arc<Models>,
    /// What `session.create` adds to the spec.
    launcher: Launcher,
    /// Sessions' agents (ADR 0013): credentials, grants, limits.
    pub(crate) agents: Arc<Agents>,
    /// The fleet's home side (ADR 0012).
    fleet: Arc<Home>,
    /// The links to other nodes of the fleet (protocol 6, `envelope`).
    pub(crate) links: Arc<Links>,
    /// Open transfers (protocol 6).
    transfers: Arc<Transfers>,
    /// Uploads and attachments (protocol 10).
    attachments: Arc<Attachments>,
    /// Waiting reads by origin (`waits`).
    waits: Arc<Waits>,
    /// What agents offer, probed and kept (protocol 9, `probes`).
    probes: Probes,
    /// Projects, worktrees, git and files (protocol 10, `workspace`).
    workspaces: Arc<Workspaces>,
    /// Deleting sessions with their worktrees (protocol 10, `tidying`).
    tidying: Arc<Tidying>,
}

/// The parts a [`NodeService`] serves.
pub(crate) struct Parts {
    pub(crate) lsp: vessel_lsp::Manager,
    pub(crate) lsp_roots: Vec<crate::LspRoot>,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) lookup: Lookup,
    pub(crate) sessions: Arc<Sessions>,
    pub(crate) devices: Arc<Devices>,
    pub(crate) forwarders: TaskTracker,
    pub(crate) profiles: Arc<NodeProfiles>,
    pub(crate) models: Arc<Models>,
    pub(crate) launcher: Launcher,
    pub(crate) agents: Arc<Agents>,
    pub(crate) fleet: Arc<Home>,
    pub(crate) links: Arc<Links>,
    pub(crate) transfers: Arc<Transfers>,
    pub(crate) attachments: Arc<Attachments>,
    pub(crate) probes: Probes,
    pub(crate) workspaces: Arc<Workspaces>,
    pub(crate) tidying: Arc<Tidying>,
}

impl NodeService {
    pub(crate) fn new(parts: Parts) -> Self {
        let Parts {
            lsp,
            lsp_roots,
            lifecycle,
            lookup,
            sessions,
            devices,
            forwarders,
            profiles,
            models,
            launcher,
            agents,
            fleet,
            links,
            transfers,
            attachments,
            probes,
            workspaces,
            tidying,
        } = parts;
        Self {
            lsp,
            lsp_roots,
            lifecycle,
            lookup,
            sessions,
            devices,
            forwarders,
            profiles,
            models,
            launcher,
            agents,
            fleet,
            links,
            transfers,
            attachments,
            waits: Arc::default(),
            probes,
            workspaces,
            tidying,
        }
    }

    /// A place for the wait of a read on `connection` in `shares`, if it
    /// waits and the connection and every share have room (`waits`);
    /// otherwise the wait is dropped and the read answers at once.
    pub(crate) fn wait_slot(
        &self,
        connection: &Connection,
        shares: Vec<Share>,
        wait: &mut Option<u32>,
    ) -> Option<WaitSlot> {
        let _seconds = wait.as_ref()?;
        let slot = self.waits.take(&connection.waiting, shares);
        if slot.is_none() {
            *wait = None;
            tracing::debug!(
                "a waiting read answers at once: its connection's half, or one of its shares \
                 (origin, node, relaying device), is taken"
            );
        }
        slot
    }
}

/// Per-connection state: who is calling, and where this connection's
/// subscriptions go.
#[derive(Debug)]
pub(crate) struct Connection {
    outbound: Outbound,
    principal: Principal,
    /// The protocol the client negotiated: an older client is never handed
    /// what it cannot decode (protocol 11's added agents).
    protocol: ProtocolVersion,
    /// Its waiting reads, against its share (`waits`).
    waiting: ConnectionWaits,
}

impl Connection {
    pub(crate) fn new(peer: Peer, principal: Principal) -> Self {
        tracing::info!(
            client = %peer.client.name,
            version = %peer.client.version,
            ?principal,
            "client connected"
        );
        Self {
            outbound: peer.outbound,
            principal,
            protocol: peer.protocol,
            waiting: ConnectionWaits::default(),
        }
    }

    /// A connection of the machine's own clients (token or local socket).
    pub(crate) fn owner(peer: Peer) -> Self {
        Self::new(peer, Principal::Owner)
    }

    /// Who is calling.
    pub(crate) const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// The protocol the client speaks.
    pub(crate) const fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }

    /// This connection, calling as `principal`: a call delivered from
    /// another node runs as its delegated principal (protocol 6).
    pub(crate) fn acting_as(&self, principal: Principal) -> Self {
        Self {
            outbound: self.outbound.clone(),
            principal,
            protocol: self.protocol,
            waiting: Arc::clone(&self.waiting),
        }
    }
}

fn wire(error: &ServiceError) -> WireError {
    error.to_wire()
}

/// The calls, one per protocol method. Inherent, not a `Handler`: only
/// [`Guarded`](crate::guard::Guarded) implements `Handler`, so the service
/// cannot be served without the scope check.
impl NodeService {
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn node_info(
        &self,
        _: &Connection,
        _: GetNode,
    ) -> Result<NodeInfo, WireError> {
        Ok(self.lifecycle.info())
    }

    pub(crate) async fn list_harnesses(
        &self,
        _: &Connection,
        _: ListHarnesses,
    ) -> Result<Vec<Harness>, WireError> {
        Ok(vessel_harness::discover(&self.lookup).await)
    }

    /// `node.stop` (protocol 3, ADR 0008): stop now, or drain and stop once
    /// idle. The `Ack` reaches the client even though `now` starts the
    /// shutdown inside this call: a closing connection still answers the
    /// calls already running (`vessel-rpc`, `server::calls`).
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn stop_node(
        &self,
        _: &Connection,
        request: StopNode,
    ) -> Result<Ack, WireError> {
        self.lifecycle.stop(request.when);
        Ok(Ack)
    }

    /// `device.invite` (ADR 0010): a single-use pairing link.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn invite_device(
        &self,
        _: &Connection,
        request: InviteDevice,
    ) -> Result<Invitation, WireError> {
        self.devices.invite(&request).map_err(|error| wire(&error))
    }

    pub(crate) async fn list_devices(
        &self,
        _: &Connection,
        _: ListDevices,
    ) -> Result<Vec<DeviceInfo>, WireError> {
        self.devices.list().await.map_err(|error| wire(&error))
    }

    /// `device.revoke`: also closes the device's open connections, this
    /// one included if a device revokes itself (after its `ack`).
    pub(crate) async fn revoke_device(
        &self,
        _: &Connection,
        request: RevokeDevice,
    ) -> Result<Ack, WireError> {
        self.devices
            .revoke(request.device)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }
}
