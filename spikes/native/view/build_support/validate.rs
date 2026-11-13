//! Gate rules shared by cargo test, generation, and the coverage command.
//! Missing contributed messages are reported; invalid messages never ship.

use std::collections::{BTreeMap, BTreeSet};

use super::{BuildError, Coverage, Kind, Language, Shape, pattern, source};

pub(crate) fn expanded(language: &Language, key: &str) -> Result<Shape, BuildError> {
    let shape = expand(language, key, &mut BTreeSet::new())?;
    if !shape.conflicts.is_empty() {
        return Err(source(
            &language.tag,
            format!(
                "{key} uses incompatible types for ${}",
                shape
                    .conflicts
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", $")
            ),
        ));
    }
    Ok(shape)
}

fn expand(
    language: &Language,
    key: &str,
    visiting: &mut BTreeSet<String>,
) -> Result<Shape, BuildError> {
    if visiting.len() >= 64 || !visiting.insert(key.to_owned()) {
        return Err(source(
            &language.tag,
            format!("cyclic or excessively deep reference at {key}"),
        ));
    }
    let mut shape = language
        .messages
        .get(key)
        .map(|m| m.shape.clone())
        .or_else(|| language.terms.get(key).cloned())
        .ok_or_else(|| {
            source(
                &language.tag,
                format!("missing reference {key}; translate it too or omit the referring message"),
            )
        })?;
    for reference in &shape.references.clone() {
        let referenced = expand(language, &reference.key, visiting)?;
        shape.conflicts.extend(referenced.conflicts);
        for (name, kind) in referenced.variables {
            if !reference.bound.contains(&name) {
                if reference.key.starts_with('-') {
                    return Err(source(
                        &language.tag,
                        format!("{} needs a locally bound ${name} parameter", reference.key),
                    ));
                }
                pattern::merge(&mut shape, &name, kind);
            }
        }
    }
    let _removed = visiting.remove(key);
    Ok(shape)
}

pub(crate) fn validate(languages: &[Language]) -> Result<Vec<Coverage>, BuildError> {
    let english = languages
        .iter()
        .find(|l| l.tag == "en")
        .ok_or_else(|| source("en", "English source folder is required"))?;
    if !languages.iter().any(|l| l.tag == "ko") {
        return Err(source("ko", "Korean folder is required"));
    }
    let contracts: BTreeMap<_, _> = english
        .messages
        .keys()
        .map(|key| Ok((key.clone(), expanded(english, key)?)))
        .collect::<Result<_, BuildError>>()?;
    for (key, message) in &english.messages {
        if message.context.trim().is_empty() {
            return Err(source(
                "en",
                format!("{key} needs a translator context comment"),
            ));
        }
    }
    let mut coverage = Vec::new();
    for language in languages {
        if language
            .messages
            .get("meta-name")
            .and_then(|m| m.plain.as_deref())
            .is_none_or(|n| n.trim().is_empty())
        {
            return Err(source(
                &language.tag,
                "meta-name must be a nonempty plain native language name",
            ));
        }
        let missing: Vec<_> = contracts
            .keys()
            .filter(|key| !language.messages.contains_key(*key))
            .cloned()
            .collect();
        if language.tag == "ko" && !missing.is_empty() {
            return Err(source(
                "ko",
                format!("missing messages: {}", missing.join(", ")),
            ));
        }
        for key in language.terms.keys() {
            let _checked = expanded(language, key)?;
        }
        for key in language.messages.keys() {
            let expected = contracts
                .get(key)
                .ok_or_else(|| source(&language.tag, format!("unknown message {key}")))?;
            let actual = expanded(language, key)?;
            if actual
                .variables
                .keys()
                .any(|name| !expected.variables.contains_key(name))
            {
                return Err(source(
                    &language.tag,
                    format!("{key} uses variables English does not pass"),
                ));
            }
            if language.tag == "ko" && !actual.variables.keys().eq(expected.variables.keys()) {
                return Err(source(
                    "ko",
                    format!("{key} must use the same variables as English"),
                ));
            }
            for (name, kind) in &actual.variables {
                super::catalog::check_identifier(name, &language.tag)?;
                if let Kind::Choice(choices) = kind {
                    for choice in choices {
                        super::catalog::check_identifier(choice, &language.tag)?;
                    }
                }
                if *kind == Kind::Date && expected.variables.get(name) != Some(&Kind::Date) {
                    return Err(source(
                        &language.tag,
                        format!("{key}: ${name} requires a date but English passes another type"),
                    ));
                }
                if *kind == Kind::Number && expected.variables.get(name) != Some(&Kind::Number) {
                    return Err(source(
                        &language.tag,
                        format!("{key}: ${name} requires a number but English passes text"),
                    ));
                }
            }
        }
        coverage.push(Coverage {
            tag: language.tag.clone(),
            missing,
        });
    }
    Ok(coverage)
}
