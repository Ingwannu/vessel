//! A dispatched registry read is bounded independently of the node HTTP cache.
use super::Fetch;
use crate::agents::Answer;
use std::time::Duration;
use vessel_app_view::agent_status::catalogue::Problem;
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ErrorCode, NodeId, agents::GetCatalogue};

// Node HTTP total time is30s; its fetch mutex/cache wait is not the same
// budget. Stop this client's wait without claiming that work was cancelled.
const CATALOGUE_TIMEOUT: Duration = Duration::from_secs(45);

/// Retain the original typed source; display only localized instructions.
#[derive(Debug, thiserror::Error)]
pub enum CatalogueError {
    /// No frame was sent through this different connection.
    #[error("the catalog node {expected} is now connected as {actual}")]
    NodeChanged {
        /// Identity of the inventory that opened the catalog.
        expected: NodeId,
        /// Identity checked immediately before the typed client call.
        actual: NodeId,
    },
    /// Ending the local wait does not prove the node stopped fetching.
    #[error("the provider registry read exceeded its deadline")]
    Timeout,
    /// Keep the client/transport/node code and unmodified diagnostic.
    #[error(transparent)]
    Call(#[from] CallError),
}
impl CatalogueError {
    /// Only a definite absent method is Unsupported; pending contracts stay errors.
    #[must_use]
    pub const fn problem(&self) -> Problem {
        match self {
            Self::NodeChanged { .. } => Problem::NodeChanged,
            Self::Timeout => Problem::Timeout,
            Self::Call(CallError::NotInProtocol { .. }) => Problem::Unsupported,
            Self::Call(CallError::Disconnected) => Problem::Offline,
            Self::Call(
                CallError::UnassignedProtocol { .. }
                | CallError::RequestIdsExhausted
                | CallError::UnexpectedReply(_),
            ) => Problem::Read,
            Self::Call(CallError::Remote(error)) => match error.code {
                ErrorCode::Forbidden | ErrorCode::Unauthorized => Problem::Permission,
                ErrorCode::Unsupported | ErrorCode::ProtocolMismatch => Problem::Unsupported,
                ErrorCode::Internal
                | ErrorCode::Unavailable
                | ErrorCode::InvalidRequest
                | ErrorCode::NotFound
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Conflict => Problem::Read,
            },
        }
    }
}
pub(in crate::agents) async fn read(
    client: &NodeClient,
    expected: NodeId,
    fetch: Fetch,
) -> Result<Answer, CatalogueError> {
    let actual = client.node().id;
    if actual != expected {
        return Err(CatalogueError::NodeChanged { expected, actual });
    }
    let catalogue = tokio::time::timeout(
        CATALOGUE_TIMEOUT,
        client.call(GetCatalogue {
            refresh: matches!(fetch, Fetch::Refresh),
        }),
    )
    .await
    .map_err(|_elapsed| CatalogueError::Timeout)??;
    Ok(Answer::Catalogue {
        node: actual,
        catalogue,
    })
}

#[cfg(test)]
mod tests;
