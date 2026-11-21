//! Settings values enter once through their constructors, and the wire
//! shapes are Paseo's (`settings.ts`'s read and write results).

use super::*;
use crate::plugins::SettingsId;

fn id(raw: &str) -> SettingsId {
    raw.parse().unwrap()
}

#[test]
fn ids_and_versions_follow_paseos_define_settings() {
    for good in ["display", "a", "my_prefs-2"] {
        assert!(good.parse::<SettingsId>().is_ok(), "{good}");
    }
    for bad in ["", "Display", "2fa", "a.b", "a/b", &"a".repeat(65)] {
        assert!(bad.parse::<SettingsId>().is_err(), "{bad}");
    }
    assert_eq!(SettingsVersion::new(0), None);
    assert_eq!(SettingsVersion::new(1).map(SettingsVersion::get), Some(1));
    let largest = 9_007_199_254_740_991_u64;
    assert!(SettingsVersion::new(largest).is_some());
    assert_eq!(SettingsVersion::new(largest.checked_add(1).unwrap()), None);
    assert!(serde_json::from_str::<SettingsVersion>("0").is_err());
}

#[test]
fn a_revision_is_missing_or_a_sha256() {
    let digest = "a".repeat(64);
    assert_eq!(
        serde_json::from_str::<SettingsRevision>("\"missing\"").unwrap(),
        SettingsRevision::missing()
    );
    assert_eq!(
        SettingsRevision::from_sha256_hex(&digest).map(|revision| revision.as_str().len()),
        Some(64)
    );
    for bad in ["", "MISSING", &"A".repeat(64), &"a".repeat(63)] {
        assert!(
            serde_json::from_str::<SettingsRevision>(&format!("\"{bad}\"")).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn a_helpers_message_is_bounded_and_has_no_controls() {
    let message = SettingsMessage::bounded("size:\tmust be\na number\u{7}");
    assert_eq!(message.as_str(), "size: must be a number");
    let long = SettingsMessage::bounded(&"가".repeat(400));
    assert!(long.as_str().len() <= MAX_SETTINGS_MESSAGE_BYTES);
    assert!(long.as_str().chars().all(|c| c == '가'));
    assert!(serde_json::from_str::<SettingsMessage>("\"a\\nb\"").is_err());
}

#[test]
fn declarations_are_few_and_unique() {
    let one = |raw: &str| SettingsDeclaration {
        id: id(raw),
        version: SettingsVersion::new(1).unwrap(),
    };
    let declared = SettingsDeclarations::new(vec![one("display"), one("sync")]).unwrap();
    assert_eq!(
        declared.version_of(&id("sync")).map(SettingsVersion::get),
        Some(1)
    );
    assert_eq!(declared.version_of(&id("other")), None);
    assert_eq!(
        SettingsDeclarations::new(vec![one("display"), one("display")]),
        Err(SettingsValueError::Duplicate)
    );
    let many = (0..=MAX_SETTINGS_DOCUMENTS)
        .map(|n| one(&format!("s{n}")))
        .collect();
    assert_eq!(
        SettingsDeclarations::new(many),
        Err(SettingsValueError::TooMany)
    );
}

#[test]
fn the_steps_and_the_verdict_have_their_tags() {
    let parse = SettingsStep::Parse {
        values: HelperPayload::try_from(RawJson::from_text("{}").unwrap()).unwrap(),
    };
    assert_eq!(
        serde_json::to_string(&parse).unwrap(),
        r#"{"step":"parse","values":{}}"#
    );
    let changed: SettingsStep = serde_json::from_str(
        r#"{"step":"changed","state":{"state":"invalid","revision":"missing"}}"#,
    )
    .unwrap();
    assert_eq!(changed.payload(), None);
    let verdict: SettingsVerdict =
        serde_json::from_str(r#"{"verdict":"invalid","message":"size must be a number"}"#).unwrap();
    assert!(matches!(verdict, SettingsVerdict::Invalid { .. }));
    assert!(
        serde_json::from_str::<SettingsVerdict>(r#"{"verdict":"valid","values":{},"x":1}"#)
            .is_err()
    );
}

#[test]
fn replies_are_paseos_read_and_write_results() {
    let ready = PluginSettingsRead::Ready {
        revision: SettingsRevision::missing(),
        values: RawJson::from_text(r#"{"color":"blue"}"#).unwrap(),
    };
    assert_eq!(
        serde_json::to_string(&ready).unwrap(),
        r#"{"status":"ready","revision":"missing","values":{"color":"blue"}}"#
    );
    let conflict = PluginSettingsWrite::Conflict {
        error: SettingsMessage::bounded("Settings changed on another client."),
    };
    assert_eq!(
        serde_json::to_string(&conflict).unwrap(),
        r#"{"status":"conflict","error":"Settings changed on another client."}"#
    );
}
