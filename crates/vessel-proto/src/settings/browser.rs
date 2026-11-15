//! The setting is a preference, never runtime/credential authority (ADR 0078).
use super::NodeSettings;
use crate::ProtocolVersion;
use crate::browser::BrowserTools;
use serde::{Deserialize, Deserializer};

/// Serde invokes this only for a present field. Deserialize the value itself,
/// so null/boolean/object cannot collapse a named mutation into Option::None.
pub(super) fn change<'de, D: Deserializer<'de>>(
    source: D,
) -> Result<Option<BrowserTools>, D::Error> {
    BrowserTools::deserialize(source).map(Some)
}
/// Compose the existing recovery mask rather than replace its projection.
pub(super) fn project(
    original: &NodeSettings,
    projected: &mut Option<NodeSettings>,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) {
    if assigned.is_none_or(|required| protocol < required)
        && original.browser_tools != BrowserTools::Off
    {
        projected
            .get_or_insert_with(|| original.clone())
            .browser_tools = BrowserTools::Off;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Request as _;
    use crate::parking::ParkAfter;
    use crate::restart::RestartRecovery;
    use crate::settings::{AgentDrivers, SetNodeSettings};
    #[test]
    fn a_named_field_cannot_disappear_at_null_or_unknown_values() {
        for input in [
            r#"{"browser_tools":null}"#,
            r#"{"browser_tools":false}"#,
            r#"{"browser_tools":1}"#,
            r#"{"browser_tools":{}}"#,
            r#"{"browser_tools":[]}"#,
        ] {
            assert!(serde_json::from_str::<SetNodeSettings>(input).is_err());
        }
        let missing: SetNodeSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing.browser_tools, None);
        for (value, expected) in [
            ("on", BrowserTools::On),
            ("off", BrowserTools::Off),
            ("future", BrowserTools::Unknown),
        ] {
            let input = format!(r#"{{"browser_tools":"{value}"}}"#);
            let request: SetNodeSettings = serde_json::from_str(&input).unwrap();
            assert_eq!(request.browser_tools, Some(expected));
            assert_eq!(
                request.into_call().required_protocol(),
                crate::browser::BROWSER_PROTOCOL
            );
        }
    }
    #[test]
    fn a_browser_projection_composes_recovery_and_keeps_old_default_bytes() {
        let original = NodeSettings {
            park_after: ParkAfter::Never,
            drivers: AgentDrivers::default(),
            restart_recovery: RestartRecovery {
                enabled: true,
                prompt: "own synthetic startup prompt".parse().unwrap(),
            },
            browser_tools: BrowserTools::On,
        };
        let projected = original.for_protocol(crate::PROTOCOL_VERSION).unwrap();
        assert_eq!(projected.browser_tools, BrowserTools::Off);
        assert_eq!(projected.restart_recovery, RestartRecovery::default());
        let encoded = serde_json::to_string(&projected).unwrap();
        assert!(!encoded.contains("browser_tools"));
        assert!(!encoded.contains("restart_recovery"));
        let older: NodeSettings =
            serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
        assert_eq!(older.browser_tools, BrowserTools::Off);
        let assigned = Some(crate::PROTOCOL_VERSION);
        let mut projection = None;
        project(
            &original,
            &mut projection,
            crate::PROTOCOL_VERSION,
            assigned,
        );
        assert_eq!(projection, None);
    }
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    use crate::settings::SetNodeSettings;
    use crate::{Deliver, Forward, NodeId, Origin, OriginBy, RelayCall, Request as _, Scope};
    #[test]
    fn every_envelope_keeps_the_pending_named_field_requirement() {
        let node = NodeId::random();
        let origin = Origin {
            node,
            by: OriginBy::Owner,
            scopes: vec![Scope::Admin],
        };
        for preference in [BrowserTools::Off, BrowserTools::On, BrowserTools::Unknown] {
            let inner = SetNodeSettings {
                browser_tools: Some(preference),
                ..SetNodeSettings::default()
            }
            .into_call();
            let calls = [
                inner.clone(),
                Forward {
                    node,
                    call: Box::new(inner.clone()),
                }
                .into_call(),
                RelayCall {
                    node,
                    call: Box::new(inner.clone()),
                    origin: origin.clone(),
                }
                .into_call(),
                Deliver {
                    call: Box::new(inner),
                    origin: origin.clone(),
                }
                .into_call(),
            ];
            for call in calls {
                assert_eq!(call.required_protocol(), crate::browser::BROWSER_PROTOCOL);
            }
        }
    }
}
