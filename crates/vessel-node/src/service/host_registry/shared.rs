//! Direct local Read authority exposes only the actor's public inventory projection.
use super::{Connection, NodeService, Principal, wire};
use vessel_proto::host_registry::{ListSharedHostRegistrations, SharedHostDirectory};
use vessel_proto::{ErrorCode, ListNodes, NodeId, WireError, rpc::Method};

impl NodeService {
    pub(crate) async fn list_shared_host_registrations(
        &self,
        connection: &Connection,
        _request: ListSharedHostRegistrations,
    ) -> Result<SharedHostDirectory, WireError> {
        admit(connection.principal(), self.links.node())?;
        let mut hosts = self
            .host_registry()?
            .shared()
            .await
            .map_err(|error| wire(&error))?;
        let nodes = self.list_nodes(connection, ListNodes).await;
        hosts.observe_names(|id| {
            nodes.as_ref().ok().and_then(|nodes| {
                nodes
                    .iter()
                    .find(|node| node.node == id)
                    .and_then(|node| node.name.parse().ok())
            })
        });
        Ok(SharedHostDirectory {
            node: self.links.node(),
            hosts,
        })
    }
}

fn admit(principal: &Principal, local: NodeId) -> Result<(), WireError> {
    principal.permit(Method::ListSharedHostRegistrations)?;
    match principal {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        Principal::Agent { node, .. } if *node == local => Ok(()),
        // A Read scope does not turn a foreign agent or forwarded device into a local reader.
        Principal::Agent { .. } | Principal::Delegated { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "public host inventory requires a direct local authenticated reader".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests;
