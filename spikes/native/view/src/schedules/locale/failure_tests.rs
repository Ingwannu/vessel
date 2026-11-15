//! Every local reason and wire code chooses a complete catalog phrase.
use super::*;
use crate::schedules::load::Failure;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Input {
    Older,
    NotEnabled,
    Offline,
    Busy,
    Unexpected,
    Unconfirmed,
    Exhausted,
    NoTransport,
    Node { code: vessel_proto::ErrorCode },
}
impl From<Input> for Failure {
    fn from(value: Input) -> Self {
        match value {
            Input::Older => Self::Older,
            Input::NotEnabled => Self::NotEnabled,
            Input::Offline => Self::Offline,
            Input::Busy => Self::Busy,
            Input::Unexpected => Self::Unexpected,
            Input::Unconfirmed => Self::Unconfirmed,
            Input::Exhausted => Self::Exhausted,
            Input::NoTransport => Self::NoTransport,
            Input::Node { code } => {
                Self::Node(vessel_proto::WireError::new(code, "diagnostic-only"))
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ExpectedVariant {
    Error,
    Info,
    Warning,
}
impl From<ExpectedVariant> for crate::alert::Variant {
    fn from(value: ExpectedVariant) -> Self {
        match value {
            ExpectedVariant::Error => Self::Error,
            ExpectedVariant::Info => Self::Info,
            ExpectedVariant::Warning => Self::Warning,
        }
    }
}
#[derive(Deserialize)]
struct Case {
    id: String,
    input: Input,
    en: String,
    ko: String,
    retry: bool,
    explains: bool,
    variant: ExpectedVariant,
}
#[test]
fn every_failure_has_catalog_words_and_input_policy_in_both_languages() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/failures.json"
    ))
    .unwrap();
    let en = Translator::get(crate::localization::ENGLISH).unwrap();
    let ko = Translator::get(crate::localization::Locale::from_tag("ko").unwrap()).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for case in cases {
        assert!(ids.insert(case.id.clone()));
        let value = Failure::from(case.input);
        assert_eq!(failure(&value, en), case.en, "{}", case.id);
        assert_eq!(failure(&value, ko), case.ko, "{}", case.id);
        assert_eq!(value.retryable(), case.retry, "{}", case.id);
        assert_eq!(value.needs_explanation(), case.explains, "{}", case.id);
        assert_eq!(value.variant(), case.variant.into(), "{}", case.id);
    }
    assert_eq!(ids.len(), 19);
}
