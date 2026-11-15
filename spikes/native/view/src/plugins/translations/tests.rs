use super::*;
use crate::localization::{LanguagePreference, Locale, resolve_language};
use std::error::Error;
use std::fmt::Write as _;

fn source(locale: &str, text: &str) -> Resource {
    Resource::new(locale.parse().expect("locale"), text).expect("bounded source")
}

fn formatter(english: &str, korean: Option<&str>, tag: &str) -> Formatter {
    let mut sources = vec![source("en", english)];
    if let Some(ko) = korean {
        sources.push(source("ko", ko));
    }
    Resources::new(sources)
        .expect("namespace")
        .prepare(Locale::from_tag(tag).expect("locale"))
        .expect("formatter")
}

fn key(key: &str) -> MessageKey {
    key.parse().expect("message key")
}
fn text(text: &str) -> Argument {
    Argument::Text(text.parse().expect("bounded text"))
}

#[test]
fn partial_locale_falls_back_per_message_with_english_plural_rules() {
    let english =
        "title = Plugin\ncount = { $n ->\n    [one] One item\n   *[other] Many items\n}\n";
    let ko = formatter(english, Some("title = 플러그인\n"), "ko");
    assert_eq!(
        ko.caption(&key("title"), &Arguments::default())
            .expect("KO")
            .as_str(),
        "플러그인"
    );
    let one = Arguments::new(vec![("n", Argument::Number(Number::count(1)))]).expect("count");
    assert_eq!(
        ko.caption(&key("count"), &one)
            .expect("English one")
            .as_str(),
        "One item"
    );
}

#[test]
fn namespaces_are_independent_and_language_reprepare_does_not_mutate_core() {
    let first = Resources::new(vec![
        source("en", "name = First\n"),
        source("ko", "name = 첫째\n"),
    ])
    .expect("first");
    let second = formatter("name = Second\n", None, "ko");
    let en = first
        .prepare(Locale::from_tag("en").expect("en"))
        .expect("en");
    let ko = first
        .prepare(Locale::from_tag("ko").expect("ko"))
        .expect("ko");
    assert_eq!(
        en.theme_name(&key("name"), &Arguments::default())
            .expect("first")
            .as_str(),
        "First"
    );
    assert_eq!(
        ko.theme_name(&key("name"), &Arguments::default())
            .expect("KO")
            .as_str(),
        "첫째"
    );
    assert_eq!(
        second
            .theme_name(&key("name"), &Arguments::default())
            .expect("other install")
            .as_str(),
        "Second"
    );
    drop(first);
    assert_eq!(
        en.theme_name(&key("name"), &Arguments::default())
            .expect("owned resource")
            .as_str(),
        "First"
    );
}

#[test]
fn missing_english_duplicate_canonical_locale_keys_and_attributes_refuse() {
    assert!(matches!(
        Resources::new(vec![source("ko", "name = 이름\n")]),
        Err(Problem::MissingEnglish)
    ));
    assert!(matches!(
        Resources::new(vec![
            source("en", "name = One\n"),
            source("EN", "name = Two\n")
        ]),
        Err(Problem::Duplicate)
    ));
    assert!(matches!(
        Resources::new(vec![source("en", "name = One\nname = Two\n")]),
        Err(Problem::Duplicate)
    ));
    assert!(matches!(
        Resources::new(vec![source("en", "name = One\n    .title = Two\n")]),
        Err(Problem::Attributes)
    ));
    assert!(
        Resources::new(vec![
            source("en", "name = One\n"),
            source("ko", "unknown = 알 수 없음\n")
        ])
        .is_err()
    );
}

#[test]
fn shared_ast_checks_reject_bad_function_operands_terms_and_raw_date_reads() {
    for english in [
        "bad = { NUMBER(\"text\") }\n",
        "-term = { NUMBER($n) }\nbad = { -term(n: \"text\") }\n",
        "bad = { DATETIME($date) } { $date }\n",
        "bad = { missing }\n",
    ] {
        assert!(
            Resources::new(vec![source("en", english)]).is_err(),
            "invalid contract"
        );
    }
    assert!(
        Resources::new(vec![
            source("en", "name = { NUMBER($n) }\n"),
            source("ko", "name = { DATETIME($n) }\n")
        ])
        .is_err()
    );
}

#[test]
fn typed_numbers_dates_arguments_and_per_target_limits_are_checked_before_publication() {
    let prepared = formatter(
        "amount = { NUMBER($n) }\nwhen = { DATETIME($date) }\nraw = { $date }\nmultiline = First\n    Second\n",
        None,
        "en",
    );
    let number = Arguments::new(vec![("n", Argument::Number(Number::count(3)))]).expect("number");
    assert!(
        prepared
            .caption(&key("amount"), &number)
            .expect("number")
            .as_str()
            .contains('3')
    );
    assert!(
        prepared
            .caption(
                &key("amount"),
                &Arguments::new(vec![("n", text("three"))]).expect("text")
            )
            .is_err()
    );
    let day = Date::Day(chrono::NaiveDate::from_ymd_opt(2026, 11, 15).expect("date"));
    let date = Arguments::new(vec![("date", Argument::Date(day))]).expect("date");
    let formatted = prepared.caption(&key("when"), &date).expect("date");
    assert!(!formatted.as_str().contains("Day"));
    assert!(prepared.caption(&key("raw"), &date).is_err());
    assert!(
        prepared
            .content(&key("multiline"), &Arguments::default())
            .expect("paragraph")
            .as_str()
            .contains('\n')
    );
    assert!(
        prepared
            .caption(&key("multiline"), &Arguments::default())
            .is_err()
    );
}

#[test]
fn invalid_translated_target_falls_back_and_missing_or_extra_arguments_refuse() {
    let long = format!("name = {}\n", "x".repeat(121));
    let prepared = formatter("name = Short\ngreet = Hello { $who }\n", Some(&long), "ko");
    assert_eq!(
        prepared
            .caption(&key("name"), &Arguments::default())
            .expect("fallback")
            .as_str(),
        "Short"
    );
    assert!(
        prepared
            .caption(&key("greet"), &Arguments::default())
            .is_err()
    );
    assert!(Arguments::new(vec![("who", text("A")), ("who", text("B"))]).is_err());
    let extra = Arguments::new(vec![("unused", text("value"))]).expect("args");
    assert!(prepared.caption(&key("name"), &extra).is_err());
    let injection = Arguments::new(vec![("who", text("{ NUMBER(\"text\") }"))]).expect("literal");
    assert!(
        prepared
            .caption(&key("greet"), &injection)
            .expect("not source")
            .as_str()
            .contains("NUMBER")
    );
}

#[test]
fn resource_parser_and_contract_errors_seal_every_diagnostic_source_level() {
    for english in ["PRIVATE-CANARY = {\n", "name = { PRIVATE-CANARY }\n"] {
        let error = Resources::new(vec![source("en", english)]).expect_err("invalid");
        let mut cause: &dyn Error = &error;
        loop {
            assert!(!format!("{cause:?} {cause}").contains("PRIVATE-CANARY"));
            match cause.source() {
                Some(next) => cause = next,
                None => break,
            }
        }
    }
}

#[test]
fn budgets_precede_parser_and_reject_whole_resources_without_truncation() {
    assert!(matches!(
        Resource::new(
            "en".parse().expect("locale"),
            &"x".repeat(MAX_SOURCE_BYTES.saturating_add(1))
        ),
        Err(Problem::Budget(Budget::Source))
    ));
    let nested = "{".repeat(MAX_SYNTAX_OPENERS.saturating_add(1));
    assert!(matches!(
        Resource::new("en".parse().expect("locale"), &nested),
        Err(Problem::Budget(Budget::Syntax))
    ));
    assert!(matches!(
        Resources::new(
            (0..MAX_LOCALES.saturating_add(1))
                .map(|_| source("en", "name = Text\n"))
                .collect()
        ),
        Err(Problem::Budget(Budget::Locales))
    ));
    assert!(MessageKey::from_str(&"a".repeat(MAX_NAME_BYTES.saturating_add(1))).is_err());
}

#[test]
fn pseudo_checking_uses_shared_factory_and_output_growth_is_bounded() {
    let resources = Resources::new(vec![source("en", "name = Plugin theme\n")]).expect("namespace");
    let locale = resolve_language(LanguagePreference::System, None, Some("en-XA"));
    let prepared = resources.prepare(locale).expect("pseudo");
    assert_ne!(
        prepared
            .caption(&key("name"), &Arguments::default())
            .expect("pseudo")
            .as_str(),
        "Plugin theme"
    );
    let repeated = format!("text = {}\n", "{ $value }".repeat(16));
    let prepared = formatter(&repeated, None, "en");
    let args = Arguments::new(vec![("value", text(&"x".repeat(4096)))]).expect("bounded arg");
    assert!(
        matches!(prepared.content(&key("text"), &args), Err(Problem::Fallback { english, .. }) if matches!(*english, Problem::Budget(Budget::Output)))
    );
}

#[test]
fn total_key_argument_and_date_zone_bounds_are_whole_result_refusals() {
    let mut lots = String::new();
    for i in 0..MAX_KEYS.saturating_add(1) {
        writeln!(lots, "key-{i} = Text").expect("string writer");
    }
    assert!(matches!(
        Resources::new(vec![source("en", &lots)]),
        Err(Problem::Budget(Budget::Keys))
    ));
    let large = format!(
        "name = {}\n",
        "x".repeat(MAX_SOURCE_BYTES.saturating_sub(20))
    );
    let sources = ["en", "ko", "ja", "fr", "de", "it", "es", "pt", "ru"]
        .into_iter()
        .map(|locale| source(locale, &large))
        .collect();
    assert!(matches!(
        Resources::new(sources),
        Err(Problem::Budget(Budget::Total))
    ));
    let names = (0..MAX_ARGUMENTS.saturating_add(1))
        .map(|i| format!("name-{i}"))
        .collect::<Vec<_>>();
    let arguments = names
        .iter()
        .map(|name| (name.as_str(), text("value")))
        .collect();
    assert!(matches!(
        Arguments::new(arguments),
        Err(Problem::Budget(Budget::Arguments))
    ));
    let time = chrono::NaiveTime::from_hms_opt(10, 0, 0).expect("time");
    let invalid_zone = Date::Time {
        time,
        zone: "x".repeat(MAX_ZONE_BYTES.saturating_add(1)),
    };
    assert!(matches!(
        Arguments::new(vec![("date", Argument::Date(invalid_zone))]),
        Err(Problem::Arguments)
    ));
    let private = Arguments::new(vec![("value", text("PRIVATE-ARGUMENT-CANARY"))]).expect("args");
    assert!(!format!("{private:?}").contains("PRIVATE-ARGUMENT-CANARY"));
}

#[test]
fn individual_argument_diagnostics_do_not_reopen_private_values() {
    assert!(!format!("{:?}", text("PRIVATE-ARGUMENT-CANARY")).contains("PRIVATE-ARGUMENT-CANARY"));
}
