//! Decoder and diagnostic probes contain only synthetic URL material.

use std::error::Error as _;

use super::*;

#[test]
fn origin_text_is_canonical_and_round_trips_without_route_authority() {
    for (text, expected) in [
        ("https://EXAMPLE.test:443", "https://example.test/"),
        ("http://example.test:8080/", "http://example.test:8080/"),
        ("http://127.0.0.1:8080", "http://127.0.0.1:8080/"),
        ("http://[::1]:8080/", "http://[::1]:8080/"),
        ("http://localhost", "http://localhost/"),
    ] {
        let value: ServiceProxyUrl = text.parse().unwrap();
        assert_eq!(value.as_str(), expected);
        assert_eq!(value.to_string(), expected);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<ServiceProxyUrl>(&json).unwrap(),
            value
        );
    }
}

#[test]
fn userinfo_paths_and_parser_repairs_cannot_become_service_origins() {
    for text in [
        "ftp://example.test/",
        "http://@example.test/",
        "https://user:synthetic-secret@example.test/",
        "http://example.test/path",
        "http://example.test/./",
        "http://example.test/%2e/",
        "http://example.test//",
        "http://example.test?",
        "http://example.test#",
        "http:example.test",
        "http:///example.test/",
        "http://example.test\\",
        "http://example.test/\n",
        "http://example.test/\u{85}",
    ] {
        assert!(text.parse::<ServiceProxyUrl>().is_err(), "{text:?}");
        let encoded = serde_json::to_string(text).unwrap();
        assert!(serde_json::from_str::<ServiceProxyUrl>(&encoded).is_err());
    }
    let below = ServicePort::MIN
        .get()
        .checked_sub(ServicePort::MIN.get())
        .unwrap();
    let zero_port = format!("http://example.test:{below}/");
    assert!(zero_port.parse::<ServiceProxyUrl>().is_err());
    let encoded = serde_json::to_string(&zero_port).unwrap();
    assert!(serde_json::from_str::<ServiceProxyUrl>(&encoded).is_err());
}

#[test]
fn bounds_are_derived_from_their_one_owner() {
    let over = "x".repeat(MAX_PROXY_URL_BYTES.checked_add(1).unwrap());
    let error = over.parse::<ServiceProxyUrl>().unwrap_err();
    assert!(matches!(error, InvalidServiceProxyUrl::Length));
    assert!(error.to_string().contains(&MAX_PROXY_URL_BYTES.to_string()));

    let label = "a".repeat(MAX_PROXY_LABEL_BYTES);
    let prefix = [label.as_str(); 3].join(".");
    let remaining = MAX_PROXY_HOST_BYTES
        .checked_sub(prefix.len())
        .unwrap()
        .checked_sub(1)
        .unwrap();
    let host = format!("{prefix}.{}", "b".repeat(remaining));
    assert_eq!(host.len(), MAX_PROXY_HOST_BYTES);
    assert!(
        format!("https://{host}/")
            .parse::<ServiceProxyUrl>()
            .is_ok()
    );
    for host in [
        format!("{host}b"),
        "a".repeat(MAX_PROXY_LABEL_BYTES.checked_add(1).unwrap()),
    ] {
        assert!(
            format!("https://{host}/")
                .parse::<ServiceProxyUrl>()
                .is_err()
        );
    }
}

#[test]
fn refusal_formatters_and_all_sources_do_not_echo_url_input() {
    const CANARY: &str = "synthetic-proxy-origin-canary";
    for text in [
        format!("https://user:{CANARY}@example.test/"),
        format!("https://example.test/{CANARY}"),
        format!("https://{CANARY}:invalid/"),
    ] {
        let error = text.parse::<ServiceProxyUrl>().unwrap_err();
        let mut cause: &(dyn std::error::Error + 'static) = &error;
        loop {
            assert!(!format!("{cause:?}").contains(CANARY));
            assert!(!format!("{cause}").contains(CANARY));
            match cause.source() {
                Some(source) => cause = source,
                None => break,
            }
        }
    }
    let syntax = "https://example.test:invalid/"
        .parse::<ServiceProxyUrl>()
        .unwrap_err();
    assert!(syntax.source().is_some());
    let below = ServicePort::MIN
        .get()
        .checked_sub(ServicePort::MIN.get())
        .unwrap();
    let port = format!("http://example.test:{below}/")
        .parse::<ServiceProxyUrl>()
        .unwrap_err();
    assert!(port.source().is_some());
}
