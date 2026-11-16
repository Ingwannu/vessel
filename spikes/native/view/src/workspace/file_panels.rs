//! Whether a machine has the file panels' calls (`file.observe`,
//! `file.duplicate`, `file.read_bytes`, ... ; ADR 0083, assigned one
//! protocol number together, ADR 0105). The Files tab offers its watch,
//! Duplicate and Download only on such a machine, as Paseo hides Duplicate
//! from a host without `fsEntryDuplicateEnabled`; elsewhere it behaves as it
//! did before they existed.

use vessel_proto::ProtocolVersion;
use vessel_proto::workspace::panel_files::FILE_PANELS_PROTOCOL;

/// Whether a machine speaking `protocol` (`None`: not connected) can be
/// asked the file panels' calls.
#[must_use]
pub fn offered(protocol: Option<ProtocolVersion>) -> bool {
    matches!((protocol, FILE_PANELS_PROTOCOL), (Some(speaks), Some(needs)) if speaks >= needs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_machine_speaking_the_file_panels_protocol_has_them() {
        assert!(!offered(None));
        assert_eq!(
            offered(Some(vessel_proto::PROTOCOL_VERSION)),
            FILE_PANELS_PROTOCOL.is_some()
        );
        if let Some(needs) = FILE_PANELS_PROTOCOL {
            assert!(offered(Some(needs)));
            assert!(!offered(Some(ProtocolVersion::new(
                needs.get().saturating_sub(1)
            ))));
        }
    }
}
