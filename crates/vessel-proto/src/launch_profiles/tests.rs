//! Whole-value/edge fixtures. No method, storage, agent or launch behavior is implied.

use super::*;
use crate::{ChoiceValue, HarnessKind, LaunchProfileId, OptionChoice, OptionId, OptionValueId};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct UncheckedSnapshot<'a> {
    revision: LaunchProfileRevision,
    profiles: &'a [LaunchProfile],
}

fn select(id: &str, value: &str) -> LaunchChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
    .try_into()
    .unwrap()
}
fn profile(index: usize) -> LaunchProfile {
    LaunchProfile::new(
        format!("00000000-0000-4000-8000-{index:012}")
            .parse::<LaunchProfileId>()
            .unwrap(),
        "Repeated name".parse().unwrap(),
        LaunchSelection::new(HarnessKind::Codex.into(), None, LaunchChoices::default()),
    )
}
fn roundtrip<T>(value: &T)
where
    T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + PartialEq,
{
    let bytes = serde_json::to_vec(value).unwrap();
    assert_eq!(&serde_json::from_slice::<T>(&bytes).unwrap(), value);
}

#[test]
fn text_boundaries_use_scalars_for_names_bytes_for_notes_and_typed_keys() {
    let name: LaunchProfileName = "  이름  ".parse().unwrap();
    assert_eq!(name.as_str(), "이름");
    assert!(
        "한"
            .repeat(LaunchProfileName::MAX_CHARS)
            .parse::<LaunchProfileName>()
            .is_ok()
    );
    assert!(
        "한"
            .repeat(LaunchProfileName::MAX_CHARS + 1)
            .parse::<LaunchProfileName>()
            .is_err()
    );
    for bad in [
        "",
        "  ",
        "a\nb",
        "a\u{0085}b",
        "a\u{2028}b",
        "a\u{2029}b",
        "a\0b",
    ] {
        assert!(bad.parse::<LaunchProfileName>().is_err());
    }
    let notes: LaunchProfileNotes = "  First\r\n\tSecond  ".parse().unwrap();
    assert_eq!(notes.as_str(), "First\r\n\tSecond");
    assert!(
        "x".repeat(LaunchProfileNotes::MAX_BYTES)
            .parse::<LaunchProfileNotes>()
            .is_ok()
    );
    assert!(
        "x".repeat(LaunchProfileNotes::MAX_BYTES + 1)
            .parse::<LaunchProfileNotes>()
            .is_err()
    );
    assert!(
        "한"
            .repeat(LaunchProfileNotes::MAX_BYTES)
            .parse::<LaunchProfileNotes>()
            .is_err()
    );
    for bad in ["", " \n\t ", "a\0b", "a\u{001b}b", "a\u{0085}b"] {
        assert!(bad.parse::<LaunchProfileNotes>().is_err());
    }
    assert!(
        "x".repeat(LaunchAppearanceKey::MAX_BYTES)
            .parse::<LaunchIconKey>()
            .is_ok()
    );
    assert!(
        "x".repeat(LaunchAppearanceKey::MAX_BYTES + 1)
            .parse::<LaunchColorKey>()
            .is_err()
    );
    for bad in ["", "한", "robot/icon", "rgb(1)", "▾"] {
        assert!(bad.parse::<LaunchIconKey>().is_err());
    }
    roundtrip(&name);
    roundtrip(&notes);
    roundtrip(&"robot-key".parse::<LaunchIconKey>().unwrap());
    roundtrip(&"green".parse::<LaunchColorKey>().unwrap());
    assert!(!format!("{notes:?}").contains("Second"));
}

#[test]
fn choices_retain_case_order_and_kinds_but_refuse_duplicate_unbounded_or_untyped_ids() {
    let first = select("Model", "Exact-Value");
    let toggle: LaunchChoice = OptionChoice {
        id: OptionId::new("fast"),
        value: ChoiceValue::Toggle { on: true },
    }
    .try_into()
    .unwrap();
    let choices = LaunchChoices::try_from(vec![first.clone(), toggle]).unwrap();
    assert_eq!(
        choices.as_slice().first().unwrap().as_choice().id.as_str(),
        "Model"
    );
    assert_eq!(
        choices.as_slice().get(1).unwrap().as_choice().value,
        ChoiceValue::Toggle { on: true }
    );
    roundtrip(&choices);
    assert!(matches!(
        LaunchChoices::try_from(vec![first.clone(), first]),
        Err(InvalidLaunchValue::DuplicateChoice)
    ));
    let full: Vec<_> = (0..LaunchChoices::MAX_CHOICES)
        .map(|n| select(&format!("o{n}"), "v"))
        .collect();
    assert!(LaunchChoices::try_from(full.clone()).is_ok());
    let mut too_many = full;
    too_many.push(select("excess", "v"));
    assert!(matches!(
        LaunchChoices::try_from(too_many.clone()),
        Err(InvalidLaunchValue::Choices)
    ));
    assert!(
        serde_json::from_slice::<LaunchChoices>(&serde_json::to_vec(&too_many).unwrap()).is_err()
    );
    let boundary = "x".repeat(LaunchChoice::MAX_ID_BYTES);
    roundtrip(&select(&boundary, &boundary));
    for bad in [
        String::new(),
        "\0".to_owned(),
        "\n".to_owned(),
        "x".repeat(LaunchChoice::MAX_ID_BYTES + 1),
    ] {
        for choice in [
            OptionChoice {
                id: OptionId::new(bad.clone()),
                value: ChoiceValue::Toggle { on: false },
            },
            OptionChoice {
                id: OptionId::new("safe"),
                value: ChoiceValue::Select {
                    value: OptionValueId::new(bad.clone()),
                },
            },
        ] {
            assert!(LaunchChoice::try_from(choice).is_err());
        }
    }
    for bad in [
        r#"{"id":"x","kind":"select","value":null}"#,
        r#"{"id":"x","kind":"toggle","on":"secret-input"}"#,
        r#"{"id":"x","kind":"toggle","on":true,"secret-input":"x"}"#,
        r#"{"id":"x","kind":"future","value":"secret-input"}"#,
    ] {
        let error = serde_json::from_str::<LaunchChoice>(bad)
            .unwrap_err()
            .to_string();
        assert!(!error.contains("secret-input"), "{error}");
    }
}

#[test]
fn the_excess_array_item_is_refused_before_its_deserializer_runs() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DECODED: AtomicUsize = AtomicUsize::new(0);
    // This deserializer deliberately fails with a distinct diagnostic if
    // called. Reject's seed must win before this element is constructed.
    struct Forbidden;
    impl<'de> Deserialize<'de> for Forbidden {
        fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
            let _previous = DECODED.fetch_add(1, Ordering::SeqCst);
            Err(serde::de::Error::custom("EXCESS DESERIALIZER WAS CALLED"))
        }
    }
    // An empty capacity isolates admission independently of normal item parsing.
    let mut input = serde_json::Deserializer::from_str("[{}]");
    let error = codec::bounded::<_, Forbidden, 0>(&mut input)
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains("EXCESS DESERIALIZER WAS CALLED"));
    assert_eq!(DECODED.load(Ordering::SeqCst), 0);
    let rows = vec![select("x", "v"); LaunchChoices::MAX_CHOICES];
    let encoded = serde_json::to_string(&rows).unwrap();
    let prefix = encoded.strip_suffix(']').unwrap();
    // Even malformed extra JSON is not parsed into a further domain item.
    let error =
        serde_json::from_str::<LaunchChoices>(&format!("{prefix},{{\"secret-input\":null}}]"))
            .unwrap_err()
            .to_string();
    assert!(!error.contains("secret-input"));
}

#[test]
fn record_replacement_clears_optional_fields_without_losing_id_or_order() {
    let id = profile(0).id();
    let original = profile(0)
        .with_notes(Some("public guidance".parse().unwrap()))
        .with_icon(Some("robot".parse().unwrap()))
        .with_color(Some("green".parse().unwrap()));
    roundtrip(&original);
    let replacement: LaunchProfile = serde_json::from_str(&format!(
        r#"{{"id":"{id}","name":"Other","launch":{{"harness":"codex","account_profile":"offline-account","options":[]}}}}"#
    )).unwrap();
    assert_eq!(replacement.id(), id);
    assert_eq!(replacement.notes(), None);
    assert_eq!(replacement.icon(), None);
    assert_eq!(replacement.color(), None);
    assert_eq!(
        replacement.launch().account_profile().unwrap().as_str(),
        "offline-account"
    );
    let list = LaunchProfiles::try_from(vec![profile(1), replacement.clone()]).unwrap();
    assert_eq!(
        list.as_slice()
            .iter()
            .map(LaunchProfile::id)
            .collect::<Vec<_>>(),
        vec![profile(1).id(), id]
    );
    let snapshot = LaunchProfilesSnapshot::new(LaunchProfileRevision::new(7), list);
    roundtrip(&snapshot);
    assert_eq!(snapshot.revision().get(), 7);
    assert!(matches!(
        LaunchProfiles::try_from(vec![replacement.clone(), replacement]),
        Err(InvalidLaunchValue::DuplicateProfile)
    ));
}

#[test]
fn whole_lists_refuse_a_bad_later_record_unknown_fields_and_invalid_revisions() {
    let complete: Vec<_> = (0..LaunchProfiles::MAX_PROFILES).map(profile).collect();
    let full = LaunchProfiles::try_from(complete.clone()).unwrap();
    roundtrip(&full);
    let mut excess = complete;
    excess.push(profile(LaunchProfiles::MAX_PROFILES));
    assert!(matches!(
        LaunchProfiles::try_from(excess.clone()),
        Err(InvalidLaunchValue::Profiles)
    ));
    assert!(
        serde_json::from_slice::<LaunchProfiles>(&serde_json::to_vec(&excess).unwrap()).is_err()
    );
    let first = serde_json::to_string(&profile(0)).unwrap();
    for bad in [
        format!("[{first},{{\"secret-input\":\"value\"}}]"),
        format!("[{first},{{\"id\":\"secret-input\"}}]"),
        "\"secret-input\"".to_owned(),
    ] {
        let error = serde_json::from_str::<LaunchProfiles>(&bad)
            .unwrap_err()
            .to_string();
        assert!(!error.contains("secret-input"), "{error}");
    }
    for bad in [
        "-1".to_owned(),
        (u64::from(LaunchProfileRevision::MAX.get()) + 1).to_string(),
        "\"secret-input\"".to_owned(),
    ] {
        let error = serde_json::from_str::<LaunchProfileRevision>(&bad)
            .unwrap_err()
            .to_string();
        assert!(!error.contains("secret-input"), "{error}");
    }
    assert_eq!(LaunchProfileRevision::MAX.next(), None);
    assert_eq!(
        LaunchProfileRevision::ZERO.next(),
        Some(LaunchProfileRevision::new(1))
    );
    let empty = LaunchProfilesSnapshot::new(LaunchProfileRevision::ZERO, LaunchProfiles::default());
    assert_eq!(
        serde_json::to_string(&empty).unwrap(),
        r#"{"revision":0,"profiles":[]}"#
    );
    roundtrip(&empty);
}

#[test]
fn optional_fields_are_cleared_by_omission_not_untyped_null() {
    let id = profile(0).id();
    for field in ["icon", "color", "notes"] {
        let body = format!(
            r#"{{"id":"{id}","name":"Valid","launch":{{"harness":"codex"}},"{field}":null}}"#
        );
        assert!(serde_json::from_str::<LaunchProfile>(&body).is_err());
    }
    assert!(
        serde_json::from_str::<LaunchSelection>(r#"{"harness":"codex","account_profile":null}"#)
            .is_err()
    );
    assert!(
        serde_json::from_str::<LaunchSelection>(r#"{"harness":"codex","options":null}"#).is_err()
    );
}

#[test]
fn counting_serialization_includes_escaping_and_stops_at_the_exact_byte_budget() {
    let quotes = serde_json::to_vec("").unwrap().len();
    let exact = "x".repeat(LaunchPayload::MAX_BYTES - quotes);
    assert!(LaunchPayload::check(&exact).is_ok());
    assert!(matches!(
        LaunchPayload::check(&format!("{exact}x")),
        Err(InvalidLaunchValue::Payload)
    ));
    let escaped = "\t".repeat(LaunchPayload::MAX_BYTES.checked_div(quotes).unwrap());
    assert!(escaped.len() < LaunchPayload::MAX_BYTES);
    assert!(serde_json::to_vec(&escaped).unwrap().len() > LaunchPayload::MAX_BYTES);
    assert!(matches!(
        LaunchPayload::check(&escaped),
        Err(InvalidLaunchValue::Payload)
    ));
}

#[test]
fn list_admission_reserves_the_maximum_revision_even_when_zero_would_fit() {
    let rows: Vec<_> = (0..LaunchProfiles::MAX_PROFILES)
        .map(|n| profile(n).with_notes(Some("x".parse().unwrap())))
        .collect();
    let base = LaunchProfilesSnapshot::new(
        LaunchProfileRevision::MAX,
        LaunchProfiles::try_from(rows.clone()).unwrap(),
    );
    let remaining = LaunchPayload::MAX_BYTES - serde_json::to_vec(&base).unwrap().len();
    let per_row = remaining.checked_div(rows.len()).unwrap();
    let residue = remaining % rows.len();
    let exact: Vec<_> = rows
        .into_iter()
        .enumerate()
        .map(|(n, p)| {
            let count = 1 + per_row + usize::from(n < residue);
            assert!(count <= LaunchProfileNotes::MAX_BYTES);
            p.with_notes(Some("x".repeat(count).parse().unwrap()))
        })
        .collect();
    let accepted = LaunchProfiles::try_from(exact.clone()).unwrap();
    let max = LaunchProfilesSnapshot::new(LaunchProfileRevision::MAX, accepted);
    assert_eq!(
        serde_json::to_vec(&max).unwrap().len(),
        LaunchPayload::MAX_BYTES
    );
    let mut one_more = exact;
    let first = one_more.first_mut().unwrap();
    let extra = format!("{}x", first.notes().unwrap().as_str());
    *first = first.clone().with_notes(Some(extra.parse().unwrap()));
    assert!(
        serde_json::to_vec(&UncheckedSnapshot {
            revision: LaunchProfileRevision::ZERO,
            profiles: &one_more
        })
        .unwrap()
        .len()
            <= LaunchPayload::MAX_BYTES
    );
    assert!(matches!(
        LaunchProfiles::try_from(one_more),
        Err(InvalidLaunchValue::Payload)
    ));
}

#[test]
fn pending_feature_is_registered_without_a_fabricated_protocol() {
    use crate::Request as _;
    assert_eq!(NAMED_LAUNCH_PROFILES, None);
    let call = ListLaunchProfiles {}.into_call();
    assert_eq!(call.required_protocol(), NAMED_LAUNCH_PROFILES);
    let encoded = serde_json::to_string(&call).unwrap();
    assert_eq!(serde_json::from_str::<crate::Call>(&encoded).unwrap(), call);
    assert_eq!(call.method().scope(), Some(crate::Scope::Read));
    assert!(call.method().forwardable());
}

#[cfg(feature = "ts")]
#[test]
fn typescript_optional_launch_fields_match_omission_without_admitting_null() {
    use ts_rs::TS as _;
    let config = ts_rs::Config::new();
    let selection = LaunchSelection::decl(&config);
    assert!(
        selection.contains("account_profile?: ProfileName"),
        "{selection}"
    );
    let declaration = LaunchProfile::decl(&config);
    for field in [
        "icon?: LaunchIconKey",
        "color?: LaunchColorKey",
        "notes?: LaunchProfileNotes",
    ] {
        assert!(declaration.contains(field), "{declaration}");
    }
    let outcome = ApplySessionOptionsResult::decl(&config);
    assert!(
        outcome.contains("notice?: ConfigurationNotice"),
        "{outcome}"
    );
    assert!(
        outcome.contains("failure?: LaunchConfigurationFailure"),
        "{outcome}"
    );
    for declaration in [&selection, &declaration, &outcome] {
        assert!(!declaration.contains("null"), "{declaration}");
    }
    let profile = profile(0);
    let encoded = serde_json::to_string(&profile).unwrap();
    for field in ["account_profile", "icon", "color", "notes"] {
        assert!(!encoded.contains(&format!("\"{field}\"")), "{encoded}");
    }
    let result = ApplySessionOptionsResult::new(LaunchChoices::default(), None, None).unwrap();
    assert_eq!(serde_json::to_string(&result).unwrap(), r#"{"applied":[]}"#);
    for json in [
        r#"{"applied":[],"notice":null}"#,
        r#"{"applied":[],"failure":null}"#,
    ] {
        assert!(serde_json::from_str::<ApplySessionOptionsResult>(json).is_err());
    }
}
