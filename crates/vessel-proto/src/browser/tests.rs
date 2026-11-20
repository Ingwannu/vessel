//! Edge tests: ids, URLs, bounds, and the settings field against a peer
//! below [`crate::node::BROWSER_PROTOCOL`]. The constant is not a chosen
//! protocol number.

use super::*;
use crate::node::{BROWSER_PROTOCOL, ProtocolVersion};
use crate::parking::ParkAfter;
use crate::settings::{AgentDrivers, BrowserTools, NodeSettings, SetNodeSettings};
use crate::{Method, Request};

#[test]
fn ids_match_paseos_two_shapes_and_refs_are_at_e() {
    let minted = BrowserId::random();
    let parsed: BrowserId = minted.to_string().parse().unwrap();
    assert_eq!(parsed, minted);
    let stamp: BrowserId = "1710000000000-ab".parse().unwrap();
    assert_eq!(stamp.to_string(), "1710000000000-ab");
    assert!("not-an-id".parse::<BrowserId>().is_err());
    assert!("@e12".parse::<ElementRef>().is_ok());
    assert!("@e".parse::<ElementRef>().is_err());
    assert!("e1".parse::<ElementRef>().is_err());
}

#[test]
fn tool_urls_gain_http_and_reject_other_schemes_and_whitespace() {
    let page = BrowserUrl::parse_tool("example.com/a").unwrap();
    assert_eq!(page.as_str(), "http://example.com/a");
    assert!(BrowserUrl::parse_tool("https://example.com").is_ok());
    assert_eq!(
        BrowserUrl::parse_tool("javascript:alert(1)").unwrap_err(),
        InvalidInput::UrlScheme
    );
    assert_eq!(
        BrowserUrl::parse_tool("file:///etc/passwd").unwrap_err(),
        InvalidInput::UrlScheme
    );
    assert_eq!(
        BrowserUrl::parse_tool("data:text/html,hi").unwrap_err(),
        InvalidInput::UrlScheme
    );
    assert_eq!(
        BrowserUrl::parse_tool("http://exa mple.com").unwrap_err(),
        InvalidInput::UrlSpace
    );
    assert_eq!(
        BrowserUrl::parse_tool("").unwrap_err(),
        InvalidInput::UrlEmpty
    );
    assert!("javascript:alert(1)".parse::<BrowserUrl>().is_err());
    let secret = BrowserUrl::parse_tool("https://user:token@example.com/a").unwrap();
    let debug = format!("{secret:?}");
    assert!(!debug.contains("token"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
}

#[test]
fn viewport_wait_and_log_bounds_reject_the_edges() {
    assert!(ViewportSide::new(0).is_err());
    assert!(ViewportSide::new(8193).is_err());
    assert_eq!(ViewportSide::new(8192).unwrap().get(), 8192);
    assert!(WaitMs::new(0).is_err());
    assert!(WaitMs::new(30_001).is_err());
    assert_eq!(WaitMs::new(30_000).unwrap().get(), 30_000);
    assert!(LogLimit::new(0).is_err());
    assert!(LogLimit::new(201).is_err());
    assert!(wait_expect(Some("a"), Some("b")).is_err());
    assert!(wait_expect(None, None).is_err());
    assert!(wait_expect(Some("a"), None).is_ok());
    assert!(CssDelta::new(f64::NAN, 1.0).is_err());
    assert!(CssDelta::new(1.0, f64::INFINITY).is_err());
    assert!(CssDelta::new(1.5, -2.0).is_ok());
}

#[test]
fn browser_tools_are_absent_when_off_for_a_peer_below_the_constant() {
    let below = ProtocolVersion::new(
        BROWSER_PROTOCOL
            .checked_sub(1)
            .unwrap(),
    );
    assert!(Method::BrowserCall.since() > below);
    assert_eq!(Method::BrowserCall.since().get(), BROWSER_PROTOCOL);
    let off = NodeSettings {
        park_after: ParkAfter::Never,
        drivers: AgentDrivers::default(),
        browser_tools: BrowserTools::Off,
    };
    let json = serde_json::to_string(&off).unwrap();
    assert!(
        !json.contains("browser_tools"),
        "an older peer must not have to know the key: {json}"
    );
    let read: NodeSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(read.browser_tools, BrowserTools::Off);
    let older: NodeSettings = serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
    assert_eq!(older.browser_tools, BrowserTools::Off);
    let change = SetNodeSettings {
        browser_tools: Some(BrowserTools::On),
        ..SetNodeSettings::default()
    };
    assert!(change.clone().into_call().since() > below);
    assert_eq!(change.into_call().since().get(), BROWSER_PROTOCOL);
    let unknown: BrowserTools = serde_json::from_str("\"later\"").unwrap();
    assert_eq!(unknown, BrowserTools::Unknown);
}

#[test]
fn a_refusal_names_the_reason_and_not_the_page() {
    let error = refusal(BrowserFailure::NoEngine);
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert_eq!(
        error.detail,
        Some(ErrorDetail::Browser {
            reason: BrowserFailure::NoEngine
        })
    );
    assert!(!error.message.contains("example.com"));
    assert!(!error.message.contains("<html"));
    let invalid = invalid(InvalidInput::UrlScheme);
    assert_eq!(invalid.code, ErrorCode::InvalidRequest);
    assert!(!invalid.message.contains("javascript"));
}

#[test]
fn tool_names_are_the_twenty_two() {
    assert_eq!(TOOL_NAMES.len(), 22);
}
