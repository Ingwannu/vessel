//! Usage choices reuse field-wise fallback; sidebar visibility has one separate owner.
use crate::usage::{DisplayAs, PinSelection};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    NodeId,
    limits::LimitKey,
    usage_reports::{SavedUsageSource, UsageSourceId},
};
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
    /// None follows source defaults; an explicit empty list pins nothing.
    #[serde(deserialize_with = "pins")]
    pub pins: PinSelection<UsageSourceId, LimitKey>,
}
fn pins<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<PinSelection<UsageSourceId, LimitKey>, D::Error> {
    let pins: PinSelection<UsageSourceId, LimitKey> = super::lenient(decoder)?;
    Ok(pins.map_sources(|source| {
        let saved = match source.as_str() {
            "claude" | "claude-code" => Some(SavedUsageSource::Claude),
            "codex" => Some(SavedUsageSource::Codex),
            "opencode" => Some(SavedUsageSource::OpenCode),
            _ => None,
        };
        saved.and_then(|saved| saved.id().ok()).unwrap_or(source)
    }))
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
    #[test]
    fn old_saved_source_pins_migrate_without_turning_explicit_empty_into_defaults() {
        let prefs: UsagePreferences =
            serde_json::from_str(r#"{"pins":[{"source":"claude-code","window":"weekly"}]}"#)
                .unwrap();
        assert_eq!(prefs.pins.effective(&[])[0].source.as_str(), "saved-claude");
        let empty: UsagePreferences = serde_json::from_str(r#"{"pins":[]}"#).unwrap();
        assert_eq!(
            serde_json::to_value(&empty).unwrap()["pins"],
            serde_json::json!([])
        );
        let defaults: UsagePreferences = serde_json::from_str(r#"{"pins":null}"#).unwrap();
        assert_eq!(
            serde_json::to_value(defaults).unwrap()["pins"],
            serde_json::Value::Null
        );
    }
}
