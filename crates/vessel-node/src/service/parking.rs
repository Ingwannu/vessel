//! Protocol 12's `session.park` and `session.wake` (track A2, ADR 0038).
//! Routing only: when a session may be parked and how it is woken are the
//! session runtime's (`vessel_session`'s `parking`).

use vessel_proto::parking::{ParkSession, WakeSession};
use vessel_proto::{SessionInfo, WireError};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

impl NodeService {
    /// `session.park`: stop an idle session's agent now.
    pub(crate) async fn park_session(
        &self,
        _: &Connection,
        request: ParkSession,
    ) -> Result<SessionInfo, WireError> {
        let handle = self
            .sessions
            .acting_on(request.session)
            .await
            .map_err(|error| wire(&error))?;
        handle
            .park()
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&ServiceError::Session(error)))
    }

    /// `session.wake`: start a parked session's agent again, answered once
    /// it is ready.
    pub(crate) async fn wake_session(
        &self,
        _: &Connection,
        request: WakeSession,
    ) -> Result<SessionInfo, WireError> {
        let handle = self
            .sessions
            .acting_on(request.session)
            .await
            .map_err(|error| wire(&error))?;
        handle
            .wake()
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&ServiceError::Session(error)))
    }
}
