//! Admission precedes request ids and the writer queue. Keeping this
//! rule separate prevents a pending field becoming a numeric version.

use crate::CallError;
use vessel_proto::{Method, ProtocolVersion};

/// Refuse a call before it can consume an id or reach an older peer.
///
/// # Errors
/// A pending protocol assignment or a newer required version.
pub(super) fn check(
    method: Method,
    required: Option<ProtocolVersion>,
    negotiated: ProtocolVersion,
) -> Result<(), CallError> {
    let needs = required.ok_or(CallError::UnassignedProtocol { method })?;
    if needs > negotiated {
        return Err(CallError::NotInProtocol {
            method,
            needs,
            negotiated,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unassigned_and_newer_requirements_are_refused_without_a_numeric_stand_in() {
        let method = Method::NodeInfo;
        let old = vessel_proto::MIN_PROTOCOL_VERSION;
        let pending = check(method, None, old).unwrap_err();
        assert_eq!(pending, CallError::UnassignedProtocol { method });
        assert_eq!(pending.code(), Some(vessel_proto::ErrorCode::Unsupported));
        let required = vessel_proto::PROTOCOL_VERSION;
        assert_eq!(
            check(method, Some(required), old),
            Err(CallError::NotInProtocol {
                method,
                needs: required,
                negotiated: old
            })
        );
        assert_eq!(check(method, Some(required), required), Ok(()));
        assert_eq!(check(method, Some(old), required), Ok(()));
    }
}
