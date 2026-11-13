//! Keep NUMBER numeric for Fluent's plural selectors. The formatter hook
//! changes display only, using folder-owned rules cached by Fluent's memoizer.
use fluent_bundle::{FluentArgs, FluentValue, types::FluentNumber};
use intl_memoizer::{Memoizable, concurrent::IntlLangMemoizer};
use std::convert::Infallible;

#[derive(Debug)]
pub(super) struct Rules {
    pub(super) tag: &'static str,
    pub(super) group: &'static str,
    pub(super) decimal: &'static str,
    pub(super) digits: &'static str,
    pub(super) primary: usize,
    pub(super) secondary: usize,
    pub(super) hour_cycle: &'static str,
}

pub(super) fn rules(tag: &str) -> Option<&'static Rules> {
    super::NUMBER_RULES
        .iter()
        .find(|r| r.tag == tag)
        .or_else(|| super::NUMBER_RULES.iter().find(|r| r.tag == "en"))
}

struct Formatter(Option<&'static Rules>);
impl Memoizable for Formatter {
    type Args = ();
    type Error = Infallible;
    fn construct(lang: unic_langid::LanguageIdentifier, (): ()) -> Result<Self, Infallible> {
        Ok(Self(rules(&lang.to_string())))
    }
}

/// Keep the numeric result, including any requested rounding, available to
/// Fluent's plural matcher. Returning a display string here would lose it.
pub(super) fn fluent<'a>(
    positional: &[FluentValue<'a>],
    named: &FluentArgs<'_>,
) -> FluentValue<'a> {
    let Some(FluentValue::Number(value)) = positional.first() else {
        return FluentValue::Error;
    };
    let mut value = value.clone();
    value.options.merge(named);
    if let Some(maximum) = value.options.maximum_fraction_digits {
        value.options.minimum_fraction_digits = Some(
            value
                .options
                .minimum_fraction_digits
                .unwrap_or(0)
                .min(maximum),
        );
        let rounded = format!("{:.*}", maximum.min(9), value.value);
        let Ok(rounded) = rounded.parse::<f64>() else {
            return FluentValue::Error;
        };
        value.value = rounded;
    }
    value.into()
}

pub(super) fn display(value: &FluentValue<'_>, memo: &IntlLangMemoizer) -> Option<String> {
    let FluentValue::Number(number) = value else {
        return None;
    };
    let result = memo.with_try_get::<Formatter, _, _>((), |formatter| {
        formatter.0.map(|rules| format(number, rules))
    });
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

fn format(number: &FluentNumber, rules: &Rules) -> String {
    // Rust/Fluent own the decimal representation; this is digit placement,
    // not a number parser. No currency, percent or significant-digit engine.
    let minimum = number.options.minimum_fraction_digits.unwrap_or(0).min(9);
    let mut raw = number.options.maximum_fraction_digits.map_or_else(
        || number.value.to_string(),
        |maximum| format!("{:.*}", maximum.min(9), number.value),
    );
    if let Some((integer, fraction)) = raw.split_once('.') {
        let fraction = fraction.trim_end_matches('0');
        raw = if fraction.is_empty() {
            integer.to_owned()
        } else {
            format!("{integer}.{fraction}")
        };
    }
    let fraction_length = raw.split_once('.').map_or(0, |(_, f)| f.len());
    if minimum > fraction_length {
        if !raw.contains('.') {
            raw.push('.');
        }
        raw.push_str(&"0".repeat(minimum.saturating_sub(fraction_length)));
    }
    let (integer, fraction) = raw
        .split_once('.')
        .map_or((raw.as_str(), None), |(a, b)| (a, Some(b)));
    let (sign, integer) = integer
        .strip_prefix('-')
        .map_or(("", integer), |n| ("-", n));
    let width = number.options.minimum_integer_digits.unwrap_or(0).min(20);
    let integer = format!("{integer:0>width$}");
    let mut ascii = String::from(sign);
    let length = integer.len();
    for (position, digit) in integer.chars().enumerate() {
        let remaining = length.saturating_sub(position);
        if number.options.use_grouping
            && position > 0
            && remaining >= rules.primary
            && (remaining == rules.primary
                || remaining
                    .saturating_sub(rules.primary)
                    .is_multiple_of(rules.secondary))
        {
            ascii.push_str(rules.group);
        }
        ascii.push(digit);
    }
    if let Some(fraction) = fraction.filter(|s| !s.is_empty()) {
        ascii.push_str(rules.decimal);
        ascii.push_str(fraction);
    }
    ascii
        .chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|n| usize::try_from(n).ok())
                .and_then(|n| rules.digits.chars().nth(n))
                .unwrap_or(c)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::types::FluentNumberOptions;
    #[test]
    fn grouping_digits_and_decimal_are_data_not_language_branches() {
        let rules = Rules {
            tag: "test",
            group: ".",
            decimal: ",",
            digits: "٠١٢٣٤٥٦٧٨٩",
            primary: 3,
            secondary: 2,
            hour_cycle: "h24",
        };
        let number = FluentNumber::new(
            -12_345_678.5,
            FluentNumberOptions {
                minimum_fraction_digits: Some(2),
                ..FluentNumberOptions::default()
            },
        );
        assert_eq!(format(&number, &rules), "-١.٢٣.٤٥.٦٧٨,٥٠");
        let ungrouped = FluentNumber::new(
            4.0,
            FluentNumberOptions {
                use_grouping: false,
                minimum_integer_digits: Some(2),
                ..FluentNumberOptions::default()
            },
        );
        assert_eq!(format(&ungrouped, &rules), "٠٤");
    }
}
