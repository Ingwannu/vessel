//! Expand reference contracts with cycle/depth checks and memoized DAG traversal.
//! Memo entries retain subtree height: a cached shallow visit must not let the
//! same subtree bypass the limit when reached through a deeper path later.
use super::{
    Contract, ContractError, Key, Shape,
    types::{BindingRequirement, Conflicts, LocalLiteral},
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_REFERENCE_DEPTH: usize = 64;

#[derive(Clone)]
struct Expanded {
    shape: Shape,
    height: usize,
}

pub(crate) fn expanded<'a>(
    key: &Key,
    lookup: impl Fn(&Key) -> Option<&'a Shape>,
) -> Result<Contract, ContractError> {
    let expanded = expand(key, 0, &lookup, &mut BTreeSet::new(), &mut BTreeMap::new())?;
    if !expanded.shape.conflicts.is_empty() {
        return Err(ContractError::ConflictingTypes {
            key: key.clone(),
            variables: Conflicts(expanded.shape.conflicts),
        });
    }
    Ok(Contract {
        variables: expanded.shape.variables,
    })
}

fn expand<'a>(
    key: &Key,
    depth: usize,
    lookup: &impl Fn(&Key) -> Option<&'a Shape>,
    visiting: &mut BTreeSet<Key>,
    memo: &mut BTreeMap<Key, Expanded>,
) -> Result<Expanded, ContractError> {
    if depth >= MAX_REFERENCE_DEPTH {
        return Err(ContractError::ReferenceDepth(key.clone()));
    }
    if visiting.contains(key) {
        return Err(ContractError::ReferenceCycle(key.clone()));
    }
    if let Some(cached) = memo.get(key) {
        if depth.saturating_add(cached.height) > MAX_REFERENCE_DEPTH {
            return Err(ContractError::ReferenceDepth(key.clone()));
        }
        return Ok(Expanded {
            shape: cached.shape.clone(),
            height: cached.height,
        });
    }
    let _new = visiting.insert(key.clone());
    let result = expand_references(key, depth, lookup, visiting, memo)?;
    let _removed = visiting.remove(key);
    let _old = memo.insert(key.clone(), result.clone());
    Ok(result)
}

fn expand_references<'a>(
    key: &Key,
    depth: usize,
    lookup: &impl Fn(&Key) -> Option<&'a Shape>,
    visiting: &mut BTreeSet<Key>,
    memo: &mut BTreeMap<Key, Expanded>,
) -> Result<Expanded, ContractError> {
    let mut shape = lookup(key)
        .cloned()
        .ok_or_else(|| ContractError::MissingReference(key.clone()))?;
    let mut height: usize = 1;
    for reference in &shape.references.clone() {
        // Saturation rejects excessive depth instead of wrapping it into range.
        let referenced = expand(
            &reference.key,
            depth.saturating_add(1),
            lookup,
            visiting,
            memo,
        )?;
        height = height.max(referenced.height.saturating_add(1));
        shape.conflicts.extend(referenced.shape.conflicts);
        for (name, kind) in referenced.shape.variables {
            if let Some(literal) = reference.bound.get(&name) {
                check_binding(&reference.key, &name, &kind, *literal)?;
            } else {
                match &reference.key {
                    Key::Term(_) => {
                        return Err(ContractError::UnboundTerm {
                            key: reference.key.clone(),
                            name,
                        });
                    }
                    Key::Message(_) => shape.merge(name, kind),
                }
            }
        }
    }
    Ok(Expanded { shape, height })
}

fn check_binding(
    key: &Key,
    name: &super::Name,
    kind: &super::Kind,
    literal: LocalLiteral,
) -> Result<(), ContractError> {
    let required = match (kind, literal) {
        (super::Kind::Date, LocalLiteral::Text | LocalLiteral::Number) => {
            Some(BindingRequirement::CallerDate)
        }
        (super::Kind::Number, LocalLiteral::Text) => Some(BindingRequirement::Number),
        (super::Kind::Text | super::Kind::Choice(_), LocalLiteral::Text | LocalLiteral::Number)
        | (super::Kind::Number, LocalLiteral::Number) => None,
    };
    if let Some(required) = required {
        return Err(ContractError::BindingType {
            key: key.clone(),
            name: name.clone(),
            required,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
