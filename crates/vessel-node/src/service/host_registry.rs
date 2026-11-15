//! Source identity comes from authentication; registrations never create credential authority.
use super::{Connection, NodeService};
use crate::{
    host_registry::{HostRegistry, RegistryError, TrustedDestinations},
    principal::Principal,
};
use vessel_proto::{
    Ack, ErrorCode, FleetNode, ListNodes, WireError,
    host_registry::{
        AddHostRegistration, HostDestination, HostDirectory, HostEnabled, HostRegistration,
        HostSource, HostStatus, HostSummary, ListHostRegistrations, RemoveHostRegistration,
        UpdateHostRegistration,
    },
};
mod shared;
impl NodeService {
    pub(crate) async fn list_host_registrations(
        &self,
        connection: &Connection,
        _request: ListHostRegistrations,
    ) -> Result<HostDirectory, WireError> {
        let source = source(connection.principal())?;
        let entries = self
            .host_registry()?
            .list(source)
            .await
            .map_err(|error| wire(&error))?;
        let destinations = self.list_nodes(connection, ListNodes).await;
        let hosts = entries
            .into_iter()
            .map(|registration| summary(registration, &destinations))
            .collect();
        Ok(HostDirectory {
            node: self.lifecycle.info(),
            hosts,
        })
    }
    pub(crate) async fn add_host_registration(
        &self,
        connection: &Connection,
        request: AddHostRegistration,
    ) -> Result<HostRegistration, WireError> {
        let source = source(connection.principal())?;
        self.host_registry()?
            .add(source, request, self.trusted_hosts(connection).await?)
            .await
            .map_err(|error| wire(&error))
    }
    pub(crate) async fn update_host_registration(
        &self,
        connection: &Connection,
        request: UpdateHostRegistration,
    ) -> Result<HostRegistration, WireError> {
        let source = source(connection.principal())?;
        // Annotation/enablement edits remain possible when a destination is offline
        // or its authority has disappeared. Only rebinding needs a new trust proof.
        let trusted = if request.destination.is_some() {
            self.trusted_hosts(connection).await?
        } else {
            TrustedDestinations(Vec::new())
        };
        self.host_registry()?
            .update(source, request, trusted)
            .await
            .map_err(|error| wire(&error))
    }
    pub(crate) async fn remove_host_registration(
        &self,
        connection: &Connection,
        request: RemoveHostRegistration,
    ) -> Result<Ack, WireError> {
        let source = source(connection.principal())?;
        self.host_registry()?
            .remove(source, request)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }
    fn host_registry(&self) -> Result<&HostRegistry, WireError> {
        self.host_registry.as_ref().ok_or_else(|| {
            WireError::new(
                ErrorCode::Unavailable,
                "host registry is unavailable; saved data is retained".to_owned(),
            )
        })
    }
    async fn trusted_hosts(
        &self,
        connection: &Connection,
    ) -> Result<TrustedDestinations, WireError> {
        // Incoming paired devices are client identities, not outbound node
        // destinations. Only existing fleet node authorities are resolvable
        // on this head; unresolved paired refs refuse instead of inventing trust.
        let own = self.links.node();
        let known = self.list_nodes(connection, ListNodes).await?;
        Ok(TrustedDestinations(
            known
                .into_iter()
                .filter(|node| node.node != own)
                .map(|node| HostDestination::Fleet { node: node.node })
                .collect(),
        ))
    }
}
fn source(principal: &Principal) -> Result<HostSource, WireError> {
    match principal {
        Principal::Owner => Ok(HostSource::Owner),
        Principal::Device { id, .. } => Ok(HostSource::Device { device: *id }),
        Principal::Agent { .. } | Principal::Delegated { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "host registrations require a direct owner or paired-device connection".to_owned(),
        )),
    }
}
fn summary(
    registration: HostRegistration,
    nodes: &Result<Vec<FleetNode>, WireError>,
) -> HostSummary {
    let known = nodes.as_ref().ok().and_then(|nodes| {
        nodes
            .iter()
            .find(|node| node.node == registration.destination.node())
    });
    let (mut status, mut error) = if registration.enabled == HostEnabled::Disabled {
        (HostStatus::Disabled, None)
    } else {
        match nodes {
            Err(error) => (HostStatus::Error, Some(error.clone())),
            Ok(_nodes) => match known {
                Some(node) => (
                    if node.reachable {
                        HostStatus::Online
                    } else {
                        HostStatus::Offline
                    },
                    None,
                ),
                None => (
                    HostStatus::Error,
                    Some(WireError::new(
                        ErrorCode::NotFound,
                        "destination trust authority is no longer available".to_owned(),
                    )),
                ),
            },
        }
    };
    let destination_name = match known.map(|node| node.name.parse()) {
        None => None,
        Some(Ok(label)) => Some(label),
        Some(Err(_invalid)) => {
            if error.is_none() {
                status = HostStatus::Error;
                error = Some(WireError::new(
                    ErrorCode::InvalidRequest,
                    "destination supplied an invalid display label".to_owned(),
                ));
            }
            None
        }
    };
    HostSummary {
        registration,
        status,
        destination_name,
        error,
    }
}
fn wire(error: &RegistryError) -> WireError {
    let code = match error {
        RegistryError::NotFound => ErrorCode::NotFound,
        RegistryError::Forbidden => ErrorCode::Forbidden,
        RegistryError::Untrusted => ErrorCode::InvalidRequest,
        RegistryError::Stale
        | RegistryError::IdentityChanged
        | RegistryError::RevisionExhausted => ErrorCode::Conflict,
        RegistryError::Busy
        | RegistryError::Timeout(_)
        | RegistryError::Worker(_)
        | RegistryError::File(_)
        | RegistryError::SharedInventory(_)
        | RegistryError::Full => ErrorCode::Unavailable,
    };
    WireError::new(code, error.to_string())
}
