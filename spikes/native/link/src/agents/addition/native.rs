//! A stopped client wait never implies that the node's file write was undone.
use super::Addition;
use crate::agents::Answer;
use std::time::Duration;
use vessel_app_view::agent_status::addition::{Failure, matches_ack};
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ErrorCode, NodeId};

/// One administrative write cannot occupy a shared worker indefinitely.
const ADD_TIMEOUT: Duration = Duration::from_secs(30);

/// Typed sources are retained for diagnostics, never used as translated prose.
#[derive(Debug, thiserror::Error)]
pub enum AdditionError {
    /// Refused before a frame was sent through a different physical connection.
    #[error("the addition node {expected} is now connected as {actual}")]
    NodeChanged {
        /// Original inventory node.
        expected: NodeId,
        /// Actual client checked immediately before dispatch.
        actual: NodeId,
    },
    /// The node may already have written the definition; do not replay.
    #[error("the provider addition was not acknowledged before its deadline")]
    Timeout,
    /// A response was received but did not identify this requested definition.
    #[error("the provider addition acknowledgement did not match its request")]
    Acknowledgement,
    /// Preserve the actual client/wire refusal.
    #[error(transparent)]
    Call(#[from] CallError),
}
impl AdditionError {
    /// Only uncertain writes prohibit retry from retained definitions.
    #[must_use]
    pub fn is_uncertain(&self) -> bool {
        self.problem() == Failure::Unknown
    }
    /// Server codes and pre-dispatch refusals determine the whole instruction.
    #[must_use]
    pub const fn problem(&self) -> Failure {
        match self {
            Self::NodeChanged { .. } => Failure::Changed,
            Self::Timeout
            | Self::Acknowledgement
            | Self::Call(CallError::Disconnected | CallError::UnexpectedReply(_)) => {
                Failure::Unknown
            }
            Self::Call(CallError::NotInProtocol { .. }) => Failure::Unsupported,
            Self::Call(CallError::UnassignedProtocol { .. }) => Failure::BuildUnavailable,
            Self::Call(CallError::RequestIdsExhausted) => Failure::Exhausted,
            Self::Call(CallError::Remote(error)) => match error.code {
                ErrorCode::Forbidden | ErrorCode::Unauthorized => Failure::Permission,
                ErrorCode::Conflict => Failure::Conflict,
                ErrorCode::Internal | ErrorCode::Unavailable => Failure::Unknown,
                ErrorCode::ProtocolMismatch | ErrorCode::Unsupported => Failure::Unsupported,
                ErrorCode::InvalidRequest
                | ErrorCode::NotFound
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed => Failure::Rejected,
            },
        }
    }
}
pub(in crate::agents) async fn add(
    client: &NodeClient,
    target: Addition,
) -> Result<Answer, AdditionError> {
    let actual = client.node().id;
    if actual != target.node {
        return Err(AdditionError::NodeChanged {
            expected: target.node,
            actual,
        });
    }
    // Typed client admission checks the numbered method before sending. There
    // is no app version table and no retry after an uncertain dispatch.
    let info = tokio::time::timeout(ADD_TIMEOUT, client.call(target.call.clone()))
        .await
        .map_err(|_elapsed| AdditionError::Timeout)??;
    if !matches_ack(&target.call, &info) {
        return Err(AdditionError::Acknowledgement);
    }
    Ok(Answer::Added { node: actual, info })
}

#[cfg(test)]
mod tests;
