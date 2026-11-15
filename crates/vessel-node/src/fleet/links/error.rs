//! Translate one hop's errors without depending on connection state.

use super::{internal, unavailable};
use vessel_client::CallError;
use vessel_proto::{ErrorCode, NodeId, PROTOCOL_VERSION, WireError};

/// A hop's failure as the caller sees it: the target's own error as it
/// is, the hop's in words that say which node.
pub(super) fn hop_error(node: NodeId, error: CallError) -> WireError {
    match error {
        CallError::Remote(error) => error,
        CallError::UnassignedProtocol { .. } => {
            WireError::new(ErrorCode::Unsupported, error.to_string())
        }
        CallError::NotInProtocol {
            method, negotiated, ..
        } => WireError::new(
            ErrorCode::Unsupported,
            format!(
                "node {node} speaks protocol {negotiated}; `{}` needs {PROTOCOL_VERSION}",
                method.as_str()
            ),
        ),
        CallError::Disconnected => unavailable(format!(
            "lost the connection to node {node}; the call may or may not have run there"
        )),
        CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => internal(&error),
    }
}
