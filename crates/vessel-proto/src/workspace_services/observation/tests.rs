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
fn mapped_ipv4_uses_the_same_parsed_loopback_class_at_both_edges() {
    for (address, expected) in [
        ("::ffff:127.0.0.1", ServiceLinkAccess::NodeLoopback),
        ("::ffff:127.3.5.7", ServiceLinkAccess::NodeLoopback),
        ("::1", ServiceLinkAccess::NodeLoopback),
        ("2001:db8::1", ServiceLinkAccess::AnonymousPublic),
        ("::ffff:192.0.2.1", ServiceLinkAccess::AnonymousPublic),
    ] {
        let url: ServiceProxyUrl = format!("http://[{address}]:8080/").parse().unwrap();
        let local = ServiceLink {
            access: ServiceLinkAccess::NodeLoopback,
            url: url.clone(),
        };
        let public = ServiceLink {
            access: ServiceLinkAccess::AnonymousPublic,
            url,
        };
        let local_json = format!(r#"{{"local":{}}}"#, serde_json::to_string(&local).unwrap());
        let public_json = format!(
            r#"{{"public":{}}}"#,
            serde_json::to_string(&public).unwrap()
        );
        let local = ServiceProxyUrls::new(Some(local), None);
        let public = ServiceProxyUrls::new(None, Some(public));
        let decoded_local = serde_json::from_str::<ServiceProxyUrls>(&local_json);
        let decoded_public = serde_json::from_str::<ServiceProxyUrls>(&public_json);
        match expected {
            ServiceLinkAccess::NodeLoopback => {
                assert!(local.is_ok());
                assert!(decoded_local.is_ok());
                assert_eq!(public, Err(InvalidServiceProxyUrls::Access));
                assert!(decoded_public.is_err());
            }
            ServiceLinkAccess::AnonymousPublic => {
                assert!(public.is_ok());
                assert!(decoded_public.is_ok());
                assert_eq!(local, Err(InvalidServiceProxyUrls::Access));
                assert!(decoded_local.is_err());
            }
            ServiceLinkAccess::Unknown => panic!("fixture needs a concrete role"),
        }
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
