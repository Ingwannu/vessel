use super::*;
use vessel_proto::{InvitationSecret, NodePublicKey, OfferVersion, PairingOffer};

fn offer() -> PairingOffer {
    PairingOffer {
        v: OfferVersion,
        node: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        key: NodePublicKey::from_bytes([4; 32]),
        direct: vec!["wss://hidden-user:hidden-password@[::1]:7443/private-path?private-query#private-fragment"
            .parse().unwrap()],
        relays: vec!["wss://relay.invalid:443/secret-ticket?secret-token"
            .parse().unwrap()],
        invite: InvitationSecret::from_bytes([7; 16]),
        expires: "2026-11-20T00:00:00Z".parse().unwrap(),
    }
}

#[test]
fn safe_routes_keep_kind_order_bilingual_kinds_and_bare_endpoints() {
    let routes = OfferRoutes::of(&offer());
    let [direct, relay] = routes.rows() else {
        panic!("two projected route rows required");
    };
    assert!(matches!(direct.route(), Route::Direct(_)));
    assert!(matches!(relay.route(), Route::Relay(_)));
    assert_eq!(direct.route().endpoint().as_str(), "[::1]:7443");
    assert_eq!(relay.route().endpoint().as_str(), "relay.invalid:443");
    assert_eq!(direct.kind(Lang::En), "TCP");
    assert_eq!(direct.kind(Lang::Ko), "TCP");
    assert_eq!(relay.kind(Lang::En), "Relay");
    assert_eq!(relay.kind(Lang::Ko), "릴레이");
    assert_eq!(&**direct.endpoint(), "[::1]:7443");
    assert_eq!(&**relay.endpoint(), "relay.invalid:443");
    let diagnostic = format!("{routes:?}");
    for hidden in [
        "hidden-user",
        "hidden-password",
        "private-path",
        "private-query",
        "private-fragment",
        "secret-ticket",
        "secret-token",
    ] {
        assert!(!diagnostic.contains(hidden));
        for row in routes.rows() {
            assert!(!row.endpoint().contains(hidden));
            for lang in [Lang::En, Lang::Ko] {
                assert!(!row.kind(lang).contains(hidden));
            }
        }
    }
    let cloned = routes.clone();
    assert!(Arc::ptr_eq(&routes.0, &cloned.0));
}

#[test]
fn no_route_claim_does_not_invent_a_relay() {
    let mut offer = offer();
    offer.direct.clear();
    offer.relays.clear();
    assert!(OfferRoutes::of(&offer).rows().is_empty());
}
