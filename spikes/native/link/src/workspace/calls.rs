//! One typed call, its failure made a [`Problem`]: the node's refusal kept
//! with its code, a call the machine's protocol does not have read as an
//! older machine, anything else as a connection gone.

use vessel_app_view::workspace::problem::Problem;
use vessel_client::{CallError, NodeClient, Unary};

/// Send `request` on `client`.
pub(crate) async fn call<R: Unary>(
    client: &NodeClient,
    request: R,
) -> Result<R::Response, Problem> {
    client.call(request).await.map_err(|error| problem(&error))
}

/// `error` as a screen reads it.
pub(crate) fn problem(error: &CallError) -> Problem {
    match error {
        CallError::Remote(wire) => Problem::Node {
            code: wire.code,
            message: wire.message.clone(),
        },
        CallError::NotInProtocol { .. } => Problem::Older,
        CallError::Disconnected
        | CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted => Problem::Gone(error.to_string()),
    }
}
