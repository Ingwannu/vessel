//! Classify admitted mutations, including a hop's encoded connection loss.
use crate::CallError;
use vessel_proto::ErrorCode;

/// Whether an admitted RPC failure leaves its mutation outcome unknown.
/// Neither case grants permission to automatically replay the mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationError {
    /// A definitive local refusal or a typed node refusal; no rollback promised.
    Refused,
    /// No trustworthy acknowledgement of whether the mutation took effect.
    Uncertain,
}
impl MutationError {
    /// Classify the error after a mutating call was admitted.
    /// A forwarding node can encode a lost downstream reply as Unavailable or
    /// Internal; treating every Remote error as a definite refusal would hide
    /// possible worktrees, sessions or accepted file bytes on another node.
    #[must_use]
    pub fn of(error: &CallError) -> Self {
        match error {
            CallError::Disconnected | CallError::UnexpectedReply(_) => Self::Uncertain,
            CallError::RequestIdsExhausted | CallError::NotInProtocol { .. } => Self::Refused,
            CallError::Remote(error) => match error.code {
                ErrorCode::Unavailable | ErrorCode::Internal => Self::Uncertain,
                ErrorCode::InvalidRequest
                | ErrorCode::Unauthorized
                | ErrorCode::Forbidden
                | ErrorCode::ProtocolMismatch
                | ErrorCode::NotFound
                | ErrorCode::Conflict
                | ErrorCode::Unsupported
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed => Self::Refused,
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{Method, ProtocolVersion, WireError};
    #[test]
    fn encoded_hop_loss_is_uncertain_but_local_unsent_and_guard_refusals_are_definite() {
        for code in [ErrorCode::Unavailable, ErrorCode::Internal] {
            assert_eq!(
                MutationError::of(&CallError::Remote(WireError::new(code, "hop lost"))),
                MutationError::Uncertain
            );
        }
        assert_eq!(
            MutationError::of(&CallError::Disconnected),
            MutationError::Uncertain
        );
        for code in [
            ErrorCode::Forbidden,
            ErrorCode::Conflict,
            ErrorCode::Unsupported,
            ErrorCode::InvalidRequest,
        ] {
            assert_eq!(
                MutationError::of(&CallError::Remote(WireError::new(code, "refused"))),
                MutationError::Refused
            );
        }
        assert_eq!(
            MutationError::of(&CallError::NotInProtocol {
                method: Method::BeginUpload,
                needs: ProtocolVersion::new(10),
                negotiated: ProtocolVersion::new(9)
            }),
            MutationError::Refused
        );
        assert_eq!(
            MutationError::of(&CallError::RequestIdsExhausted),
            MutationError::Refused
        );
    }
}
