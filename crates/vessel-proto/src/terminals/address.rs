//! A PTY's typed target is independent of its renderer, viewport or node connection.
use crate::{SessionId, TerminalId};
use serde::{Deserialize, Serialize};

/// Session TUIs and standalone terminals keep their existing distinct identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TerminalAddress {
    /// A session whose mode is Terminal; this is not a standalone terminal id.
    Session(SessionId),
    /// A shell, script or service in the standalone terminal registry.
    Standalone(TerminalId),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_uuid_text_still_has_two_distinct_target_kinds() {
        let text = SessionId::random().to_string();
        // Adversarial wire fixture, never a production identifier conversion.
        let session = TerminalAddress::Session(text.parse().unwrap());
        let standalone = TerminalAddress::Standalone(text.parse().unwrap());
        assert_ne!(session, standalone);
        for target in [session, standalone] {
            let json = serde_json::to_string(&target).unwrap();
            assert_eq!(
                serde_json::from_str::<TerminalAddress>(&json).unwrap(),
                target
            );
        }
        for json in [
            format!(r#"{{"kind":"session","id":"{text}","node":"invented"}}"#),
            format!(r#"{{"kind":"other","id":"{text}"}}"#),
        ] {
            assert!(serde_json::from_str::<TerminalAddress>(&json).is_err());
        }
    }
}
