//! Stored payload diagnostics must not escape through a read-scope response.

use crate::service::wire;
use vessel_proto::WireError;
use vessel_store::StoreError;

pub(in crate::service) fn stored(error: StoreError) -> WireError {
    // serde can quote an invalid value, including configured argv or receipt
    // ownership data. Preserve its typed code but expose only the safe wrapper.
    if matches!(
        error,
        StoreError::CorruptTerminalProfiles(_)
            | StoreError::CorruptCreation(_)
            | StoreError::CorruptCreationOwner(_)
    ) {
        return WireError::new(error.code(), error.to_string());
    }
    wire(&error.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::ErrorCode;

    #[test]
    fn corrupt_profile_and_receipt_values_do_not_escape_in_wire_diagnostics() {
        const PRIVATE_VALUE: &str = "synthetic-private-command-argument";
        for kind in 0..3 {
            let source =
                serde_json::from_value::<u64>(serde_json::json!(PRIVATE_VALUE)).unwrap_err();
            assert!(source.to_string().contains(PRIVATE_VALUE));
            let error = match kind {
                0 => StoreError::CorruptTerminalProfiles(source),
                1 => StoreError::CorruptCreation(source),
                _ => StoreError::CorruptCreationOwner(source),
            };
            let diagnostic = stored(error);
            assert_eq!(diagnostic.code, ErrorCode::Internal);
            assert!(!diagnostic.message.contains(PRIVATE_VALUE));
            assert!(diagnostic.message.contains("invalid"));
        }
    }

    #[test]
    fn a_known_configuration_refusal_preserves_its_protocol_code() {
        let error = stored(StoreError::TerminalProfiles(
            vessel_proto::terminal_profiles::ProfileError::StaleRevision,
        ));
        assert_eq!(error.code, ErrorCode::Conflict);
    }
}
