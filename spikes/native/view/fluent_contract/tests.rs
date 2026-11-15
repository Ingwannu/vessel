//! The same parser/shape API is usable without a directory or core registry.
use super::{
    ContractError, Key, Kind, Name, PatternRole, Shape, VariableCoverage, compare, expanded, scan,
};
use fluent_syntax::{ast::Entry, parser};
use std::collections::BTreeMap;

fn shapes(source: &str) -> Result<BTreeMap<Key, Shape>, ContractError> {
    let resource = parser::parse(source).unwrap();
    let mut shapes = BTreeMap::new();
    for entry in resource.body {
        let (key, pattern) = match entry {
            Entry::Message(message) => (
                Key::Message(Name::parse(message.id.name)?),
                message.value.unwrap(),
            ),
            Entry::Term(term) => (Key::Term(Name::parse(term.id.name)?), term.value),
            Entry::Comment(_)
            | Entry::GroupComment(_)
            | Entry::ResourceComment(_)
            | Entry::Junk { .. } => continue,
        };
        let mut shape = Shape::default();
        scan(&pattern, &mut shape, PatternRole::Message)?;
        assert!(shapes.insert(key, shape).is_none());
    }
    Ok(shapes)
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
