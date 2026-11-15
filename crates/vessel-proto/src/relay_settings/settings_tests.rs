//! Existing settings methods keep their authority and refuse pending fields.
use super::*;
use crate::settings::{
    AgentDrivers, GetNodeSettings, NodeSettings, SetAgentDrivers, SetNodeSettings,
};
use crate::{Reply, Request as _, Scope};

fn report() -> RelaySettings {
    RelaySettings {
        configured: ConfiguredRelays::Disabled,
        fleet_callback: CallbackRelays::Present,
    }
}

#[test]
fn an_older_read_has_unknown_relay_support_instead_of_invented_disabled_consent() {
    let settings: NodeSettings =
        serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
    assert_eq!(settings.relay, None);
    assert!(!serde_json::to_string(&settings).unwrap().contains("relay"));
    let change: SetNodeSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(change.relay, None);
}

#[test]
fn relay_fields_do_not_change_existing_method_scopes_shapes_or_forwarding() {
    assert_eq!(GetNodeSettings::METHOD.scope(), Some(Scope::Read));
    assert_eq!(SetNodeSettings::METHOD.scope(), Some(Scope::Admin));
    assert!(!GetNodeSettings::METHOD.forwardable());
    assert!(!SetNodeSettings::METHOD.forwardable());
    assert_eq!(
        GetNodeSettings::METHOD.shape(),
        crate::rpc::MethodShape::Unary
    );
    assert_eq!(
        SetNodeSettings::METHOD.shape(),
        crate::rpc::MethodShape::Unary
    );
}

#[test]
fn even_mixed_and_unknown_choices_require_the_assigned_relay_field_version() {
    let legacy = SetNodeSettings {
        relay: None,
        park_after: Some(crate::parking::ParkAfter::Never),
        drivers: Some(SetAgentDrivers::default()),
    };
    let baseline = legacy.into_call().since().unwrap();
    for relay in [
        RelayChange::Enabled,
        RelayChange::Disabled,
        RelayChange::Unknown,
    ] {
        let change = SetNodeSettings {
            relay: Some(relay),
            ..legacy
        };
        let expected = RELAY_SETTINGS_PROTOCOL.map(|assigned| assigned.max(baseline));
        assert_eq!(change.into_call().required_protocol(), expected);
    }
}

#[test]
fn both_read_and_legacy_write_replies_hide_pending_or_unsupported_relay_values() {
    let original = NodeSettings {
        relay: Some(report()),
        park_after: crate::parking::ParkAfter::Never,
        drivers: AgentDrivers::default(),
        restart_recovery: crate::restart::RestartRecovery::default(),
    };
    let mut expected = original.clone();
    expected.relay = None;
    for protocol in (crate::MIN_PROTOCOL_VERSION.get()..=crate::PROTOCOL_VERSION.get())
        .map(ProtocolVersion::new)
        .filter(|protocol| RELAY_SETTINGS_PROTOCOL.is_none_or(|required| *protocol < required))
    {
        for (reply, projected) in [
            (
                Reply::GetNodeSettings(original.clone()),
                Reply::GetNodeSettings(expected.clone()),
            ),
            (
                Reply::SetNodeSettings(original.clone()),
                Reply::SetNodeSettings(expected.clone()),
            ),
        ] {
            assert_eq!(reply.for_protocol(protocol), Some(projected));
        }
    }
    if let Some(assigned) = RELAY_SETTINGS_PROTOCOL {
        assert_eq!(original.for_protocol(assigned), None);
    }
}

#[test]
fn extra_route_token_and_home_fields_cannot_survive_the_closed_read_projection() {
    let input = r#"{
        "configured":{"state":"disabled","route":"ws://secret.invalid/?token=canary"},
        "fleet_callback":"absent",
        "token":"canary-token", "pairing_link":"canary-invitation", "home_path":"/home/canary"
    }"#;
    let value: RelaySettings = serde_json::from_str(input).unwrap();
    let wire = serde_json::to_string(&value).unwrap();
    let debug = format!("{value:?}");
    for private in [
        "secret.invalid",
        "canary",
        "/home/",
        "pairing_link",
        "token",
    ] {
        assert!(!wire.contains(private));
        assert!(!debug.contains(private));
    }
    assert_eq!(
        wire,
        r#"{"configured":{"state":"disabled"},"fleet_callback":"absent"}"#
    );
}
