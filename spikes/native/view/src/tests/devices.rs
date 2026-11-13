//! The page's `data/devices.test.ts`, and the pairing link's showing.

use chrono::{DateTime, Duration, Utc};
use vessel_proto::DeviceInfo;

use crate::Lang;
use crate::devices::{LinkShown, device_line, invite_label, paired_devices};

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn phone() -> DeviceInfo {
    serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "label": "phone",
        "created_at": "2026-11-11T05:00:00Z",
        "last_seen": "2026-11-11T06:57:00Z",
        "scopes": ["read", "operate"],
    }))
    .unwrap()
}

#[test]
fn says_what_a_device_may_do_and_when_it_was_seen_and_paired() {
    let now = at("2026-11-11T07:00:00Z");
    assert_eq!(
        device_line(Lang::En, &phone(), now),
        ["Can read, run sessions", "seen 3m ago", "paired 2h ago"]
    );
    let unseen = DeviceInfo {
        last_seen: None,
        ..phone()
    };
    assert_eq!(device_line(Lang::En, &unseen, now)[1], "never seen");
}

#[test]
fn lists_only_the_devices_that_still_have_access() {
    let revoked = DeviceInfo {
        revoked_at: Some(at("2026-11-11T06:00:00Z")),
        ..phone()
    };
    let devices = [phone(), revoked];
    assert_eq!(paired_devices(&devices), [&devices[0]]);
}

#[test]
fn a_new_devices_label_is_trimmed_and_none_when_empty() {
    assert_eq!(invite_label("  tablet "), Some("tablet".to_owned()));
    assert_eq!(invite_label("   "), None);
}

/// The link is a credential: shown when asked, hidden again after 30 s.
#[test]
fn a_shown_link_hides_itself_after_thirty_seconds() {
    let shown = at("2026-11-11T07:00:00Z");
    assert!(!LinkShown::Hidden.visible(shown));
    assert_eq!(LinkShown::Hidden.hides_at(), None);
    let since = LinkShown::Since(shown);
    assert!(since.visible(shown + Duration::seconds(29)));
    assert!(!since.visible(shown + Duration::seconds(30)));
    assert_eq!(since.hides_at(), Some(shown + Duration::seconds(30)));
}
