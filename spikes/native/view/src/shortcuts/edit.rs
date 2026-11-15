//! Changing a shortcut (the page's `data/shortcuts/edit.ts`): which keys
//! a person may record, what a new key clashes with, and the edits that
//! give an action new keys, take a key from another, remove one or
//! restore the defaults.
//!
//! [Decision Log] The page's refusals, kept on every surface (ADR 0036)
//! - Purpose: a key recorded in the native app must also be recordable
//!   on the page, since both keep the same overrides.
//! - Choice: the page's rules unchanged: a chord needs Control, Alt or
//!   Command; Command (Control elsewhere) alone with N, T, W, Q, Tab or a
//!   digit is kept by the browser and the system; with A, C, V, X or Z it
//!   is the editing keys'. The native app's sentence names the system
//!   rather than the browser (`text/shortcuts.rs`).
//! - The edits are the preferences' own (`prefs::ShortcutEdit`, what the
//!   bridge's `ShortcutEdit` is), kept through
//!   `PreferenceChange::Shortcuts`; restoring every action is
//!   `PreferenceChange::ResetShortcuts`.

use std::collections::BTreeMap;

use super::keys::{Binding, Chord, Platform, overlaps};
use super::registry::{Action, bindings_of, jump_of};
use crate::prefs::{ActionId, Bindings, KeyBinding, MAX_BINDINGS, ShortcutEdit, ShortcutTarget};

/// Why a recorded chord is not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// It holds no modifier that makes it a command.
    NoModifier(Platform),
    /// The browser or the system keeps it.
    Kept,
    /// It is a standard editing key (select all, copy, paste, cut,
    /// undo).
    Editing,
}

/// Why `chord` cannot be recorded on `platform`, if it cannot.
#[must_use]
pub fn refusal(chord: &Chord, platform: Platform) -> Option<Refusal> {
    if !(chord.primary || chord.ctrl || chord.alt || chord.meta) {
        return Some(Refusal::NoModifier(platform));
    }
    let primary = chord.primary && !chord.ctrl && !chord.alt && !chord.meta;
    let key = chord.key.as_str();
    if primary && ["N", "T", "W", "Q", "Tab"].contains(&key) {
        return Some(Refusal::Kept);
    }
    let digit = key.len() == 1 && key.chars().all(|one| ('1'..='9').contains(&one));
    if primary && !chord.shift && digit {
        return Some(Refusal::Kept);
    }
    if primary && !chord.shift && ["A", "C", "V", "X", "Z"].contains(&key) {
        return Some(Refusal::Editing);
    }
    None
}

/// What a binding would clash with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    /// It is already one of this action's keys.
    Same(Action),
    /// It is a fixed key (Alt and a digit).
    Fixed(Action),
    /// Another action has it (or a sequence that one would hide).
    Taken(Action),
}

/// What `binding`, given to `action`, clashes with now; `None` if
/// nothing. A sequence that starts with the binding (or the reverse)
/// clashes too: one would hide the other.
#[must_use]
pub fn clash_of(
    action: Action,
    binding: &Binding,
    overrides: &BTreeMap<ActionId, Bindings>,
    platform: Platform,
) -> Option<Clash> {
    if binding.then.is_none() && jump_of(&binding.first).is_some() {
        return Some(Clash::Fixed(Action::JumpSession));
    }
    Action::ALL
        .into_iter()
        .filter(|other| !other.fixed())
        .find(|other| {
            bindings_of(*other, overrides)
                .iter()
                .any(|existing| overlaps(existing, binding, platform))
        })
        .map(|other| {
            if other == action {
                Clash::Same(other)
            } else {
                Clash::Taken(other)
            }
        })
}

/// The edit that gives `action` exactly `bindings` (none: unbound).
pub(super) fn set_keys(action: Action, bindings: &[Binding]) -> Option<ShortcutEdit> {
    let keys: Vec<KeyBinding> = bindings
        .iter()
        .take(MAX_BINDINGS)
        .filter_map(|binding| binding.text().parse().ok())
        .collect();
    Some(ShortcutEdit {
        action: action.action_id()?,
        to: ShortcutTarget::Bindings {
            keys: Bindings::try_from(keys).ok()?,
        },
    })
}

/// The edits that add `binding` to `action` (`replace`: in place of the
/// key at that index, else as another key; the oldest key goes when it
/// already has two), taking it from `from` when it was another action's.
#[must_use]
pub fn assign(
    action: Action,
    binding: &Binding,
    overrides: &BTreeMap<ActionId, Bindings>,
    replace: Option<usize>,
    from: Option<Action>,
    platform: Platform,
) -> Vec<ShortcutEdit> {
    let current = bindings_of(action, overrides);
    let next: Vec<Binding> = match replace {
        None => {
            let mut next = current;
            next.push(binding.clone());
            let extra = next.len().saturating_sub(MAX_BINDINGS);
            next.split_off(extra)
        }
        Some(at) => current
            .into_iter()
            .enumerate()
            .map(|(index, existing)| {
                if index == at {
                    binding.clone()
                } else {
                    existing
                }
            })
            .collect(),
    };
    let taken = from.filter(|from| *from != action).map(|from| {
        let left: Vec<Binding> = bindings_of(from, overrides)
            .into_iter()
            .filter(|existing| !overlaps(existing, binding, platform))
            .collect();
        (from, left)
    });
    std::iter::once(set_keys(action, &next))
        .chain(taken.map(|(from, left)| set_keys(from, &left)))
        .flatten()
        .collect()
}

/// The edits that remove the key at `index` from `action`.
#[must_use]
pub fn remove_key(
    action: Action,
    index: usize,
    overrides: &BTreeMap<ActionId, Bindings>,
) -> Vec<ShortcutEdit> {
    let left: Vec<Binding> = bindings_of(action, overrides)
        .into_iter()
        .enumerate()
        .filter(|(at, _)| *at != index)
        .map(|(_, binding)| binding)
        .collect();
    set_keys(action, &left).into_iter().collect()
}

/// The edit that gives `action` its defaults again.
#[must_use]
pub fn restore(action: Action) -> Vec<ShortcutEdit> {
    action
        .action_id()
        .map(|action| ShortcutEdit {
            action,
            to: ShortcutTarget::Default,
        })
        .into_iter()
        .collect()
}
