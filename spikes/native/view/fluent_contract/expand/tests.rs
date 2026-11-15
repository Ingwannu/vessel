//! Adversarial reference graphs protect runtime consumers of the host contract.
use super::super::{ContractError, Key, Kind, Name, Reference, Shape};
use super::{MAX_REFERENCE_DEPTH, expanded};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};

type Graph = BTreeMap<Key, Shape>;
fn key(name: &str) -> Key {
    Key::parse(name).unwrap()
}
fn reference(name: &str) -> Reference {
    Reference {
        key: key(name),
        bound: BTreeSet::new(),
    }
}
fn graph_chain(length: usize) -> Graph {
    (0..length)
        .map(|index| {
            let mut shape = Shape::default();
            if index.saturating_add(1) < length {
                shape
                    .references
                    .push(reference(&format!("node-{}", index.saturating_add(1))));
            }
            (key(&format!("node-{index}")), shape)
        })
        .collect()
}

#[test]
fn reference_depth_limit_includes_the_root() {
    for (length, allowed) in [
        (MAX_REFERENCE_DEPTH, true),
        (MAX_REFERENCE_DEPTH.saturating_add(1), false),
    ] {
        let graph = graph_chain(length);
        assert_eq!(
            expanded(&key("node-0"), |id| graph.get(id)).is_ok(),
            allowed
        );
    }
}

#[test]
fn duplicated_dag_edges_visit_each_definition_once() {
    let mut graph = graph_chain(MAX_REFERENCE_DEPTH);
    for shape in graph.values_mut() {
        if let Some(reference) = shape.references.first().cloned() {
            shape.references.push(reference);
        }
    }
    let lookups = Cell::new(0usize);
    assert!(
        expanded(&key("node-0"), |id| {
            lookups.set(lookups.get().saturating_add(1));
            graph.get(id)
        })
        .is_ok()
    );
    assert_eq!(lookups.get(), graph.len());
}

#[test]
fn a_cached_subtree_cannot_bypass_depth_on_a_second_longer_path() {
    let mut graph = graph_chain(MAX_REFERENCE_DEPTH.saturating_sub(1));
    let root = Shape {
        references: vec![reference("node-0"), reference("indirect")],
        ..Shape::default()
    };
    assert!(graph.insert(key("root"), root).is_none());
    assert!(
        graph
            .insert(
                key("indirect"),
                Shape {
                    references: vec![reference("node-0")],
                    ..Shape::default()
                }
            )
            .is_none()
    );
    assert!(matches!(
        expanded(&key("root"), |id| graph.get(id)),
        Err(ContractError::ReferenceDepth(_))
    ));
}

#[test]
fn missing_cycles_unbound_terms_and_conflicting_reference_types_are_typed() {
    let mut graph = Graph::new();
    assert!(matches!(
        expanded(&key("missing"), |id| graph.get(id)),
        Err(ContractError::MissingReference(_))
    ));
    assert!(
        graph
            .insert(
                key("cycle"),
                Shape {
                    references: vec![reference("cycle")],
                    ..Shape::default()
                }
            )
            .is_none()
    );
    assert!(matches!(
        expanded(&key("cycle"), |id| graph.get(id)),
        Err(ContractError::ReferenceCycle(_))
    ));
    assert!(
        graph
            .insert(
                key("-term"),
                Shape {
                    variables: BTreeMap::from([(Name::parse("count").unwrap(), Kind::Number)]),
                    ..Shape::default()
                }
            )
            .is_none()
    );
    assert!(
        graph
            .insert(
                key("unbound"),
                Shape {
                    references: vec![reference("-term")],
                    ..Shape::default()
                }
            )
            .is_none()
    );
    assert!(matches!(
        expanded(&key("unbound"), |id| graph.get(id)),
        Err(ContractError::UnboundTerm { .. })
    ));
    let mut conflicting = Shape {
        references: vec![reference("number")],
        ..Shape::default()
    };
    conflicting.merge(Name::parse("value").unwrap(), Kind::Date);
    assert!(graph.insert(key("root"), conflicting).is_none());
    assert!(
        graph
            .insert(
                key("number"),
                Shape {
                    variables: BTreeMap::from([(Name::parse("value").unwrap(), Kind::Number)]),
                    ..Shape::default()
                }
            )
            .is_none()
    );
    assert!(matches!(
        expanded(&key("root"), |id| graph.get(id)),
        Err(ContractError::ConflictingTypes { .. })
    ));
}
