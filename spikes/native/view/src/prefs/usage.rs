//! Usage choices reuse field-wise fallback; sidebar visibility has one separate owner.
use crate::usage::DisplayAs;
use serde::{Deserialize, Serialize};
use vessel_proto::NodeId;
/// A host is persisted by handshake identity, never by a mutable human label.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UsagePreferences {
    /// A malformed display preference does not discard a valid host selection.
    #[serde(deserialize_with = "super::lenient")]
    pub display: DisplayAs,
    /// Explicit host pick; absent follows the active host and then the first eligible one.
    #[serde(deserialize_with = "super::lenient")]
    pub host: Option<NodeId>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_fields_fall_back_independently_and_host_survives_round_trip() {
        let host = NodeId::random();
        let saved = serde_json::json!({"display":"newer-value","host":host});
        let prefs: UsagePreferences = serde_json::from_value(saved).unwrap();
        assert_eq!(prefs.display, DisplayAs::Used);
        assert_eq!(prefs.host, Some(host));
        assert_eq!(
            serde_json::from_slice::<UsagePreferences>(&serde_json::to_vec(&prefs).unwrap())
                .unwrap(),
            prefs
        );
        let prefs: UsagePreferences =
            serde_json::from_value(serde_json::json!({"display":"remaining","host":false}))
                .unwrap();
        assert_eq!(prefs.display, DisplayAs::Remaining);
        assert_eq!(prefs.host, None);
    }
}
