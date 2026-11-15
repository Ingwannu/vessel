//! Person-only creation observations reuse the store and shared waiting limits.

use super::{Connection, NodeService, wire};
use crate::waits::Waiter;
use std::time::Duration;
use vessel_proto::creation::{ObserveCreation, Snapshot, SubscribeCreation};
use vessel_proto::{ErrorCode, WireError};
use vessel_store::creation::Identity;

impl NodeService {
    pub(crate) async fn observe_creation(
        &self,
        connection: &Connection,
        request: ObserveCreation,
    ) -> Result<Option<Snapshot>, WireError> {
        person(connection)?;
        self.creation_snapshot(request).await
    }

    pub(crate) async fn subscribe_creation(
        &self,
        connection: &Connection,
        request: SubscribeCreation,
    ) -> Result<Option<Snapshot>, WireError> {
        person(connection)?;
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let mut wait = request.wait.seconds();
        let _slot = self.wait_slot(connection, shares, &mut wait);
        // Subscribe first. A write between the SQL read and changed() remains
        // visible; notifications may coalesce, never the authoritative row.
        let mut changes = self.sessions.store().creation_changes();
        let snapshot = self.creation_snapshot(request.identity.clone()).await?;
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
        self.creation_snapshot(request.identity).await
    }

    async fn creation_snapshot(
        &self,
        request: ObserveCreation,
    ) -> Result<Option<Snapshot>, WireError> {
        self.sessions
            .store()
            .creation(&Identity::new(request.kind, request.key))
            .await
            .map(|receipt| receipt.map(|receipt| receipt.snapshot().clone()))
            .map_err(|error| wire(&error.into()))
    }
}

fn person(connection: &Connection) -> Result<(), WireError> {
    if connection.principal().agent().is_some() {
        return Err(WireError::new(
            ErrorCode::Forbidden,
            "creation receipts are a person's operation history",
        ));
    }
    Ok(())
}
