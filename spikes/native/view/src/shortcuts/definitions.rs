//! The registry's validated binding definitions and one override projection.
//! Help, matching, conflict detection and hints must all read this projection;
//! an invalid persisted combo falls back to its definition and is reported.

use std::collections::BTreeMap;

use super::context::When;
use super::engine::{Consume, EffectiveBinding, Payload, Repeat};
use super::sequence::KeySequence;
use crate::prefs::{ActionId, Bindings};

/// One canonical binding, not an action's platform-independent key aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition<A> {
    /// Stable persistence id, retained even if the default key changes.
    pub id: ActionId,
    /// Feature action shared with command-center contributions.
    pub action: A,
    /// An explicit unbound default remains available in Settings.
    pub default_keys: Option<KeySequence>,
    /// Action payload shape survives a user override.
    pub payload: Payload,
    /// Default predicate, with only its editable guard dropped by an override.
    pub when: When,
    /// Per-press or repeating behavior.
    pub repeat: Repeat,
    /// Native default policy.
    pub prevent_default: Consume,
    /// Propagation policy.
    pub stop_propagation: Consume,
}

/// Effective entries plus any rejected persisted ids, without leaking file text.
#[derive(Debug)]
pub struct EffectiveSet<A> {
    /// Exactly the entries used by matching and shortcut hints.
    pub bindings: Vec<EffectiveBinding<A>>,
    /// A caller may surface a safe preference warning; the default still works.
    pub rejected: Vec<ActionId>,
}

/// Apply overrides once. Empty lists explicitly unassign; absent ids use the
/// default; unparseable stored text retains the default and records a refusal.
#[must_use]
pub fn build_effective<A: Clone>(
    definitions: &[Definition<A>],
    overrides: &BTreeMap<ActionId, Bindings>,
) -> EffectiveSet<A> {
    let mut result = EffectiveSet {
        bindings: Vec::new(),
        rejected: Vec::new(),
    };
    for definition in definitions {
        let keys = overrides.get(&definition.id).map(|keys| {
            keys.keys()
                .iter()
                .map(|key| KeySequence::parse(key.as_str()))
                .collect::<Result<Vec<_>, _>>()
        });
        let (keys, when) = match keys {
            Some(Ok(keys)) => (keys, definition.when.overridden()),
            Some(Err(_)) => {
                result.rejected.push(definition.id.clone());
                (
                    definition.default_keys.iter().cloned().collect(),
                    definition.when,
                )
            }
            None => (
                definition.default_keys.iter().cloned().collect(),
                definition.when,
            ),
        };
        result
            .bindings
            .extend(keys.into_iter().map(|keys| EffectiveBinding {
                id: definition.id.clone(),
                action: definition.action.clone(),
                keys,
                when,
                payload: definition.payload,
                repeat: definition.repeat,
                prevent_default: definition.prevent_default,
                stop_propagation: definition.stop_propagation,
            }));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::KeyBinding;
    use crate::shortcuts::context::{Exclusion, HostKind};

    #[test]
    fn a_binding_override_unassign_and_reset_share_the_matching_projection() {
        let id: ActionId = "palette-platform".parse().unwrap();
        let definitions = [Definition {
            id: id.clone(),
            action: 1,
            default_keys: Some(KeySequence::parse("Mod+K").unwrap()),
            payload: Payload::None,
            when: When::ANY.desktop().outside_editing().outside_terminal(),
            repeat: Repeat::Deny,
            prevent_default: Consume::Yes,
            stop_propagation: Consume::Yes,
        }];
        let mut overrides = BTreeMap::new();
        assert_eq!(
            build_effective(&definitions, &overrides).bindings[0]
                .keys
                .text(),
            "Mod+K"
        );
        drop(overrides.insert(
            id.clone(),
            Bindings::try_from(vec!["Mod+J".parse::<KeyBinding>().unwrap()]).unwrap(),
        ));
        let effective = build_effective(&definitions, &overrides);
        assert_eq!(effective.bindings[0].keys.text(), "Mod+J");
        assert_eq!(effective.bindings[0].when.editable, Exclusion::Permit);
        assert_eq!(effective.bindings[0].when.terminal, Exclusion::Exclude);
        assert_eq!(effective.bindings[0].when.host, Some(HostKind::Desktop));
        drop(overrides.insert(id.clone(), Bindings::default()));
        assert!(
            build_effective(&definitions, &overrides)
                .bindings
                .is_empty()
        );
        drop(overrides.remove(&id));
        assert_eq!(
            build_effective(&definitions, &overrides).bindings[0]
                .keys
                .text(),
            "Mod+K"
        );
    }
}
