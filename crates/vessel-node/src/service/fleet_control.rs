//! Fleet control at the authenticated RPC edge (ADR 0044). Membership
//! changes reuse the offline CLI's implementation; Read receives only a
//! safe projection. Permits live inside blocking work after cancellation.
use super::{Connection, NodeService, wire};
use crate::fleet::ProjectionError;
use crate::fleet::{FileError, JoinError};
use crate::principal::Principal;
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;
use vessel_proto::{
    Ack, ErrorCode, ErrorDetail, FleetControlError, FleetPublished, FleetRole, FleetSyncStatus,
    GetFleetStatus, GetPublished, JoinFleet, JoinedFleet, LeaveFleet, Scope, SyncFleet, WireError,
};

fn refusal(reason: FleetControlError, code: ErrorCode) -> WireError {
    let mut error = WireError::new(code, reason.to_string());
    error.detail = Some(ErrorDetail::FleetControl { reason });
    error
}

fn direct(connection: &Connection) -> Result<(), WireError> {
    match connection.principal() {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        Principal::Agent { .. } | Principal::Delegated { .. } => {
            Err(refusal(FleetControlError::DirectOnly, ErrorCode::Forbidden))
        }
    }
}

fn local_admin(connection: &Connection) -> bool {
    match connection.principal() {
        Principal::Owner => true,
        Principal::Device { scopes, .. } => scopes.contains(&Scope::Admin),
        Principal::Agent { .. } | Principal::Delegated { .. } => false,
    }
}

fn file_refusal(error: &FileError, reason: FleetControlError) -> WireError {
    match error {
        FileError::Io { source, .. } if source.kind() == std::io::ErrorKind::WouldBlock => {
            refusal(FleetControlError::Busy, ErrorCode::Unavailable)
        }
        FileError::Io { source, .. } => {
            tracing::warn!(kind = ?source.kind(), "fleet state I/O failed");
            refusal(reason, ErrorCode::Internal)
        }
        FileError::Invalid { .. } => {
            // Serde's message can quote file text. Keep it off the wire
            // and out of logs; the local state is not a public payload.
            tracing::warn!("fleet state is invalid");
            refusal(reason, ErrorCode::Conflict)
        }
    }
}

fn join_refusal(error: &JoinError) -> WireError {
    match error {
        JoinError::Home => refusal(FleetControlError::AlreadyHome, ErrorCode::Conflict),
        JoinError::Member => refusal(FleetControlError::AlreadyMember, ErrorCode::Conflict),
        JoinError::File(error) => file_refusal(error, FleetControlError::CannotStore),
        JoinError::Credential(_) | JoinError::Task(_) => {
            refusal(FleetControlError::CannotStore, ErrorCode::Internal)
        }
        JoinError::Tls(_) | JoinError::Reach(_) => {
            refusal(FleetControlError::CannotJoin, ErrorCode::Unavailable)
        }
        JoinError::AfterPairing(_) => {
            refusal(FleetControlError::JoinUncertain, ErrorCode::Conflict)
        }
    }
}

/// A refused `fleet.published`: as `fleet.status`'s, but its own words when
/// the list is over a bound.
fn published_refusal(error: &ProjectionError) -> WireError {
    match error {
        ProjectionError::TooLarge | ProjectionError::Reply(_) => {
            refusal(FleetControlError::PublishedTooLarge, ErrorCode::Unavailable)
        }
        ProjectionError::File(_) | ProjectionError::Decode(_) => projection_refusal(error),
    }
}

fn projection_refusal(error: &ProjectionError) -> WireError {
    match error {
        ProjectionError::File(error) => file_refusal(error, FleetControlError::CannotRead),
        ProjectionError::Decode(_) => refusal(FleetControlError::CannotRead, ErrorCode::Conflict),
        ProjectionError::TooLarge | ProjectionError::Reply(_) => {
            refusal(FleetControlError::StatusTooLarge, ErrorCode::Unavailable)
        }
    }
}

impl NodeService {
    fn membership_permit(&self) -> Result<OwnedSemaphorePermit, WireError> {
        Arc::clone(&self.fleet_mutations)
            .try_acquire_owned()
            .map_err(|_busy| refusal(FleetControlError::Busy, ErrorCode::Unavailable))
    }

    pub(crate) async fn join_fleet(
        &self,
        connection: &Connection,
        request: JoinFleet,
    ) -> Result<JoinedFleet, WireError> {
        direct(connection)?;
        let permit = self.membership_permit()?;
        let offer = request.link.decode().map_err(|_invalid| {
            refusal(
                FleetControlError::InvalidInvitation,
                ErrorCode::InvalidRequest,
            )
        })?;
        if offer.node == self.lifecycle.info().id || offer.key == *self.devices.key() {
            return Err(refusal(FleetControlError::SelfHome, ErrorCode::Conflict));
        }
        let home = Arc::clone(&self.fleet);
        let joined = crate::fleet::join_reserved(
            self.fleet.files(),
            &offer,
            &self.lifecycle.info().name,
            request.grant,
            Some(permit),
            move || home.membership_changed(),
        )
        .await
        .map_err(|error| join_refusal(&error))?;
        Ok(JoinedFleet { home: joined.home })
    }

    pub(crate) async fn leave_fleet(
        &self,
        connection: &Connection,
        _: LeaveFleet,
    ) -> Result<Ack, WireError> {
        direct(connection)?;
        let permit = self.membership_permit()?;
        let home = Arc::clone(&self.fleet);
        let removed = tokio::task::spawn_blocking(move || {
            let removed = crate::fleet::leave_reserved(home.files(), Some(permit));
            home.membership_changed();
            removed
        })
        .await
        .map_err(|error| wire(&crate::error::ServiceError::Task(error)))?;
        // Membership is removed first. Even if credential cleanup fails,
        // revoke the previous home's device before returning that failure.
        // A vanished RPC still leaves the supervisor's wakeup above.
        self.links.reconcile().await;
        let _was_member =
            removed.map_err(|error| file_refusal(&error, FleetControlError::CannotStore))?;
        Ok(Ack)
    }

    pub(crate) async fn sync_fleet(
        &self,
        connection: &Connection,
        _: SyncFleet,
    ) -> Result<Ack, WireError> {
        direct(connection)?;
        let permit = self.membership_permit()?;
        let home = Arc::clone(&self.fleet);
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let role = crate::fleet::read_role(home.files())
                .map_err(|error| projection_refusal(&error))?;
            match role {
                FleetRole::Standalone | FleetRole::Unknown => {
                    return Err(refusal(
                        FleetControlError::NotConfigured,
                        ErrorCode::Conflict,
                    ));
                }
                FleetRole::Home | FleetRole::Member { .. } => {}
            }
            home.sync_now();
            Ok(Ack)
        })
        .await
        .map_err(|error| wire(&crate::error::ServiceError::Task(error)))?
    }

    pub(crate) async fn fleet_status(
        &self,
        connection: &Connection,
        _: GetFleetStatus,
    ) -> Result<FleetSyncStatus, WireError> {
        let permit = Arc::clone(&self.fleet_reads)
            .try_acquire_owned()
            .map_err(|_busy| refusal(FleetControlError::Busy, ErrorCode::Unavailable))?;
        let files = self.fleet.files().clone();
        let (mut status, mut permit) = tokio::task::spawn_blocking(move || {
            let status = crate::fleet::read_status(&files)?;
            Ok::<_, ProjectionError>((status, permit))
        })
        .await
        .map_err(|error| wire(&crate::error::ServiceError::Task(error)))?
        .map_err(|error| projection_refusal(&error))?;
        if status.role == FleetRole::Home && local_admin(connection) {
            // A database diagnostic can quote private row text or a data
            // path. This read boundary sends only its typed refusal.
            let (window, retained) =
                self.devices
                    .window(None, permit)
                    .await
                    .map_err(|_private_error| {
                        refusal(FleetControlError::CannotRead, ErrorCode::Internal)
                    })?;
            permit = retained;
            if window.more {
                return Err(projection_refusal(&ProjectionError::TooLarge));
            }
            status.members = Some(
                self.fleet
                    .bounded_members(window.devices)
                    .await
                    .map_err(|error| projection_refusal(&error))?,
            );
        }
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            crate::fleet::finish_status(status)
        })
        .await
        .map_err(|error| wire(&crate::error::ServiceError::Task(error)))?
        .map_err(|error| projection_refusal(&error))
    }

    /// `fleet.published` (protocol 16, ADR 0056): what this machine shares
    /// or last received, by name. `admin` (the guard), never forwarded;
    /// every role answers with a value, a standalone machine's included.
    pub(crate) async fn fleet_published(
        &self,
        _: &Connection,
        _: GetPublished,
    ) -> Result<FleetPublished, WireError> {
        let permit = Arc::clone(&self.fleet_reads)
            .try_acquire_owned()
            .map_err(|_busy| refusal(FleetControlError::Busy, ErrorCode::Unavailable))?;
        let files = self.fleet.files().clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            crate::fleet::read_published(&files)
        })
        .await
        .map_err(|error| wire(&crate::error::ServiceError::Task(error)))?
        .map_err(|error| published_refusal(&error))
    }
}
