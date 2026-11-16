//! Complete sentences, not substring assertions: the catalog is an app API.

use super::*;

#[test]
fn initialized_plain_values_are_borrowed_from_one_cache() {
    for locale in LANGUAGES.iter().copied().chain([PSEUDO]) {
        let first = Translator::get(locale).unwrap();
        let again = Translator::get(locale).unwrap();
        assert!(std::ptr::eq(first, again));
        let value = first.meta().language();
        for _ in 0..10_000 {
            assert!(std::ptr::eq(value, first.meta().language()));
        }
    }
}

#[test]
fn english_and_korean_whole_sentences() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap();
    assert_eq!(en.meta().language(), "Language");
    assert_eq!(ko.meta().language(), "언어");
    assert_eq!(
        en.examples().welcome("민수").unwrap(),
        "Welcome, \u{2068}민수\u{2069}."
    );
    assert_eq!(
        ko.examples().welcome("민수").unwrap(),
        "\u{2068}민수\u{2069}님, 환영합니다."
    );
    for (count, english, korean) in [
        (
            0,
            "\u{2068}0\u{2069} live sessions",
            "실행 중인 세션 \u{2068}0\u{2069}개",
        ),
        (
            1,
            "\u{2068}1\u{2069} live session",
            "실행 중인 세션 \u{2068}1\u{2069}개",
        ),
        (
            2,
            "\u{2068}2\u{2069} live sessions",
            "실행 중인 세션 \u{2068}2\u{2069}개",
        ),
    ] {
        assert_eq!(
            en.examples().sessions(Number::count(count)).unwrap(),
            english
        );
        assert_eq!(
            ko.examples().sessions(Number::count(count)).unwrap(),
            korean
        );
    }
    for (state, english, korean) in [
        (
            ExamplesStateState::Connecting,
            "Connecting to your machine.",
            "머신에 연결하고 있습니다.",
        ),
        (
            ExamplesStateState::Ready,
            "Your machine is ready.",
            "머신이 준비되었습니다.",
        ),
    ] {
        assert_eq!(en.examples().state(state).unwrap(), english);
        assert_eq!(ko.examples().state(state).unwrap(), korean);
    }
    assert_eq!(
        en.examples()
            .ago(Number::count(1), ExamplesAgoUnit::Minute)
            .unwrap(),
        "\u{2068}1\u{2069} minute ago"
    );
    assert_eq!(
        en.examples()
            .ago(Number::count(2), ExamplesAgoUnit::Hour)
            .unwrap(),
        "\u{2068}2\u{2069} hours ago"
    );
    assert_eq!(
        ko.examples()
            .ago(Number::count(1), ExamplesAgoUnit::Minute)
            .unwrap(),
        "\u{2068}1\u{2069}분 전"
    );
    assert_eq!(
        ko.examples()
            .ago(Number::count(2), ExamplesAgoUnit::Hour)
            .unwrap(),
        "\u{2068}2\u{2069}시간 전"
    );
    assert_eq!(
        en.examples().remove("서버").unwrap(),
        "Remove \u{2068}서버\u{2069}."
    );
    assert_eq!(
        ko.examples().remove("서버").unwrap(),
        "\u{2068}서버\u{2069}\u{2068}를\u{2069} 삭제하세요."
    );
    assert_eq!(
        ko.examples().remove("서울").unwrap(),
        "\u{2068}서울\u{2069}\u{2068}을\u{2069} 삭제하세요."
    );
}

#[test]
fn plural_rules_belong_to_the_translated_language() {
    let source = "count = { $count ->\n [one] one\n [few] few\n [many] many\n *[other] other\n}";
    let russian = runtime::fixture("ru", source).unwrap();
    let english = runtime::fixture("en", source).unwrap();
    for (count, russian_form, english_form) in [
        (1, "one", "one"),
        (2, "few", "other"),
        (5, "many", "other"),
        (21, "one", "other"),
    ] {
        let mut args = FluentArgs::new();
        args.set("count", count);
        assert_eq!(
            resolve(&russian, &english, "count", Some(&args)).unwrap(),
            russian_form
        );
        assert_eq!(
            resolve(&english, &english, "count", Some(&args)).unwrap(),
            english_form
        );
    }
}

#[test]
fn a_partial_fixture_uses_its_messages_and_english_plural_fallback() {
    let partial = fixture::bundle("ja");
    let english = fixture::bundle("en");
    assert_eq!(
        resolve(&partial, &english, "meta-name", None).unwrap(),
        "日本語"
    );
    assert_eq!(
        resolve(&partial, &english, "meta-language", None).unwrap(),
        "言語"
    );
    let mut args = FluentArgs::new();
    args.set("name", "서울");
    assert_eq!(
        resolve(&partial, &english, "meta-welcome", Some(&args)).unwrap(),
        "Welcome, \u{2068}서울\u{2069}."
    );
    for (count, expected) in [(1, "one session"), (2, "many sessions")] {
        let mut args = FluentArgs::new();
        args.set("count", count);
        assert_eq!(
            resolve(&partial, &english, "meta-sessions", Some(&args)).unwrap(),
            expected
        );
    }
}

#[test]
fn released_catalogs_are_complete() {
    assert!(LANGUAGES.iter().all(|locale| locale.is_complete()));
}

#[test]
fn broken_messages_retain_defensive_english_fallback() {
    let partial = runtime::fixture("ru", "hello = Привет\nbroken = { $unknown }").unwrap();
    let english = runtime::fixture(
        "en",
        "hello = Hello\nbroken = Still works\nmissing = English fallback\ncount = { $count ->\n [one] one session\n *[other] many sessions\n}",
    ).unwrap();
    assert_eq!(
        resolve(&partial, &english, "hello", None).unwrap(),
        "Привет"
    );
    assert_eq!(
        resolve(&partial, &english, "missing", None).unwrap(),
        "English fallback"
    );
    assert_eq!(
        resolve(&partial, &english, "broken", None).unwrap(),
        "Still works"
    );
    let mut args = FluentArgs::new();
    args.set("count", 21);
    assert_eq!(
        resolve(&partial, &english, "count", Some(&args)).unwrap(),
        "many sessions"
    );
    assert!(resolve(&partial, &english, "absent", None).is_err());
}

#[test]
fn particles_cover_hangul_digits_latin_and_empty_values() {
    let rows = [
        ("서울", "을", "로"),
        ("\u{1109}\u{1165}\u{110b}\u{116e}\u{11af}", "을", "로"),
        ("\u{1107}\u{116e}\u{1109}\u{1161}\u{11ab}", "을", "으로"),
        ("부산", "을", "으로"),
        ("서버", "를", "로"),
        ("0", "을", "으로"),
        ("1", "을", "로"),
        ("2", "를", "로"),
        ("3", "을", "으로"),
        ("4", "를", "로"),
        ("5", "를", "로"),
        ("6", "을", "으로"),
        ("7", "을", "로"),
        ("8", "을", "로"),
        ("9", "를", "로"),
        ("VM", "을", "으로"),
        ("nodeN", "을", "으로"),
        ("SQL", "을", "로"),
        ("R", "을", "로"),
        ("server", "을", "로"),
        ("nodeX", "를", "로"),
        ("nodeF", "를", "로"),
        ("nodeR", "을", "로"),
        ("“서울” ", "을", "로"),
        ("", "를", "로"),
        ("...", "를", "로"),
    ];
    for (value, object, direction) in rows {
        assert_eq!(Particle::Object.after(value), object, "{value}");
        assert_eq!(Particle::Direction.after(value), direction, "{value}");
    }
    for (value, subject, topic, with) in [("서버", "가", "는", "와"), ("부산", "이", "은", "과")]
    {
        assert_eq!(Particle::Subject.after(value), subject);
        assert_eq!(Particle::Topic.after(value), topic);
        assert_eq!(Particle::With.after(value), with);
    }
}

#[test]
fn pseudo_expands_text_but_keeps_values_and_selectors() {
    let pseudo = Translator::get(PSEUDO).unwrap();
    assert_eq!(pseudo.meta().language(), "Ŀaaƞɠuuaaɠee");
    assert_eq!(
        pseudo.examples().welcome("서울-12").unwrap(),
        "Ẇeeŀƈooḿee, \u{2068}서울-12\u{2069}."
    );
    assert_eq!(
        pseudo.examples().sessions(Number::count(1)).unwrap(),
        "\u{2068}1\u{2069} ŀiṽee şeeşşiooƞ"
    );
    assert_eq!(
        pseudo.examples().sessions(Number::count(2)).unwrap(),
        "\u{2068}2\u{2069} ŀiṽee şeeşşiooƞş"
    );
    assert!(LANGUAGES.iter().all(|locale| *locale != PSEUDO));
}

#[test]
fn locale_selection_uses_override_preference_system_exact_base_then_english() {
    let ko = Locale::from_tag("ko")
        .ok_or(InitError::MissingLocale)
        .unwrap();
    for tag in ["ko", "ko-KR", "ko-KP", "ko_KR.UTF-8", "KO-kr"] {
        assert_eq!(
            resolve_language(LanguagePreference::System, Some(tag), None),
            ko
        );
    }
    assert_eq!(
        resolve_language(LanguagePreference::from_tag("ko"), Some("en-US"), None),
        ko
    );
    assert_eq!(
        resolve_language(LanguagePreference::from_tag("ko"), Some("ko"), Some("en")),
        ENGLISH
    );
    assert_eq!(
        resolve_language(
            LanguagePreference::from_tag("ko"),
            Some("ko"),
            Some("unknown")
        ),
        ENGLISH
    );
    for tag in [Some("de-DE"), Some("en-US"), None] {
        assert_eq!(
            resolve_language(LanguagePreference::System, tag, None),
            ENGLISH
        );
    }
    assert_eq!(
        resolve_language(LanguagePreference::System, Some("en-XA"), None),
        ENGLISH
    );
    assert_eq!(
        resolve_language(LanguagePreference::System, None, Some("en-XA")),
        PSEUDO
    );
    assert_eq!(Locale::from_tag("en-XA"), None);
    assert_eq!(ko.native_name(), "한국어");
    assert!(ko.is_complete());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn plain_access_has_zero_allocations_after_initialization() {
    for locale in LANGUAGES.iter().copied().chain([PSEUDO]) {
        let translator = Translator::get(locale).unwrap();
        let allocations = allocation_counter::measure(|| {
            for _ in 0..10_000 {
                let _name = std::hint::black_box(translator.meta().name());
                let _language = std::hint::black_box(translator.meta().language());
                let _system = std::hint::black_box(translator.meta().system());
                let _language_note = std::hint::black_box(translator.meta().language_note());
                for delivery in [
                    crate::prefs::TurnDelivery::Queue,
                    crate::prefs::TurnDelivery::Steer,
                    crate::prefs::TurnDelivery::Interrupt,
                ] {
                    let _name = std::hint::black_box(translator.general().delivery(delivery));
                    let _note = std::hint::black_box(translator.general().delivery_note(delivery));
                }
                let _pair_title = std::hint::black_box(translator.pairing().title());
                let _pair_warning = std::hint::black_box(translator.pairing().security_warning());
                let _confirm = std::hint::black_box(translator.pairing().confirm_description());
                let _software = std::hint::black_box(translator.about().software());
                for part in crate::about::BUNDLED {
                    let _role = std::hint::black_box(translator.about().role(part.role));
                }
                let _cached = std::hint::black_box(Translator::get(locale));
            }
        });
        assert_eq!(allocations.count_total, 0);
        assert_eq!(allocations.bytes_total, 0);
    }
}

#[test]
fn unknown_or_checking_only_stored_tags_become_system() {
    assert_eq!(
        LanguagePreference::from_tag("unknown"),
        LanguagePreference::System
    );
    assert_eq!(
        LanguagePreference::from_tag("en-XA"),
        LanguagePreference::System
    );
    assert_eq!(LanguagePreference::from_tag("system").tag(), "system");
    assert_eq!(LanguagePreference::from_tag("ko").tag(), "ko");
}

#[test]
fn stored_preferences_are_tags_and_unknown_values_fall_back_to_system() {
    for (json, canonical) in [
        ("\"system\"", "system"),
        ("\"ko\"", "ko"),
        ("\"en\"", "en"),
        ("\"removed-language\"", "system"),
        ("\"en-XA\"", "system"),
    ] {
        let preference: LanguagePreference = serde_json::from_str(json).unwrap();
        assert_eq!(preference.tag(), canonical);
        assert_eq!(
            serde_json::to_string(&preference).unwrap(),
            format!("\"{canonical}\"")
        );
    }
}
