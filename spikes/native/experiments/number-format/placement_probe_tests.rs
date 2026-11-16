//! Folder-data projection tests, not an area-local number formatter.

use super::*;
use fixed_decimal::Decimal;

#[test]
fn decimal_and_group_characters_are_data() {
    let symbols = Symbols::new(
        ".",
        ",",
        "٠١٢٣٤٥٦٧٨٩",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(2).unwrap(),
    )
    .unwrap();
    let value: Decimal = "12345678.50".parse().unwrap();
    assert_eq!(
        symbols.format(&value.absolute, Grouping::Auto).unwrap(),
        "١.٢٣.٤٥.٦٧٨,٥٠"
    );
    assert_eq!(
        symbols.format(&value.absolute, Grouping::Never).unwrap(),
        "١٢٣٤٥٦٧٨,٥٠"
    );
}

#[test]
fn compact_minimum_two_changes_only_group_placement() {
    let symbols = Symbols::new(
        ",",
        ".",
        "0123456789",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
    .unwrap();
    for (value, ordinary, compact) in [
        ("1000", "1,000", "1000"),
        ("10000", "10,000", "10,000"),
        ("12345.6", "12,345.6", "12,345.6"),
    ] {
        let value: Decimal = value.parse().unwrap();
        assert_eq!(
            symbols.format(&value.absolute, Grouping::Auto).unwrap(),
            ordinary
        );
        assert_eq!(
            symbols
                .format(&value.absolute, Grouping::MinimumTwo)
                .unwrap(),
            compact
        );
    }
}

#[test]
fn full_unsigned_count_keeps_every_digit() {
    let symbols = Symbols::new(
        ",",
        ".",
        "0123456789",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
    .unwrap();
    let value = Decimal::from(u64::MAX);
    assert_eq!(
        symbols.format(&value.absolute, Grouping::Auto).unwrap(),
        "18,446,744,073,709,551,615"
    );
}

#[test]
fn invalid_digit_sets_fail_before_formatting() {
    for digits in ["012345678", "01234567890", "0123456788"] {
        assert!(
            Symbols::new(
                ",",
                ".",
                digits,
                NonZeroUsize::new(3).unwrap(),
                NonZeroUsize::new(3).unwrap(),
            )
            .is_err()
        );
    }
}
