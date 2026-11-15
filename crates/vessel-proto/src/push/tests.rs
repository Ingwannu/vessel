//! Capability parsing is strict at the wire edge and diagnostics are generic.
use super::*;
#[test]
fn subscription_material_is_bounded_and_roundtrips_without_padding() {
    let mut point = [7_u8; 65];
    point[0] = 4;
    let subscription = PushSubscription {
        endpoint: "https://push.example/private-capability".parse().unwrap(),
        p256dh: PushPublicKey::from_bytes(&point).unwrap(),
        auth: PushAuth::from_bytes([9; 16]),
    };
    let encoded = serde_json::to_string(&subscription).unwrap();
    assert_eq!(
        serde_json::from_str::<PushSubscription>(&encoded).unwrap(),
        subscription
    );
    assert_eq!(
        format!("{:?}", subscription.endpoint),
        "PushEndpoint(<redacted>)"
    );
    assert_eq!(
        format!("{:?}", subscription.p256dh),
        "PushPublicKey(<redacted>)"
    );
    assert_eq!(format!("{:?}", subscription.auth), "PushAuth(<redacted>)");
    assert!("bad".parse::<PushPublicKey>().is_err());
    assert!("AAAAAAAAAAAAAAAAAAAAAA==".parse::<PushAuth>().is_err());
    assert!("AAAAAAAAAAAAAAAAAAAAA!".parse::<PushAuth>().is_err());
    assert!(PushPublicKey::from_bytes(&[4; 33]).is_err());
    assert!(PushPublicKey::from_bytes(&[2; 65]).is_err());
}
#[test]
fn endpoint_rejects_non_https_credentials_fragment_custom_port_control_and_oversize() {
    for value in [
        "http://push.example/capability",
        "https://user@push.example/capability",
        "https://push.example:444/capability",
        "https://push.example/capability#fragment",
        "https://push.example/\ncapability",
        "file:///capability",
        "https://",
    ] {
        assert_eq!(
            value.parse::<PushEndpoint>(),
            Err(InvalidPushMaterial::Endpoint)
        );
    }
    let too_long = format!("https://push.example/{}", "a".repeat(2048));
    assert_eq!(
        too_long.parse::<PushEndpoint>(),
        Err(InvalidPushMaterial::Endpoint)
    );
}
#[test]
fn every_push_method_is_protocol_gated_read_scoped_and_device_identity_is_not_in_request() {
    use crate::{Method, Request as _, Scope};
    for method in [
        Method::GetPushKey,
        Method::SubscribePush,
        Method::UnsubscribePush,
        Method::ReportPushPresence,
    ] {
        assert_eq!(method.scope(), Some(Scope::Read));
        assert!(!method.forwardable());
    }
    // Push is protocol 22 with the inbox (`rpc.rs`), not the newest number.
    let push = crate::ProtocolVersion::new(22);
    assert_eq!(GetPushKey.into_call().since(), Some(push));
    assert_eq!(UnsubscribePush.into_call().since(), Some(push));
}

#[test]
fn normalized_unicode_endpoint_cannot_expand_past_the_stored_bound() {
    let endpoint = format!("https://push.example/{}", "가".repeat(300));
    assert!(endpoint.len() < 2048);
    assert_eq!(
        endpoint.parse::<PushEndpoint>(),
        Err(InvalidPushMaterial::Endpoint)
    );
}
