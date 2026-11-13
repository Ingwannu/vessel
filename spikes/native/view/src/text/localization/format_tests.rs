//! Whole formatted values exercise the folder-owned rules through the public API.
use super::*;

#[test]
fn grouped_counts_and_dates_use_the_language_inside_messages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap();
    assert_eq!(
        en.examples().grouped(Number::count(1_234_567)).unwrap(),
        "\u{2068}1,234,567\u{2069} sessions in total."
    );
    assert_eq!(
        ko.examples().grouped(Number::count(1_234_567)).unwrap(),
        "세션 총 \u{2068}1,234,567\u{2069}개입니다."
    );
    let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 13).map(Date::Day);
    assert!(date.is_some());
    if let Some(date) = date {
        assert_eq!(
            en.examples().date(&date).unwrap(),
            "Paired on \u{2068}Fri, Nov 13, 2026\u{2069}."
        );
        assert_eq!(
            ko.examples().date(&date).unwrap(),
            "페어링 날짜: \u{2068}2026년 11월 13일 금요일\u{2069}"
        );
    }
}

#[test]
fn local_time_and_timestamp_use_folder_patterns_and_english_fallback() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let ja = Translator::get(Locale::from_tag("ja").unwrap()).unwrap();
    let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();
    let time = chrono::NaiveTime::from_hms_opt(15, 4, 0).unwrap();
    let clock = Date::Time {
        time,
        zone: "UTC".to_owned(),
    };
    assert_eq!(
        en.examples().time(&clock).unwrap(),
        "Paired at \u{2068}3:04 PM UTC\u{2069}."
    );
    assert_eq!(
        ko.examples().time(&clock).unwrap(),
        "페어링 시각: \u{2068}15:04 UTC\u{2069}"
    );
    assert_eq!(
        ja.examples().time(&clock).unwrap(),
        "Paired at \u{2068}3:04 PM UTC\u{2069}."
    );
    let stamp = Date::DateTime {
        date: day,
        time,
        zone: "UTC".to_owned(),
    };
    assert_eq!(
        en.examples().timestamp(&stamp).unwrap(),
        "Paired on \u{2068}Fri, Nov 13, 2026 at 3:04 PM UTC\u{2069}."
    );
    assert_eq!(
        ko.examples().timestamp(&stamp).unwrap(),
        "페어링 일시: \u{2068}2026년 11월 13일 금요일 15:04 UTC\u{2069}"
    );
    for (count, english, korean) in [
        (1, "in \u{2068}1\u{2069} minute", "\u{2068}1\u{2069}분 후"),
        (9, "in \u{2068}9\u{2069} minutes", "\u{2068}9\u{2069}분 후"),
    ] {
        assert_eq!(
            en.examples().in_minutes(Number::count(count)).unwrap(),
            english
        );
        assert_eq!(
            ko.examples().in_minutes(Number::count(count)).unwrap(),
            korean
        );
    }
}

#[test]
fn decimal_arguments_are_bounded_and_selectors_keep_numeric_values() {
    assert!(Number::decimal(f64::NAN).is_err());
    assert!(Number::decimal(f64::INFINITY).is_err());
    assert!(Number::decimal(4_294_967_296.0).is_err());
    assert!(Number::decimal(0.000_000_000_1).is_err());
    let en = Translator::get(ENGLISH).unwrap();
    assert_eq!(
        en.examples()
            .grouped(Number::decimal(-1234.5).unwrap())
            .unwrap(),
        "\u{2068}-1,234.5\u{2069} sessions in total."
    );
    assert_eq!(
        en.examples()
            .sessions(Number::decimal(-1.0).unwrap())
            .unwrap(),
        "\u{2068}-1\u{2069} live session"
    );
    let fixture = runtime::fixture(
        "en",
        "value = { NUMBER(1234.567, minimumFractionDigits: 2) }
rounded = { NUMBER(1234.567, maximumFractionDigits: 2) }\nplural = { NUMBER(1.2, maximumFractionDigits: 0) ->\n [one] one\n *[other] other\n}",
    )
    .unwrap();
    assert_eq!(
        resolve(&fixture, &fixture, "value", None).unwrap(),
        "1,234.567"
    );
    assert_eq!(
        resolve(&fixture, &fixture, "rounded", None).unwrap(),
        "1,234.57"
    );
    assert_eq!(resolve(&fixture, &fixture, "plural", None).unwrap(), "one");
}
