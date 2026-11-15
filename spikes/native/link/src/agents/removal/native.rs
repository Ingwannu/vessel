//! No removal is replayed, including after a timeout or malformed response.
use super::Removal;
use crate::agents::Answer;
use std::time::Duration;
use vessel_app_view::agent_status::problem::Problem;
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ErrorCode, NodeId, ProtocolVersion, agents::RemoveAgent};

/// A unary administrative mutation must not occupy a hub permit indefinitely.
const REMOVE_TIMEOUT: Duration = Duration::from_secs(30);
/// ADR 0032 introduced agent administration in protocol 11.
const REMOVE_PROTOCOL: ProtocolVersion = ProtocolVersion::new(11);

/// Complete typed source of a removal failure; presentation stays in view.
#[derive(Debug, thiserror::Error)]
pub enum RemovalError {
    /// No mutation was sent to this different connection.
    #[error("the confirmed node {expected} is now connected as {actual}")]
    NodeChanged {
        /// Identity retained by confirmation.
        expected: NodeId,
        /// Identity of the actual client, checked immediately before dispatch.
        actual: NodeId,
    },
    /// No mutation was sent to an older peer.
    #[error("this connection does not support agent administration")]
    OlderNode,
    /// The result is unknown, even though the client stops waiting.
    #[error("the node did not acknowledge removal before its deadline")]
    Timeout,
    /// Keep the wire code and unmodified diagnostic for the boundary.
    #[error(transparent)]
    Call(#[from] CallError),
}

impl RemovalError {
    /// Do not offer a repeat based on a possibly stale definition snapshot.
    #[must_use]
    pub fn is_uncertain(&self) -> bool {
        self.problem() == Problem::RemovalUnknown
    }

    /// Codes decide instructions; machine prose never enters translated text.
    #[must_use]
    pub const fn problem(&self) -> Problem {
        match self {
            Self::NodeChanged { .. } => Problem::NodeChanged,
            Self::OlderNode
            | Self::Call(CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. }) => {
                Problem::OlderNode
            }
            Self::Timeout | Self::Call(CallError::Disconnected | CallError::UnexpectedReply(_)) => {
                Problem::RemovalUnknown
            }
            Self::Call(CallError::RequestIdsExhausted) => Problem::Call,
            Self::Call(CallError::Remote(error)) => match error.code {
                ErrorCode::Unauthorized | ErrorCode::Forbidden => Problem::Permission,
                ErrorCode::Conflict => Problem::ProviderInUse,
                ErrorCode::ProtocolMismatch | ErrorCode::Unsupported => Problem::OlderNode,
                ErrorCode::Internal | ErrorCode::Unavailable => Problem::RemovalUnknown,
                ErrorCode::InvalidRequest
                | ErrorCode::NotFound
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed => Problem::Call,
            },
        }
    }
}

pub(in crate::agents) async fn remove(
    client: &NodeClient,
    target: Removal,
) -> Result<Answer, RemovalError> {
    let actual = client.node().id;
    if actual != target.node {
        return Err(RemovalError::NodeChanged {
            expected: target.node,
            actual,
        });
    }
    if client.protocol() < REMOVE_PROTOCOL {
        return Err(RemovalError::OlderNode);
    }
    let _ack = tokio::time::timeout(
        REMOVE_TIMEOUT,
        client.call(RemoveAgent {
            name: target.name.clone(),
        }),
    )
    .await
    .map_err(|_elapsed| RemovalError::Timeout)??;
    Ok(Answer::Removed {
        node: actual,
        name: target.name,
    })
}

#[cfg(test)]
mod tests;
