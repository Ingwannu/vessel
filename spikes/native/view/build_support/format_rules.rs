//! Only the English source defines which numeric settings exist. Translated
//! folders supply values; the target gets checked static data, never a parser.
use super::{BuildError, Language, source};
use std::{collections::BTreeSet, fmt::Write as _};

pub(crate) fn generate(languages: &[Language]) -> Result<String, BuildError> {
    let english = languages
        .iter()
        .find(|l| l.tag == "en")
        .ok_or_else(|| source("en", "English formatting rules required"))?;
    validate_patterns(english)?;
    let mut output = String::from("static NUMBER_RULES: &[number::Rules] = &[\n");
    for language in languages {
        let value = |key: &str| {
            language
                .messages
                .get(key)
                .or_else(|| english.messages.get(key))
                .and_then(|m| m.plain.as_deref())
                .ok_or_else(|| {
                    source(
                        &language.tag,
                        format!("{key} must be a literal formatting setting"),
                    )
                })
        };
        let group = value("formats-group-separator")?;
        let decimal = value("formats-decimal-separator")?;
        let digits = value("formats-digits")?;
        let unique: BTreeSet<_> = digits.chars().collect();
        if decimal.is_empty()
            || decimal == group
            || digits.chars().count() != 10
            || unique.len() != 10
        {
            return Err(source(
                &language.tag,
                "formatting needs distinct separators and ten unique digit characters",
            ));
        }
        let size = |key| -> Result<usize, BuildError> {
            let n = value(key)?.parse::<usize>().map_err(|error| {
                source(&language.tag, format!("{key} must be 1 through 9: {error}"))
            })?;
            if !(1..=9).contains(&n) {
                return Err(source(&language.tag, format!("{key} must be 1 through 9")));
            }
            Ok(n)
        };
        let primary = size("formats-group-primary")?;
        let secondary = size("formats-group-secondary")?;
        let cycle = value("formats-hour-cycle")?;
        if !matches!(cycle, "h12" | "h24") {
            return Err(source(
                &language.tag,
                "formats-hour-cycle must be h12 or h24",
            ));
        }
        writeln!(
            output,
            "number::Rules {{ tag: {:?}, group: {group:?}, decimal: {decimal:?}, digits: {digits:?}, primary: {primary}, secondary: {secondary}, hour_cycle: {cycle:?} }},",
            language.tag
        )?;
    }
    writeln!(output, "];")?;
    Ok(output)
}

fn validate_patterns(english: &Language) -> Result<(), BuildError> {
    use super::Kind;
    for (key, allowed) in [
        ("formats-month", &["month"][..]),
        ("formats-weekday", &["weekday"][..]),
        ("formats-date", &["year", "month", "day", "weekday"][..]),
        (
            "formats-time",
            &[
                "hour-cycle",
                "hour",
                "hour-twelve",
                "minute",
                "period",
                "zone",
            ][..],
        ),
        (
            "formats-date-time",
            &[
                "year",
                "month",
                "day",
                "weekday",
                "hour-cycle",
                "hour",
                "hour-twelve",
                "minute",
                "period",
                "zone",
            ][..],
        ),
    ] {
        let shape = super::validate::expanded(english, key)?;
        for (name, kind) in shape.variables() {
            let valid_type = match name.as_str() {
                "zone" => *kind == Kind::Text,
                "hour-cycle" | "period" => matches!(kind, Kind::Choice(_)),
                _ => *kind == Kind::Number,
            };
            let valid_choices = match kind {
                Kind::Choice(choices) => choices.iter().all(|choice| match name.as_str() {
                    "hour-cycle" => matches!(choice.as_str(), "h12" | "h24"),
                    "period" => matches!(choice.as_str(), "am" | "pm"),
                    _ => false,
                }),
                Kind::Text | Kind::Number | Kind::Date => true,
            };
            if !allowed.contains(&name.as_str()) || !valid_type || !valid_choices {
                return Err(source(
                    "en",
                    format!("{key} requests an unsupported typed date/time part ${name}"),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_numeric_rules_are_generated_and_invalid_rules_fail() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut languages = super::super::read(&root).unwrap();
        let fixture_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/locales");
        let partial = super::super::read(&fixture_root)
            .unwrap()
            .into_iter()
            .find(|l| l.tag == "ja")
            .unwrap();
        languages.push(partial);
        let template = languages
            .iter()
            .find(|l| l.tag == "en")
            .unwrap()
            .messages
            .clone();
        let ja = languages.iter_mut().find(|l| l.tag == "ja").unwrap();
        for (key, value) in [
            ("formats-group-separator", "."),
            ("formats-decimal-separator", ","),
            ("formats-digits", "٠١٢٣٤٥٦٧٨٩"),
            ("formats-group-secondary", "2"),
        ] {
            let mut message = template[key].clone();
            message.plain = Some(value.to_owned());
            let _old = ja.messages.insert(key.to_owned(), message);
        }
        let output = generate(&languages).unwrap();
        assert_eq!(
            output
                .lines()
                .find(|line| line.starts_with("number::Rules { tag: \"ja\"")),
            Some(
                "number::Rules { tag: \"ja\", group: \".\", decimal: \",\", digits: \"٠١٢٣٤٥٦٧٨٩\", primary: 3, secondary: 2, hour_cycle: \"h12\" },"
            )
        );
        for (key, value) in [
            ("formats-digits", "0000000000"),
            ("formats-group-primary", "0"),
            ("formats-group-secondary", "99"),
            ("formats-hour-cycle", "h13"),
            ("formats-decimal-separator", "."),
        ] {
            let mut invalid = languages.clone();
            let ja = invalid.iter_mut().find(|l| l.tag == "ja").unwrap();
            let mut message = template[key].clone();
            message.plain = Some(value.to_owned());
            let _old = ja.messages.insert(key.to_owned(), message);
            assert!(generate(&invalid).is_err(), "{key}");
        }
    }
}
