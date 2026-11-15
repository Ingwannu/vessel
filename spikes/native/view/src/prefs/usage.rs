//! Usage choices reuse field-wise fallback; sidebar visibility has one separate owner.
use crate::usage::{DisplayAs, Pin, PinSelection};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    NodeId,
    limits::LimitKey,
    usage_reports::{SavedUsageSource, UsageSourceId},
};
/// A host is persisted by handshake identity, never by a mutable human label.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RawUsagePreferences")]
pub struct UsagePreferences {
    /// A malformed display preference does not discard a valid host selection.
    pub display: DisplayAs,
    /// Explicit host pick; absent follows the active host and then the first eligible one.
    pub host: Option<NodeId>,
    /// None follows source defaults; an explicit empty list pins nothing.
    pub pins: PinSelection<UsageSourceId, LimitKey>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawUsagePreferences {
    #[serde(alias = "displayAs", deserialize_with = "super::lenient")]
    display: DisplayAs,
    #[serde(alias = "serverId", deserialize_with = "super::lenient")]
    host: Option<NodeId>,
    pins: StoredPins,
    #[serde(deserialize_with = "super::lenient")]
    pinned: Vec<Pin<UsageSourceId, LimitKey>>,
}
#[derive(Debug, Default)]
enum StoredPins {
    #[default]
    Missing,
    Present(PinSelection<UsageSourceId, LimitKey>),
}
impl<'de> Deserialize<'de> for StoredPins {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        super::lenient(decoder).map(Self::Present)
    }
}
impl From<RawUsagePreferences> for UsagePreferences {
    fn from(raw: RawUsagePreferences) -> Self {
        let pins = match raw.pins {
            StoredPins::Present(pins) => pins,
            StoredPins::Missing if raw.pinned.is_empty() => PinSelection::default(),
            StoredPins::Missing => PinSelection::explicit(raw.pinned),
        };
        let pins = pins.map_sources(|source| {
            let saved = match source.as_str() {
                "claude" | "claude-code" => Some(SavedUsageSource::Claude),
                "codex" => Some(SavedUsageSource::Codex),
                "opencode" => Some(SavedUsageSource::OpenCode),
                _ => None,
            };
            saved.and_then(|saved| saved.id().ok()).unwrap_or(source)
        });
        Self {
            display: raw.display,
            host: raw.host,
            pins,
        }
    }
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
    fn legacy_pinned_is_read_only_when_the_new_selection_is_absent() {
        let legacy = serde_json::json!([{"sourceId":"claude-code","windowId":"weekly"}]);
        let host = NodeId::random();
        let old: UsagePreferences = serde_json::from_value(
            serde_json::json!({"displayAs":"remaining","serverId":host,"pinned":legacy}),
        )
        .unwrap();
        assert_eq!(old.display, DisplayAs::Remaining);
        assert_eq!(old.host, Some(host));
        assert_eq!(old.pins.effective(&[])[0].source.as_str(), "saved-claude");
        let wire = serde_json::to_value(old).unwrap();
        assert!(wire.get("pinned").is_none());
        assert!(wire.get("displayAs").is_none());
        for pins in [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!(false),
        ] {
            let new: UsagePreferences = serde_json::from_value(
                serde_json::json!({"pinned":legacy,"pins":pins,"host":host}),
            )
            .unwrap();
            assert!(new.pins.effective(&[]).is_empty());
            assert_eq!(new.host, Some(host));
            let actual = serde_json::to_value(new).unwrap()["pins"].clone();
            assert_eq!(
                actual,
                if pins == serde_json::json!([]) {
                    serde_json::json!([])
                } else {
                    serde_json::Value::Null
                }
            );
        }
        let old_empty: UsagePreferences =
            serde_json::from_value(serde_json::json!({"pinned":[]})).unwrap();
        assert_eq!(
            serde_json::to_value(old_empty).unwrap()["pins"],
            serde_json::Value::Null
        );
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
