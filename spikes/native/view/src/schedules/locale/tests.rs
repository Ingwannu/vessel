//! Whole phrases, including generated numeric/date argument propagation.
use super::*;
use crate::localization::{Date, ENGLISH, Locale, Number};

fn catalogs() -> [&'static Translator; 2] {
    [
        Translator::get(ENGLISH).unwrap(),
        Translator::get(Locale::from_tag("ko").unwrap()).unwrap(),
    ]
}
fn plain(text: &str) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

// Portable kebab-case inputs; a future page renderer can run the same cases.
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BucketCase {
    Active,
    Ended,
    Unknown,
}
impl From<BucketCase> for Bucket {
    fn from(value: BucketCase) -> Self {
        match value {
            BucketCase::Active => Self::Active,
            BucketCase::Ended => Self::Ended,
            BucketCase::Unknown => Self::Unknown,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ActionCase {
    Edit,
    Pause,
    Resume,
    RunNow,
    Delete,
}
impl From<ActionCase> for Action {
    fn from(value: ActionCase) -> Self {
        match value {
            ActionCase::Edit => Self::Edit,
            ActionCase::Pause => Self::Pause,
            ActionCase::Resume => Self::Resume,
            ActionCase::RunNow => Self::RunNow,
            ActionCase::Delete => Self::Delete,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Input {
    Empty { bucket: BucketCase },
    Pending { action: ActionCase },
}
#[derive(serde::Deserialize)]
struct Case {
    id: String,
    input: Input,
    en: Option<String>,
    ko: Option<String>,
    hint_en: Option<String>,
    hint_ko: Option<String>,
}
#[test]
fn portable_empty_and_pending_choices_have_whole_words_in_both_languages() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/list-words.json"
    ))
    .unwrap();
    let [en, ko] = catalogs();
    let mut ids = std::collections::BTreeSet::new();
    for case in cases {
        assert!(ids.insert(case.id.clone()));
        match case.input {
            Input::Empty { bucket } => {
                let bucket = Bucket::from(bucket);
                assert_eq!(
                    Some(empty_title(bucket, en)),
                    case.en.as_deref(),
                    "{}",
                    case.id
                );
                assert_eq!(
                    Some(empty_title(bucket, ko)),
                    case.ko.as_deref(),
                    "{}",
                    case.id
                );
                assert_eq!(
                    empty_hint(bucket, en),
                    case.hint_en.as_deref(),
                    "{}",
                    case.id
                );
                assert_eq!(
                    empty_hint(bucket, ko),
                    case.hint_ko.as_deref(),
                    "{}",
                    case.id
                );
            }
            Input::Pending { action } => {
                let action = Action::from(action);
                assert_eq!(pending(action, en), case.en.as_deref(), "{}", case.id);
                assert_eq!(pending(action, ko), case.ko.as_deref(), "{}", case.id);
            }
        }
    }
    assert_eq!(ids.len(), 8);
}

#[test]
fn generated_reference_arguments_keep_numbers_and_dates_typed() {
    let [en, ko] = catalogs();
    assert_eq!(
        plain(
            &en.schedules()
                .age_date(Number::count(15), Number::count(1))
                .unwrap()
        ),
        "Jan 15"
    );
    assert_eq!(
        plain(
            &ko.schedules()
                .age_date(Number::count(15), Number::count(1))
                .unwrap()
        ),
        "1월 15일"
    );
    let at = Date::DateTime {
        date: chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
        time: chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        zone: "Asia/Seoul".into(),
    };
    let next = en
        .schedules()
        .next_known_rule(&at, "in 2m", "America/New_York")
        .unwrap();
    assert_eq!(
        plain(&next),
        "Next run in 2m · Your time: Mon, Nov 16, 2026 at 9:00 AM Asia/Seoul · Rule’s zone: America/New_York"
    );
    let next = ko
        .schedules()
        .next_known_rule(&at, "2분 후", "America/New_York")
        .unwrap();
    assert_eq!(
        plain(&next),
        "다음 실행: 2분 후 · 내 시각: 2026년 11월 16일 월요일 09:00 Asia/Seoul · 예약 시간대: America/New_York"
    );
}
