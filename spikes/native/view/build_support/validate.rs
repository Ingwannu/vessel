//! Gate rules shared by cargo test, generation, and the coverage command.
//! Missing contributed messages are reported; invalid messages never ship.

use std::collections::BTreeMap;

use super::{BuildError, Contract, Coverage, Kind, Language, contract, source};

pub(crate) fn expanded(language: &Language, key: &str) -> Result<Contract, BuildError> {
    let key = contract::Key::parse(key).map_err(|e| source(&language.tag, e.to_string()))?;
    contract::expanded(&key, |key| match key {
        contract::Key::Message(name) => language.messages.get(name.as_str()).map(|m| &m.shape),
        contract::Key::Term(name) => language.terms.get(&format!("-{name}")),
    })
    .map_err(|error| source(&language.tag, error.to_string()))
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
    languages
        .iter()
        .map(|language| validate_language(&contracts, language))
        .collect()
}

fn validate_language(
    contracts: &BTreeMap<String, Contract>,
    language: &Language,
) -> Result<Coverage, BuildError> {
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
    validate_messages(contracts, language)?;
    Ok(Coverage {
        tag: language.tag.clone(),
        missing,
    })
}

fn validate_messages(
    contracts: &BTreeMap<String, Contract>,
    language: &Language,
) -> Result<(), BuildError> {
    for key in language.messages.keys() {
        let expected = contracts
            .get(key)
            .ok_or_else(|| source(&language.tag, format!("unknown message {key}")))?;
        let actual = expanded(language, key)?;
        let coverage = if language.tag == "ko" {
            contract::VariableCoverage::Exact
        } else {
            contract::VariableCoverage::Subset
        };
        let parsed_key =
            contract::Key::parse(key).map_err(|e| source(&language.tag, e.to_string()))?;
        contract::compare(&parsed_key, expected, &actual, coverage)
            .map_err(|error| source(&language.tag, error.to_string()))?;
        for (name, kind) in actual.variables() {
            super::catalog::check_identifier(name, &language.tag)?;
            if let Kind::Choice(choices) = kind {
                for choice in choices {
                    super::catalog::check_identifier(choice, &language.tag)?;
                }
            }
        }
    }
    Ok(())
}
