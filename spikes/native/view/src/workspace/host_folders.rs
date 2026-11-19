//! Add project's folder suggestions and projects' own icons (ADR 0112):
//! whether a machine has `folder.suggest` and `project.icon`, and Paseo's
//! numbers for asking (`add-project-flow.tsx`: a 250 ms pause after typing,
//! 30 folders). On an older machine the path field takes a typed path as
//! before and projects keep their initial tile.

use std::time::Duration;

use vessel_proto::ProtocolVersion;
use vessel_proto::workspace::host_folders::HOST_FOLDERS_PROTOCOL;

/// Paseo's pause between the last key and the ask.
pub const PAUSE: Duration = Duration::from_millis(250);

/// Paseo's count of folders asked for.
pub const FOLDERS: u16 = 30;

/// Whether a machine speaking `protocol` (`None`: not connected) answers
/// `folder.suggest` and `project.icon`.
#[must_use]
pub fn offered(protocol: Option<ProtocolVersion>) -> bool {
    matches!((protocol, HOST_FOLDERS_PROTOCOL), (Some(speaks), Some(needs)) if speaks >= needs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_machine_speaking_the_protocol_is_asked() {
        assert!(!offered(None));
        if let Some(needs) = HOST_FOLDERS_PROTOCOL {
            assert!(offered(Some(needs)));
            assert!(!offered(Some(ProtocolVersion::new(
                needs.get().saturating_sub(1)
            ))));
        }
    }
}
