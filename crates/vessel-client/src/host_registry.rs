//! Typed host directory methods use the protocol table's pending/old-peer gate.
use crate::{CallError, NodeClient};
use vessel_proto::host_registry::{
    AddHostRegistration, HostDirectory, HostRegistration, ListHostRegistrations,
    RemoveHostRegistration, UpdateHostRegistration,
};
impl NodeClient {
    /// Read public cross-source inventory; foreign/forwarded agents remain forbidden.
    /// # Errors
    /// Unassigned/older peers refuse before sending; missing Read or nonlocal authority refuses.
    pub async fn shared_host_registrations(
        &self,
    ) -> Result<vessel_proto::host_registry::SharedHostDirectory, CallError> {
        self.call(vessel_proto::host_registry::ListSharedHostRegistrations)
            .await
    }
    /// Read caller-source registrations, including disabled/unreachable targets.
    /// # Errors
    /// An unassigned feature or older peer refuses before sending; otherwise normal RPC failures.
    pub async fn host_registrations(&self) -> Result<HostDirectory, CallError> {
        self.call(ListHostRegistrations).await
    }
    /// Add metadata referencing a destination the node already trusts.
    /// # Errors
    /// Unknown authority, source/scope refusal, storage failure or normal RPC failure.
    pub async fn add_host_registration(
        &self,
        input: AddHostRegistration,
    ) -> Result<HostRegistration, CallError> {
        self.call(input).await
    }
    /// Compare revision and source before atomically replacing a registration.
    /// # Errors
    /// Stale revision, unconfirmed identity replacement, ownership or normal RPC failure.
    pub async fn update_host_registration(
        &self,
        input: UpdateHostRegistration,
    ) -> Result<HostRegistration, CallError> {
        self.call(input).await
    }
    /// Remove metadata without revoking its destination's independent authority.
    /// # Errors
    /// Stale/unknown registration, ownership or normal RPC failure.
    pub async fn remove_host_registration(
        &self,
        input: RemoveHostRegistration,
    ) -> Result<(), CallError> {
        self.call(input).await.map(drop)
    }
}
