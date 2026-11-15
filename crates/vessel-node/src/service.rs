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
use crate::automation::notices::Notices;
use crate::automation::prompts::SavedPrompts;
use crate::automation::schedules::Schedules;
use crate::connection_life::ConnectionLife;
use crate::devices::Devices;
use crate::error::ServiceError;
use crate::fleet::{Home, Links};
use crate::forge::Forges;
use crate::lifecycle::Lifecycle;
use crate::models::Models;
use crate::principal::Principal;
use crate::probes::Probes;
use crate::profiles::NodeProfiles;
use crate::profiles::launch::Launcher;
use crate::sessions::Sessions;
use crate::terminals::{Ports, Terminals};
use crate::tidying::Tidying;
use crate::transfers::Transfers;
use crate::waits::{ConnectionWaits, Share, WaitSlot, Waits};
use crate::workspace::{Occupant, Workspaces};

// The calls of protocols 2 to 8, by area.
mod discovery;
mod export_cleanup;
mod fleet;
mod fleet_control;
mod profiles;
mod sessions;
mod subagents;
mod transfers;
// Protocol 9's calls: what a session runs with, and titles.
mod options;
// Protocol 10's calls: the conversation, and starting an ended session
// again.
mod again;
mod conversation;
// Protocol 11's calls: agents beyond the four (ADR 0032).
mod agents;
mod status;
// Protocol 10's calls: projects, worktrees, git and files (`workspace`).
mod files;
mod git;
mod projects;
mod registry;
mod worktrees;
// Protocol 11's calls: pull requests (`forge`, ADR 0029).
mod forge;
// Protocol 11's calls: terminals beside sessions, a project's scripts, and
// the ports their trees listen on (`terminals`, ADR 0031).
mod scripts;
mod terminals;
// Protocol 11's calls: schedules (`automation`), and the runs they make
// through the service's own checks; protocol 13's previews of them.
mod diagnostics;
mod followed;
mod inbox;
mod limits;
mod notices;
mod notifications;
mod prompts;
mod push;
mod schedule_preview;
mod scheduled;
mod schedules;
mod usage;
// Protocol 12 (track A2): the conversation, further.
mod forking;
mod history;
mod parking;
mod search;
mod settings;
mod side_questions;

#[cfg(test)]
mod connection_life_tests;
#[cfg(test)]
mod dispatch_tests;

/// The node's protocol implementation.
#[derive(Debug)]
pub(crate) struct NodeService {
    pub(crate) lsp: vessel_lsp::Manager,
    pub(crate) lsp_roots: Vec<crate::LspRoot>,
    pub(crate) lifecycle: Lifecycle,
    lookup: Lookup,
    sessions: Arc<Sessions>,
    devices: Arc<Devices>,
    /// Forwarding tasks, including transfer cleanup retained across caller
    /// cancellation; shutdown gives them its existing bounded grace.
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
    /// Pull requests (protocol 11, `forge`).
    forges: Arc<Forges>,
    /// Terminals beside sessions and scripts' runs (protocol 11,
    /// `terminals`).
    terminals: Arc<Terminals>,
    /// The ports their trees and the sessions' listen on (protocol 11).
    ports: Arc<Ports>,
    /// What `agent.status` found (protocol 11, `readiness`).
    readiness: Arc<crate::readiness::Readiness>,
    pub(crate) diagnostics: crate::diagnostics::State,
    /// The node's copy of the ACP registry (protocol 11, `catalogue`).
    catalogue: crate::catalogue::Catalogues,
    /// The last install (protocol 11, `install`).
    installs: crate::install::Installs,
    /// The scheduler (protocol 11, `automation::schedules`).
    schedules: Schedules,
    preview_slots: Arc<tokio::sync::Semaphore>,
    /// Membership writes are non-queueing; reads retain a separate budget.
    fleet_mutations: Arc<tokio::sync::Semaphore>,
    fleet_reads: Arc<tokio::sync::Semaphore>,
    /// Saved prompts (protocol 11, `automation::prompts`).
    prompts: SavedPrompts,
    /// Places for searches running at once (protocol 12, `search`).
    searches: tokio::sync::Semaphore,
    /// Places for listings of agents' histories running at once
    /// (protocol 17, `history`).
    listings: tokio::sync::Semaphore,
    /// The node's settings (protocol 12, `crate::settings`).
    settings: Arc<crate::settings::Settings>,
    limits: crate::limits::Limits,
    /// Turns followed for their starters (protocol 11,
    /// `automation::notices`).
    notices: Arc<Notices>,
    /// Creation/reset lease also held by a detached creation task.
    side_opening: Arc<tokio::sync::Mutex<()>>,
    push: Arc<crate::push::Push>,
}

/// The parts a [`NodeService`] serves.
pub(crate) struct Parts {
    pub(crate) diagnostics: crate::diagnostics::Setup,
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
    pub(crate) forges: Arc<Forges>,
    pub(crate) terminals: Arc<Terminals>,
    pub(crate) ports: Arc<Ports>,
    pub(crate) catalogue: crate::catalogue::Catalogues,
    pub(crate) schedules: Schedules,
    pub(crate) settings: Arc<crate::settings::Settings>,
    pub(crate) push: Arc<crate::push::Push>,
    pub(crate) limits: crate::limits::Limits,
}

impl NodeService {
    pub(crate) fn new(parts: Parts) -> Self {
        let Parts {
            diagnostics,
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
            forges,
            terminals,
            ports,
            catalogue,
            schedules,
            settings,
            push,
            limits,
        } = parts;
        let store = sessions.store().clone();
        let readiness = Arc::new(crate::readiness::Readiness::default());
        let diagnostics = crate::diagnostics::State::new(
            diagnostics,
            lookup.clone(),
            Arc::clone(&profiles),
            Arc::clone(&readiness),
            store.clone(),
        );
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
            forges,
            terminals,
            ports,
            readiness,
            diagnostics,
            catalogue,
            installs: crate::install::Installs::default(),
            schedules,
            preview_slots: Arc::new(tokio::sync::Semaphore::new(4)),
            fleet_mutations: Arc::new(tokio::sync::Semaphore::new(1)),
            fleet_reads: Arc::new(tokio::sync::Semaphore::new(4)),
            prompts: SavedPrompts::new(store),
            searches: tokio::sync::Semaphore::new(search::AT_ONCE),
            listings: tokio::sync::Semaphore::new(history::AT_ONCE),
            settings,
            limits,
            notices: Arc::default(),
            side_opening: Arc::default(),
            push,
        }
    }

    /// What runs where: live sessions and (protocol 11) live terminals,
    /// each of which keeps a worktree or a project from being removed.
    pub(crate) fn occupied(&self) -> Vec<(Occupant, std::path::PathBuf)> {
        let mut live = self.sessions.live_folders();
        live.extend(self.terminals.live_folders());
        live
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
    presence_id: crate::push::PresenceId,
    /// Its identity and close observer, one per connection
    /// (`connection_life`).
    life: ConnectionLife,
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
            life: ConnectionLife::within(peer.outbound.closing_token()),
            outbound: peer.outbound,
            principal,
            protocol: peer.protocol,
            waiting: ConnectionWaits::default(),
            presence_id: crate::push::PresenceId::new(),
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
            presence_id: self.presence_id,
            life: self.life.clone(),
        }
    }

    /// This connection's identity and close observer, the same for every
    /// context acting for it. A resource the connection owns keeps
    /// `owner()` and `closed()`, never the life: holding the life keeps the
    /// connection's last context from ending it.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "A10 exception until D2 Preview or W14 reads it")
    )]
    pub(crate) const fn life(&self) -> &ConnectionLife {
        &self.life
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
        Ok(self.harnesses().await)
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

    /// The authenticated caller's grant, without device administration.
    #[expect(clippy::unused_async, reason = "uniform generated async handler shape")]
    pub(crate) async fn device_self(
        &self,
        connection: &Connection,
        _: vessel_proto::GetDeviceSelf,
    ) -> Result<vessel_proto::DeviceSelf, WireError> {
        Ok(connection.principal().device_self())
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
