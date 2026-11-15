//! The same parser/shape API is usable without a directory or core registry.
use super::{
    ContractError, Key, Kind, Name, PatternRole, Shape, VariableCoverage, compare, expanded, scan,
};
use fluent_syntax::{ast::Entry, parser};
use std::collections::BTreeMap;

// Invalid fixture syntax is a test author error; validation failures remain
// Results from the separate scanner below, rather than panics inside Result.
fn fixture_patterns(source: &str) -> BTreeMap<Key, fluent_syntax::ast::Pattern<&str>> {
    let resource = parser::parse(source).unwrap();
    let mut patterns = BTreeMap::new();
    for entry in resource.body {
        let (key, pattern) = match entry {
            Entry::Message(message) => (
                Key::Message(Name::parse(message.id.name).unwrap()),
                message.value.unwrap(),
            ),
            Entry::Term(term) => (Key::Term(Name::parse(term.id.name).unwrap()), term.value),
            Entry::Comment(_)
            | Entry::GroupComment(_)
            | Entry::ResourceComment(_)
            | Entry::Junk { .. } => continue,
        };
        assert!(patterns.insert(key, pattern).is_none());
    }
    patterns
}

fn shapes(source: &str) -> Result<BTreeMap<Key, Shape>, ContractError> {
    fixture_patterns(source)
        .into_iter()
        .map(|(key, pattern)| {
            let mut shape = Shape::default();
            scan(&pattern, &mut shape, PatternRole::Message)?;
            Ok((key, shape))
        })
        .collect()
}

#[test]
fn reference_expansion_preserves_the_english_number_date_and_choice_contract() {
    let graph = shapes("parts = { NUMBER($count) } { DATETIME($date) }\nlabel = { parts } { $state ->\n [on] On\n *[off] Off\n}").unwrap();
    let key = Key::parse("label").unwrap();
    let shape = expanded(&key, |key| graph.get(key)).unwrap();
    assert_eq!(shape.variables().get("count"), Some(&Kind::Number));
    assert_eq!(shape.variables().get("date"), Some(&Kind::Date));
    assert!(
        matches!(shape.variables().get("state"), Some(Kind::Choice(choices)) if choices.contains("on") && choices.contains("off"))
    );
    let translated = shapes("label = { $extra }").unwrap();
    let actual = expanded(&key, |key| translated.get(key)).unwrap();
    assert!(matches!(
        compare(&key, &shape, &actual, VariableCoverage::Subset),
        Err(ContractError::ExtraVariables(_))
    ));
    let plain_graph = shapes("label = Plain").unwrap();
    let plain = expanded(&key, |key| plain_graph.get(key)).unwrap();
    assert!(compare(&key, &shape, &plain, VariableCoverage::Subset).is_ok());
    assert!(matches!(
        compare(&key, &shape, &plain, VariableCoverage::Exact),
        Err(ContractError::MissingVariables(_))
    ));
}

#[test]
fn translated_functions_cannot_require_types_the_english_caller_does_not_pass() {
    let key = Key::parse("label").unwrap();
    let english = shapes("label = { $value }").unwrap();
    let expected = expanded(&key, |key| english.get(key)).unwrap();
    for (source, date) in [
        ("label = { DATETIME($value) }", true),
        ("label = { NUMBER($value) }", false),
    ] {
        let translated = shapes(source).unwrap();
        let actual = expanded(&key, |key| translated.get(key)).unwrap();
        let error = compare(&key, &expected, &actual, VariableCoverage::Subset).unwrap_err();
        assert!(matches!(
            (error, date),
            (ContractError::DateRequired { .. }, true)
                | (ContractError::NumberRequired { .. }, false)
        ));
    }
}

#[test]
fn named_term_arguments_are_local_not_extra_caller_variables() {
    let graph = shapes("-thing = { NUMBER($count) }\nlabel = { -thing(count: 2) }").unwrap();
    assert!(
        expanded(&Key::parse("label").unwrap(), |key| graph.get(key))
            .unwrap()
            .variables()
            .is_empty()
    );
}

#[test]
fn unsupported_number_operands_are_rejected_before_callback_error_can_be_silent() {
    for source in [
        "label = { NUMBER(\"text\") }",
        "other = 2\nlabel = { NUMBER(other) }",
        "label = { NUMBER(DATETIME($date)) }",
        "label = { NUMBER(PARTICLE($name, \"object\")) }",
    ] {
        assert!(matches!(shapes(source), Err(ContractError::NumberArgument)));
    }
    for source in [
        "label = { NUMBER(2) }",
        "label = { NUMBER(NUMBER($count)) }",
    ] {
        let graph = shapes(source).unwrap();
        assert!(expanded(&Key::parse("label").unwrap(), |key| graph.get(key)).is_ok());
    }
}

#[test]
fn term_literals_must_match_the_referenced_typed_parameter() {
    for source in [
        "-term = { NUMBER($count) }\nlabel = { -term(count: \"text\") }",
        "-term = { DATETIME($date) }\nlabel = { -term(date: \"text\") }",
        "-term = { DATETIME($date) }\nlabel = { -term(date: 2) }",
    ] {
        let graph = shapes(source).unwrap();
        assert!(matches!(
            expanded(&Key::parse("label").unwrap(), |key| graph.get(key)),
            Err(ContractError::BindingType { .. })
        ));
    }
    let graph = shapes("-term = { NUMBER($count) }\nlabel = { -term(count: 2) }").unwrap();
    assert!(
        expanded(&Key::parse("label").unwrap(), |key| graph.get(key))
            .unwrap()
            .variables()
            .is_empty()
    );
}

#[test]
fn typed_dates_cannot_be_read_as_their_private_encoding() {
    let key = Key::parse("label").unwrap();
    let en = shapes("label = { DATETIME($date) }").unwrap();
    let expected = expanded(&key, |key| en.get(key)).unwrap();
    let translated = shapes("label = { $date }").unwrap();
    let actual = expanded(&key, |key| translated.get(key)).unwrap();
    assert!(matches!(
        compare(&key, &expected, &actual, VariableCoverage::Exact),
        Err(ContractError::DateTextRead { .. })
    ));
    for source in [
        "label = { DATETIME($date) } { $date }",
        "piece = { $date }\nlabel = { DATETIME($date) } { piece }",
    ] {
        let graph = shapes(source).unwrap();
        assert!(matches!(
            expanded(&key, |key| graph.get(key)),
            Err(ContractError::DateTextRead { .. })
        ));
    }
}

#[test]
fn formatted_date_composition_and_locally_shadowed_names_remain_valid() {
    for source in [
        "label = { DATETIME($date) }{ PARTICLE(DATETIME($date), \"object\") }",
        "-term = { $date }\nlabel = { DATETIME($date) } { -term(date: \"caption\") }",
    ] {
        let graph = shapes(source).unwrap();
        assert!(expanded(&Key::parse("label").unwrap(), |key| graph.get(key)).is_ok());
    }
    let key = Key::parse("label").unwrap();
    let en = shapes("label = { NUMBER($count) }").unwrap();
    let ko = shapes("label = { $count }").unwrap();
    assert!(
        compare(
            &key,
            &expanded(&key, |key| en.get(key)).unwrap(),
            &expanded(&key, |key| ko.get(key)).unwrap(),
            VariableCoverage::Exact
        )
        .is_ok()
    );
}
