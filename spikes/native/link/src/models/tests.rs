//! Exhaustive public failure meanings without throwing away typed sources.
use super::*;
use vessel_client::CallError;
use vessel_proto::{ErrorCode, Method, ProtocolVersion, WireError};
#[test]
fn every_wire_error_maps_to_plain_words_while_retaining_the_source() {
    for (code, problem) in [
        (ErrorCode::Forbidden, Problem::Denied),
        (ErrorCode::Unauthorized, Problem::Denied),
        (ErrorCode::Unsupported, Problem::Unsupported),
        (ErrorCode::ProtocolMismatch, Problem::Unsupported),
        (ErrorCode::InvalidRequest, Problem::Failed),
        (ErrorCode::NotFound, Problem::Failed),
        (ErrorCode::Conflict, Problem::Failed),
        (ErrorCode::HarnessNotInstalled, Problem::Failed),
        (ErrorCode::SpawnFailed, Problem::Failed),
        (ErrorCode::Unavailable, Problem::Failed),
        (ErrorCode::Internal, Problem::Failed),
    ] {
        let wire = WireError::new(code, "fixture diagnostic");
        let error = ModelsError::Call(CallError::Remote(wire.clone()));
        assert_eq!(error.problem(), problem);
        let ModelsError::Call(CallError::Remote(retained)) = error else {
            panic!("source lost")
        };
        assert_eq!(retained, wire);
    }
}
#[test]
fn transport_page_and_protocol_failures_are_not_successful_empty_lists() {
    for (error, problem) in [
        (ModelsError::Offline, Problem::Offline),
        (ModelsError::Busy, Problem::Busy),
        (ModelsError::Timeout, Problem::Timeout),
        (ModelsError::NoTransport, Problem::NoTransport),
        (ModelsError::Call(CallError::Disconnected), Problem::Offline),
        (
            ModelsError::Call(CallError::UnassignedProtocol {
                method: Method::ListModels,
            }),
            Problem::Unsupported,
        ),
        (
            ModelsError::Call(CallError::RequestIdsExhausted),
            Problem::Failed,
        ),
        (
            ModelsError::Call(CallError::UnexpectedReply(vessel_proto::UnexpectedReply {
                expected: Method::ListModels,
                got: Method::NodeInfo,
            })),
            Problem::Failed,
        ),
        (
            ModelsError::Call(CallError::NotInProtocol {
                method: Method::ListModels,
                needs: Method::ListModels.since(),
                negotiated: ProtocolVersion::new(
                    Method::ListModels.since().get().checked_sub(1).unwrap(),
                ),
            }),
            Problem::Unsupported,
        ),
    ] {
        assert_eq!(error.problem(), problem);
    }
}
