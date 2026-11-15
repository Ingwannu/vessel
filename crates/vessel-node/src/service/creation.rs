//! Principal-bound creation observations reuse the store and shared waiting limits.

#[cfg(test)]
pub(crate) mod driver;
#[cfg(test)]
pub(crate) mod empty;
mod failure;
#[cfg(test)]
mod intake;

pub(super) use failure::stored as stored_creation_error;

use super::{Connection, NodeService};
use crate::{principal::Principal, waits::Waiter};
use std::time::Duration;
use vessel_proto::creation::{Kind, ObserveCreation, Snapshot, SubscribeCreation};
use vessel_proto::{ErrorCode, Method, WireError};
use vessel_store::CreationIdentity as Identity;

impl NodeService {
    pub(crate) async fn observe_creation(
        &self,
        connection: &Connection,
        request: ObserveCreation,
    ) -> Result<Option<Snapshot>, WireError> {
        let identity = identity(connection.principal(), request)?;
        self.creation_snapshot(&identity).await
    }

    pub(crate) async fn subscribe_creation(
        &self,
        connection: &Connection,
        request: SubscribeCreation,
    ) -> Result<Option<Snapshot>, WireError> {
        let identity = identity(connection.principal(), request.identity)?;
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let mut wait = request.wait.seconds();
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
        let _slot = self.wait_slot(connection, shares, &mut wait);
        // Subscribe first. A write between the SQL read and changed() remains
        // visible; notifications may coalesce, never the authoritative row.
        let mut changes = self.sessions.store().creation_changes();
        let snapshot = self.creation_snapshot(&identity).await?;
        let Some(current) = &snapshot else {
            return Ok(None);
        };
        if let Some(after) = request.after {
            if after > current.revision {
                return Err(WireError::new(
                    ErrorCode::Conflict,
                    "creation cursor is ahead of the stored receipt",
                ));
            }
            if after < current.revision {
                return Ok(snapshot);
            }
        } else {
            return Ok(snapshot);
        }
        let Some(seconds) = wait else {
            return Ok(snapshot);
        };
        let _wake =
            tokio::time::timeout(Duration::from_secs(u64::from(seconds)), changes.changed()).await;
        self.creation_snapshot(&identity).await
    }

    async fn creation_snapshot(&self, identity: &Identity) -> Result<Option<Snapshot>, WireError> {
        self.sessions
            .store()
            .creation(identity)
            .await
            .map(|receipt| receipt.map(|receipt| receipt.snapshot().clone()))
            .map_err(stored_creation_error)
    }
}

fn identity(principal: &Principal, request: ObserveCreation) -> Result<Identity, WireError> {
    principal.permit(Method::ObserveCreation)?;
    match principal.agent() {
        Some((node, parent)) => match request.kind {
            Kind::Agent => Ok(Identity::for_agent(node, parent, request.key)),
            Kind::Workspace | Kind::Terminal => Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent may observe only its own agent creation operations",
            )),
        },
        None => Ok(Identity::new(request.kind, request.key)),
    }
}

#[cfg(test)]
mod tests;
