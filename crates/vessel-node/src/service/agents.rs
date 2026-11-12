//! Protocol 11's agents (ADR 0032): agents beyond the four, as the node's
//! service routes them. Routing only, as the rest of the service.

use vessel_proto::{SessionId, WireError};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

impl NodeService {
    /// Refuse a call that would act on, or subscribe to, an added agent's
    /// session for a client of protocol 2 to 10, before it runs: the
    /// client could not decode that session, so it must not change it
    /// unseen (its replies are withheld too, `Reply::withheld`). A session
    /// this node does not have passes, for the call itself to answer.
    pub(crate) async fn refuse_older(
        &self,
        connection: &Connection,
        session: SessionId,
    ) -> Result<(), WireError> {
        let protocol = connection.protocol();
        if protocol >= vessel_proto::agents::READS_ADDED_AGENTS {
            return Ok(());
        }
        match self.sessions.get(session).await {
            Ok(info) if !vessel_proto::agents::readable(&info, protocol) => {
                Err(vessel_proto::agents::unreadable())
            }
            Ok(_) | Err(ServiceError::NotFound(_)) => Ok(()),
            Err(error) => Err(wire(&error)),
        }
    }
}
