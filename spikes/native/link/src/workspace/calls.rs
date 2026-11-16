//! One typed call, its failure made a [`Problem`]: the node's refusal kept
//! with its code, a call the machine's protocol does not have read as an
//! older machine, anything else as a connection gone.

use vessel_app_view::workspace::problem::Problem;
use vessel_client::{CallError, NodeClient, Unary, WorkspaceFileCallError};

/// Send `request` on `client`.
pub(crate) async fn call<R: Unary>(
    client: &NodeClient,
    request: R,
) -> Result<R::Response, Problem> {
    client.call(request).await.map_err(|error| problem(&error))
}

/// A file-panel call's failure (ADR 0083's SDK) as a screen reads it: its
/// deadline ended a wait the node never answered, like a connection gone.
pub(crate) fn file_problem(error: &WorkspaceFileCallError) -> Problem {
    match error {
        WorkspaceFileCallError::Call(error) => problem(error),
        WorkspaceFileCallError::Deadline => Problem::Gone(error.to_string()),
    }
}

/// `error` as a screen reads it.
pub(crate) fn problem(error: &CallError) -> Problem {
    match error {
        CallError::Remote(wire) => Problem::Node {
            code: wire.code,
            message: wire.message.clone(),
        },
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => Problem::Older,
        CallError::Disconnected
        | CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted => Problem::Gone(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unassigned_call_is_unsupported_without_a_transport_failure() {
        assert!(matches!(
            problem(&CallError::UnassignedProtocol {
                method: vessel_proto::Method::NodeInfo,
            }),
            Problem::Older
        ));
    }
}
