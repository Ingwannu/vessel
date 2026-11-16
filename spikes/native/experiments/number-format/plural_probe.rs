//! Test-only ICU candidate input for the controlled offline experiment.
//! These locales are not registered/shipped by adding this unlinked file.
//! Expectations follow the referenced CLDR rules, not a local rule engine.

use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};

const FIRST_INEXACT_INTEGER: u64 = 9_007_199_254_740_993;
const RUSSIAN_ENDING_ONE: u64 = 9_007_199_254_741_001;
const ARABIC_ENDING_NINETY_NINE: u64 = 9_007_199_254_741_099;
const LARGE_MILLION_MULTIPLE: u64 = 9_007_200_000_000_000;
const LARGE_MILLION_NEXT: u64 = 9_007_200_000_000_001;

#[test]
fn shipped_cardinals_are_stable_for_exact_unsigned_inputs() {
    let english: Locale = "en".parse().unwrap();
    let korean: Locale = "ko".parse().unwrap();
    let english = PluralRules::try_new_cardinal(english.into()).unwrap();
    let korean = PluralRules::try_new_cardinal(korean.into()).unwrap();
    for count in [
        0,
        1,
        2,
        u64::from(u32::MAX),
        FIRST_INEXACT_INTEGER,
        RUSSIAN_ENDING_ONE,
        u64::MAX,
    ] {
        assert_eq!(
            english.category_for(count),
            if count == 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        );
        assert_eq!(korean.category_for(count), PluralCategory::Other);
    }
}

#[test]
fn english_ordinal_retains_the_first_lost_integer_digit() {
    let locale: Locale = "en".parse().unwrap();
    let rules = PluralRules::try_new_ordinal(locale.into()).unwrap();
    for (count, expected) in [
        (1, PluralCategory::One),
        (2, PluralCategory::Two),
        (3, PluralCategory::Few),
        (13, PluralCategory::Other),
        (FIRST_INEXACT_INTEGER, PluralCategory::Few),
    ] {
        assert_eq!(rules.category_for(count), expected);
    }
}

#[test]
fn last_digit_cardinals_use_exact_unsigned_operands() {
    let russian: Locale = "ru".parse().unwrap();
    let arabic: Locale = "ar".parse().unwrap();
    let russian = PluralRules::try_new_cardinal(russian.into()).unwrap();
    let arabic = PluralRules::try_new_cardinal(arabic.into()).unwrap();
    for (count, expected) in [
        (1, PluralCategory::One),
        (11, PluralCategory::Many),
        (21, PluralCategory::One),
        (RUSSIAN_ENDING_ONE, PluralCategory::One),
        (u64::MAX, PluralCategory::Many),
    ] {
        assert_eq!(russian.category_for(count), expected);
    }
    for (count, expected) in [
        (0, PluralCategory::Zero),
        (1, PluralCategory::One),
        (2, PluralCategory::Two),
        (3, PluralCategory::Few),
        (11, PluralCategory::Many),
        (ARABIC_ENDING_NINETY_NINE, PluralCategory::Many),
        (u64::MAX, PluralCategory::Many),
    ] {
        assert_eq!(arabic.category_for(count), expected);
    }
}

#[test]
fn million_divisibility_does_not_round_a_neighbor_into_many() {
    for language in ["es", "fr", "pt-BR"] {
        let locale: Locale = language.parse().unwrap();
        let rules = PluralRules::try_new_cardinal(locale.into()).unwrap();
        assert_eq!(
            rules.category_for(LARGE_MILLION_MULTIPLE),
            PluralCategory::Many,
            "{language}"
        );
        assert_eq!(
            rules.category_for(LARGE_MILLION_NEXT),
            PluralCategory::Other,
            "{language}"
        );
    }
}
