//! Wire fixtures verify projection distinctions without any network authority.

use super::*;

fn link(access: ServiceLinkAccess) -> ServiceLink {
    let url = match access {
        ServiceLinkAccess::NodeLoopback => "http://service.localhost:8080/",
        ServiceLinkAccess::AnonymousPublic | ServiceLinkAccess::Unknown => {
            "https://service.example.test/"
        }
    };
    ServiceLink {
        access,
        url: url.parse().unwrap(),
    }
}

#[test]
fn both_urls_survive_and_public_first_preference_is_not_stored() {
    let local = link(ServiceLinkAccess::NodeLoopback);
    let public = link(ServiceLinkAccess::AnonymousPublic);
    let urls = ServiceProxyUrls::new(Some(local.clone()), Some(public.clone())).unwrap();
    assert_eq!(urls.local(), Some(&local));
    assert_eq!(urls.public(), Some(&public));
    assert_eq!(urls.preferred(), Some(&public));
    let text = serde_json::to_string(&urls).unwrap();
    assert!(!text.contains("preferred"));
    assert_eq!(
        serde_json::from_str::<ServiceProxyUrls>(&text).unwrap(),
        urls
    );
    let only_local = ServiceProxyUrls::new(Some(local.clone()), None).unwrap();
    assert_eq!(only_local.preferred(), Some(&local));
    let only_public = ServiceProxyUrls::new(None, Some(public.clone())).unwrap();
    assert_eq!(only_public.preferred(), Some(&public));
}

#[test]
fn role_invariants_hold_at_constructor_and_decoder() {
    assert_eq!(
        ServiceProxyUrls::new(None, None),
        Err(InvalidServiceProxyUrls::Empty)
    );
    for (local, public) in [
        (Some(link(ServiceLinkAccess::AnonymousPublic)), None),
        (None, Some(link(ServiceLinkAccess::NodeLoopback))),
        (Some(link(ServiceLinkAccess::Unknown)), None),
    ] {
        assert_eq!(
            ServiceProxyUrls::new(local, public),
            Err(InvalidServiceProxyUrls::Access)
        );
    }
    for text in [
        "{}",
        r#"{"local":{"access":"anonymous_public","url":"https://service.example.test/"}}"#,
        r#"{"public":{"access":"node_loopback","url":"https://service.example.test/"}}"#,
        r#"{"local":{"access":"node_loopback","url":"https://service.example.test/"}}"#,
        r#"{"public":{"access":"anonymous_public","url":"http://127.0.0.1/"}}"#,
    ] {
        assert!(serde_json::from_str::<ServiceProxyUrls>(text).is_err());
    }
}

#[test]
fn planned_registered_missing_and_future_health_are_distinct_observations() {
    let urls = ServiceProxyUrls::new(None, Some(link(ServiceLinkAccess::AnonymousPublic))).unwrap();
    let planned = ServiceProxyState::Planned { urls: urls.clone() };
    let registered = ServiceProxyState::Registered { urls };
    assert_ne!(planned, registered);
    for value in [
        ServiceHealth::NotObserved,
        ServiceHealth::Pending,
        ServiceHealth::Healthy,
        ServiceHealth::Unhealthy,
        ServiceHealth::Unavailable {
            reason: ServiceUnavailable::OwnershipUnavailable,
        },
        ServiceHealth::Unknown,
    ] {
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<ServiceHealth>(&text).unwrap(), value);
    }
    assert_eq!(
        serde_json::from_str::<ServiceHealth>(r#"{"kind":"future"}"#).unwrap(),
        ServiceHealth::Unknown
    );
    assert_eq!(
        serde_json::from_str::<ServiceProxyState>(r#"{"kind":"future"}"#).unwrap(),
        ServiceProxyState::Unknown
    );
    assert_eq!(
        serde_json::from_str::<ServicePortPlan>(r#"{"kind":"future"}"#).unwrap(),
        ServicePortPlan::Unknown
    );
    assert_ne!(
        ServicePortPlan::Unplanned,
        ServicePortPlan::Unavailable {
            reason: ServiceUnavailable::PlanUnavailable
        }
    );
}
