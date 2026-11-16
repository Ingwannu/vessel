//! Atomic Settings edits share the engine's effective snapshot. Reassignment
//! removes only overlapping sequences and preserves another row's alternatives.
//! Migrating aliases in the same transaction prevents a reset from resurrecting
//! an old action-level override underneath the canonical binding id.

use std::collections::{BTreeMap, BTreeSet};

use super::context::ShortcutContext;
use super::engine::EffectiveBinding;
use super::sequence::KeySequence;
use super::sequence_capture::sequences_overlap;
use super::{Action, migrated_overrides};
use crate::prefs::{ActionId, Bindings, InvalidKeyBinding, ShortcutEdit, ShortcutTarget, TooMany};

/// A valid capture can still exceed a preference transaction's bound.
#[derive(Debug, thiserror::Error)]
pub enum RebindError {
    /// A caller supplied a sequence outside the shared preference grammar.
    #[error(transparent)]
    Keys(#[from] InvalidKeyBinding),
    /// Refuse the complete transaction rather than silently discard an alternative.
    #[error(transparent)]
    Bound(#[from] TooMany),
}

/// Prepare one explicit replacement, clear or reset, including legacy migration.
///
/// # Errors
/// Returns a typed refusal if a canonical sequence cannot be persisted.
pub fn edit(
    raw: &BTreeMap<ActionId, Bindings>,
    context: ShortcutContext,
    id: &ActionId,
    target: Option<&KeySequence>,
    reset: bool,
) -> Result<Vec<ShortcutEdit>, RebindError> {
    let mut next = migrated_overrides(raw, context);
    if reset {
        drop(next.remove(id));
    } else {
        drop(next.insert(id.clone(), saved(target.into_iter())?));
    }
    Ok(difference(raw, &next))
}

/// Assign after confirmation against the latest snapshot, clearing every supplied
/// conflict and retaining its non-overlapping alternatives. The UI must compare
/// the reviewed conflict identities before committing a changed snapshot.
///
/// # Errors
/// Refuses invalid or oversized persistence data without partially applying it.
pub fn reassign(
    raw: &BTreeMap<ActionId, Bindings>,
    context: ShortcutContext,
    id: &ActionId,
    keys: &KeySequence,
    conflicts: &[ActionId],
    bindings: &[EffectiveBinding<Action>],
) -> Result<Vec<ShortcutEdit>, RebindError> {
    let mut next = migrated_overrides(raw, context);
    for conflict in conflicts {
        let retained = bindings
            .iter()
            .filter(|binding| {
                &binding.id == conflict && !sequences_overlap(keys, &binding.keys, context.platform)
            })
            .map(|binding| &binding.keys);
        drop(next.insert(conflict.clone(), saved(retained)?));
    }
    drop(next.insert(id.clone(), saved(std::iter::once(keys))?));
    Ok(difference(raw, &next))
}

fn saved<'a>(keys: impl Iterator<Item = &'a KeySequence>) -> Result<Bindings, RebindError> {
    let keys = keys
        .map(|keys| keys.text().parse())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Bindings::try_from(keys)?)
}

fn difference(
    raw: &BTreeMap<ActionId, Bindings>,
    next: &BTreeMap<ActionId, Bindings>,
) -> Vec<ShortcutEdit> {
    let ids: BTreeSet<_> = raw.keys().chain(next.keys()).collect();
    ids.into_iter()
        .filter(|id| raw.get(*id) != next.get(*id))
        .map(|id| ShortcutEdit {
            action: id.clone(),
            to: next
                .get(id)
                .map_or(ShortcutTarget::Default, |keys| ShortcutTarget::Bindings {
                    keys: keys.clone(),
                }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::{PreferenceChange, Preferences, ShortcutEdits};
    use crate::shortcuts::{
        Platform, binding_id, default_context, effective_bindings, primary_keys,
    };

    fn apply(
        raw: BTreeMap<ActionId, Bindings>,
        edits: Vec<ShortcutEdit>,
    ) -> BTreeMap<ActionId, Bindings> {
        let prefs = Preferences {
            shortcuts: raw,
            ..Preferences::default()
        };
        prefs
            .changed(&PreferenceChange::Shortcuts {
                edits: ShortcutEdits::try_from(edits).unwrap(),
            })
            .unwrap()
            .shortcuts
    }

    #[test]
    fn resetting_a_migrated_binding_does_not_resurrect_its_legacy_override() {
        let context = default_context(Platform::Other);
        let id = binding_id(Action::Palette, context).unwrap();
        let raw = BTreeMap::from([(
            "palette".parse().unwrap(),
            Bindings::try_from(vec!["Mod+J".parse().unwrap()]).unwrap(),
        )]);
        let next = apply(raw.clone(), edit(&raw, context, &id, None, true).unwrap());
        assert!(next.is_empty());
        assert_eq!(
            primary_keys(Action::Palette, &next, context)
                .unwrap()
                .unwrap()
                .text(),
            "Ctrl+K"
        );
    }

    #[test]
    fn reassign_clears_all_overlapping_prefixes_and_preserves_other_alternatives() {
        let context = default_context(Platform::Other);
        let palette = binding_id(Action::Palette, context).unwrap();
        let settings = binding_id(Action::Settings, context).unwrap();
        let find = binding_id(Action::Find, context).unwrap();
        let raw = BTreeMap::from([
            (
                palette.clone(),
                Bindings::try_from(vec!["Mod+J".parse().unwrap(), "Mod+L".parse().unwrap()])
                    .unwrap(),
            ),
            (
                settings.clone(),
                Bindings::try_from(vec!["Mod+J S".parse().unwrap()]).unwrap(),
            ),
        ]);
        let bindings = effective_bindings(&raw, context).unwrap();
        let keys = KeySequence::parse("Mod+J").unwrap();
        let edits = reassign(
            &raw,
            context,
            &find,
            &keys,
            &[palette.clone(), settings.clone()],
            &bindings,
        )
        .unwrap();
        let next = apply(raw, edits);
        assert_eq!(next[&palette].keys()[0].as_str(), "Mod+L");
        assert!(next[&settings].keys().is_empty());
        assert_eq!(next[&find].keys()[0].as_str(), "Mod+J");
    }
}
