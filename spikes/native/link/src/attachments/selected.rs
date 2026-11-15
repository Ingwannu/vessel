//! One closed error projection; all physical admission stays with G4.
use super::RequestProblem;
use crate::connections::RequestFailure;
use vessel_client::CallError;
pub(crate) fn problem(error: &RequestFailure) -> RequestProblem {
    match error {
        RequestFailure::Refused(why) => RequestProblem::Retired(*why),
        RequestFailure::Call(error) => match error {
            CallError::Remote(wire) => RequestProblem::Refused(wire.code),
            CallError::Disconnected => RequestProblem::Closed,
            CallError::UnexpectedReply(_) => RequestProblem::InvalidReply,
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
                RequestProblem::Unsupported
            }
            CallError::RequestIdsExhausted => RequestProblem::Exhausted,
        },
    }
}
