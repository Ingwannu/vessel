//! Portable preset cases from pinned schedule-cadence-options.test.ts. Vessel's
//! accepted interval exception preserves phase/seconds instead of converting it.
use super::*;
use crate::localization::{ENGLISH, Locale};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PresetId {
    Minute,
    Hour,
    Daily,
    Weekdays,
    Mondays,
}
impl From<PresetId> for Preset {
    fn from(value: PresetId) -> Self {
        match value {
            PresetId::Minute => Self::Minute,
            PresetId::Hour => Self::Hour,
            PresetId::Daily => Self::Daily,
            PresetId::Weekdays => Self::Weekdays,
            PresetId::Mondays => Self::Mondays,
        }
    }
}
#[derive(Deserialize)]
struct Case {
    id: String,
    preset: PresetId,
    expression: String,
    en: String,
    ko: String,
}
fn zone(name: &str) -> TimeZoneName {
    name.parse().unwrap()
}
fn every(seconds: u32) -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(seconds).unwrap(),
        anchor: Some("2026-11-13T00:00:00Z".parse().unwrap()),
    }
}
#[test]
fn reference_presets_keep_exact_order_expression_and_bilingual_words() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/schedules/editor-cadence.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), Preset::ALL.len());
    for (case, expected) in cases.into_iter().zip(Preset::ALL) {
        let preset = Preset::from(case.preset);
        assert_eq!(preset, expected, "{}", case.id);
        assert_eq!(preset.expression(), case.expression, "{}", case.id);
        assert_eq!(preset.label(en), case.en, "{}", case.id);
        assert_eq!(preset.label(ko), case.ko, "{}", case.id);
        let mut draft = Draft::open(every(3600), zone("Europe/Madrid")).unwrap();
        draft.preset(preset);
        assert_eq!(draft.choice(), Choice::Preset(preset));
        assert_eq!(
            draft.checked().unwrap(),
            &super::super::super::cadence::cron(&case.expression, zone("Europe/Madrid")).unwrap()
        );
    }
}
#[test]
fn opening_an_interval_preserves_seconds_anchor_and_lossless_units() {
    for (seconds, count, unit) in [
        (90, "90", IntervalUnit::Seconds),
        (7200, "2", IntervalUnit::Hours),
        (86400, "1", IntervalUnit::Days),
    ] {
        let original = every(seconds);
        let draft = Draft::open(original.clone(), zone("Asia/Seoul")).unwrap();
        assert_eq!(draft.checked().unwrap(), &original);
        assert_eq!(draft.choice(), Choice::Interval);
        assert_eq!(draft.text(), count);
        assert_eq!(draft.unit(), Some(unit));
        assert_eq!(
            draft.units().contains(&IntervalUnit::Seconds),
            seconds == 90
        );
    }
}
#[test]
fn cron_edits_keep_the_opened_zone_and_interval_switches_use_the_viewer_zone() {
    // schedule-cadence-policy.test.ts: existing cron zone, explicit UTC legacy
    // boundary, and interval-to-cron device zone. The wire requires a zone;
    // it cannot represent the reference's absent-zone legacy cron variant.
    for (original, viewer, expected) in [
        (
            super::super::super::cadence::cron("0 9 * * *", zone("America/New_York")).unwrap(),
            "America/Los_Angeles",
            "America/New_York",
        ),
        (
            super::super::super::cadence::cron("0 9 * * *", TimeZoneName::utc()).unwrap(),
            "America/Los_Angeles",
            "UTC",
        ),
        (every(3600), "America/Los_Angeles", "America/Los_Angeles"),
    ] {
        let mut draft = Draft::open(original, zone(viewer)).unwrap();
        draft.cron("30 9 * * *".into());
        assert_eq!(draft.zone(), &zone(expected));
        assert_eq!(
            draft.checked().unwrap(),
            &super::super::super::cadence::cron("30 9 * * *", zone(expected)).unwrap()
        );
    }
}
#[test]
fn custom_choice_invalid_input_and_zone_changes_never_guess_a_rule() {
    let mut draft = Draft::open(every(90), zone("Asia/Seoul")).unwrap();
    draft.preset(Preset::Hour);
    draft.custom();
    assert_eq!(draft.choice(), Choice::Custom);
    assert_eq!(draft.text(), "0 * * * *");
    draft.cron("99 9 * * *".into());
    assert!(draft.checked().is_err());
    draft.set_zone(zone("America/New_York"));
    assert!(draft.checked().is_err());
    draft.cron("0 9 * * *".into());
    assert_eq!(draft.zone(), &zone("America/New_York"));
    draft.interval("1".into(), IntervalUnit::Hours);
    assert_eq!(
        draft.checked().unwrap(),
        &Cadence::Every {
            seconds: EverySeconds::try_from(3600).unwrap(),
            anchor: None
        }
    );
    draft.preset(Preset::Daily);
    assert_eq!(draft.zone(), &zone("America/New_York"));
    assert!(matches!(
        Draft::open(Cadence::Unknown, zone("UTC")),
        Err(EditError::Unknown)
    ));
}

#[test]
fn a_new_rule_uses_the_source_hour_boundary_in_the_explicit_viewer_zone() {
    for name in ["Asia/Seoul", "America/New_York"] {
        let cadence = initial(zone(name)).unwrap();
        let Cadence::Cron {
            expr,
            zone: selected,
        } = &cadence
        else {
            panic!("wall-clock default")
        };
        assert_eq!(expr.as_str(), "0 * * * *");
        assert_eq!(selected.as_str(), name);
        let draft = Draft::open(cadence, zone("UTC")).unwrap();
        assert_eq!(draft.choice(), Choice::Preset(Preset::Hour));
        assert_eq!(draft.zone().as_str(), name);
    }
}
