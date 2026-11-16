//! A balance amount as the viewer's language writes it, after Paseo
//! usage/format.ts `formatAmount` (185-199).
//!
//! [Decision Log] (2026-11-16, coordinator: balances printed raw floats)
//! - Purpose: "0.3" and "1,234.5" in the viewer's grouping and decimal mark,
//!   never Rust's `0.30000000000000004`.
//! - Constraint: ADR 0101 (shared amount/count formatter with currency and
//!   compact notation) is proposed but not built; the L track is paused.
//! - Choice (coordinator approved): the line's Fluent `NUMBER` (ADR 0039
//!   folder rules) with Paseo's fraction policy: USD two decimals, other
//!   units at most two, the label naming the unit as in Paseo. The value is
//!   rounded by decimal formatting before it becomes a message number, so a
//!   binary float's tail cannot reach the formatter.
//! - Gaps until ADR 0101 lands, then this module switches to it: tokens show
//!   whole and grouped instead of compact (1.2K / 1.2천), and an amount whose
//!   magnitude exceeds what `Number::decimal` takes (above u32) is written
//!   by Rust's rounded text without grouping. Exact ties round half to even
//!   here, half away from zero in Intl.
use crate::localization::{Number, Translator};
use vessel_proto::usage_reports::UsageBalanceUnit;

/// One amount in `unit`, signed, in the viewer's number format.
#[must_use]
pub fn amount_text(value: f64, unit: UsageBalanceUnit, words: &'static Translator) -> String {
    let digits = match unit {
        UsageBalanceUnit::Tokens => 0,
        UsageBalanceUnit::Usd | UsageBalanceUnit::Credits | UsageBalanceUnit::Requests => 2,
    };
    let rounded = format!("{value:.digits$}");
    let Ok(parsed) = rounded.parse::<f64>() else {
        return rounded;
    };
    let Ok(number) = Number::decimal(parsed.abs()) else {
        return rounded;
    };
    let text = words.usage();
    let shown = match unit {
        UsageBalanceUnit::Usd => text.amount_usd(number),
        UsageBalanceUnit::Tokens => text.amount_tokens(number),
        UsageBalanceUnit::Credits | UsageBalanceUnit::Requests => text.amount(number),
    };
    let Ok(shown) = shown else {
        return rounded;
    };
    // Intl writes the sign before the currency ("-$5.00"); a rounded zero
    // shows no sign.
    if parsed < 0.0 {
        format!("-{shown}")
    } else {
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};

    fn en() -> &'static Translator {
        Translator::get(ENGLISH).unwrap()
    }
    fn ko() -> &'static Translator {
        Translator::get(Locale::from_tag("ko").unwrap()).unwrap()
    }

    #[test]
    fn a_binary_tail_never_reaches_the_text() {
        // Literal sums: the binary tail a source can report (0.1 + 0.2).
        let noisy = 0.1_f64 + 0.2_f64;
        assert_eq!(amount_text(noisy, UsageBalanceUnit::Credits, en()), "0.3");
        assert_eq!(
            amount_text(2.0 / 3.0, UsageBalanceUnit::Requests, en()),
            "0.67"
        );
        assert_eq!(amount_text(6.0, UsageBalanceUnit::Credits, en()), "6");
    }

    #[test]
    fn grouping_currency_and_tokens_follow_paseo_in_both_languages() {
        assert_eq!(
            amount_text(1234.5, UsageBalanceUnit::Usd, en()),
            "$1,234.50"
        );
        assert_eq!(
            amount_text(1234.5, UsageBalanceUnit::Usd, ko()),
            "US$1,234.50"
        );
        assert_eq!(
            amount_text(12345.678, UsageBalanceUnit::Credits, ko()),
            "12,345.68"
        );
        assert_eq!(
            amount_text(1_234_567.4, UsageBalanceUnit::Tokens, en()),
            "1,234,567"
        );
        // Negative literals parsed, so the test needs no float negation.
        let debt: f64 = "-5".parse().unwrap();
        let dust: f64 = "-0.001".parse().unwrap();
        assert_eq!(amount_text(debt, UsageBalanceUnit::Usd, en()), "-$5.00");
        assert_eq!(amount_text(dust, UsageBalanceUnit::Credits, en()), "0");
    }

    #[test]
    fn beyond_the_message_range_the_rounded_value_still_shows() {
        assert_eq!(
            amount_text(5_000_000_000.25, UsageBalanceUnit::Credits, en()),
            "5000000000.25"
        );
    }
}
