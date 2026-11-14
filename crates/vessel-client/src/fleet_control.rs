//! Typed fleet controls (protocol 15, ADR 0044); no local file access.
use crate::NodeClient;
use vessel_proto::{
    FleetSyncStatus, GetFleetStatus, JoinFleet, JoinedFleet, LeaveFleet, PairingLink, Scope,
    SyncFleet,
};
use vessel_rpc::CallError;

impl NodeClient {
    /// Redeem a single-use invitation on this node, authenticated as Admin.
    /// Never retry automatically: a disconnected call may have persisted.
    ///
    /// # Errors
    /// `NotInProtocol` before 15; `forbidden` without Admin; typed refusal
    /// for an existing membership/home, unusable invitation or commit failure.
    pub async fn join_fleet(
        &self,
        link: PairingLink,
        grant: Vec<Scope>,
    ) -> Result<JoinedFleet, CallError> {
        self.call(JoinFleet { link, grant }).await
    }

    /// Remove this node's membership and credential; materialized files stay.
    ///
    /// # Errors
    /// `NotInProtocol` before 15; `forbidden` without Admin; an I/O refusal.
    pub async fn leave_fleet(&self) -> Result<(), CallError> {
        self.call(LeaveFleet {}).await.map(|_| ())
    }

    /// Request sync now. Completion means accepted, not already applied.
    /// Requests coalesce behind the existing publisher/pull loop.
    ///
    /// # Errors
    /// `NotInProtocol` before 15; `forbidden` without Admin; `conflict` if
    /// no fleet is configured. A disconnected request may have been accepted.
    pub async fn sync_fleet(&self) -> Result<(), CallError> {
        self.call(SyncFleet {}).await.map(|_| ())
    }

    /// Read local role and apply history; Admin on a home additionally sees
    /// the existing per-member reports. It never resolves secret values.
    ///
    /// # Errors
    /// `NotInProtocol` before 15; `forbidden` without Read; unreadable state.
    pub async fn fleet_status(&self) -> Result<FleetSyncStatus, CallError> {
        self.call(GetFleetStatus {}).await
    }

    /// `fleet.published` (protocol 16): what the machine shares through its
    /// fleet, or last received, by name; a standalone machine answers that
    /// it shares nothing.
    ///
    /// # Errors
    /// `NotInProtocol` before 16; `forbidden` without admin; a typed
    /// refusal when the list is over its bound or cannot be read.
    pub async fn fleet_published(&self) -> Result<vessel_proto::FleetPublished, CallError> {
        self.call(vessel_proto::GetPublished {}).await
    }
}
