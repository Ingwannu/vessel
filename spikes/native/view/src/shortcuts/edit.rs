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

use super::keys::{Binding, Chord, Platform, overlaps};
use super::registry::{Action, MAX_KEYS, Overrides, bindings_of, jump_of};

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
    overrides: &Overrides,
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

/// One action's keys after a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeysTo {
    /// These keys, as an override writes them.
    Bindings(Vec<String>),
    /// The action's defaults again.
    Default,
}

/// One action's keys after a change (the bridge's `ShortcutEdit`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutEdit {
    /// The action.
    pub action: Action,
    /// Its keys after it.
    pub to: KeysTo,
}

fn set_keys(action: Action, bindings: &[Binding]) -> ShortcutEdit {
    ShortcutEdit {
        action,
        to: KeysTo::Bindings(bindings.iter().take(MAX_KEYS).map(Binding::text).collect()),
    }
}

/// The edits that add `binding` to `action` (`replace`: in place of the
/// key at that index, else as another key; the oldest key goes when it
/// already has two), taking it from `from` when it was another action's.
#[must_use]
pub fn assign(
    action: Action,
    binding: &Binding,
    overrides: &Overrides,
    replace: Option<usize>,
    from: Option<Action>,
    platform: Platform,
) -> Vec<ShortcutEdit> {
    let current = bindings_of(action, overrides);
    let next: Vec<Binding> = match replace {
        None => {
            let mut next = current;
            next.push(binding.clone());
            let extra = next.len().saturating_sub(MAX_KEYS);
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
    let mut edits = vec![set_keys(action, &next)];
    if let Some(from) = from.filter(|from| *from != action) {
        let left: Vec<Binding> = bindings_of(from, overrides)
            .into_iter()
            .filter(|existing| !overlaps(existing, binding, platform))
            .collect();
        edits.push(set_keys(from, &left));
    }
    edits
}

/// The edits that remove the key at `index` from `action`.
#[must_use]
pub fn remove_key(action: Action, index: usize, overrides: &Overrides) -> Vec<ShortcutEdit> {
    let left: Vec<Binding> = bindings_of(action, overrides)
        .into_iter()
        .enumerate()
        .filter(|(at, _)| *at != index)
        .map(|(_, binding)| binding)
        .collect();
    vec![set_keys(action, &left)]
}

/// The edit that gives `action` its defaults again.
#[must_use]
pub fn restore(action: Action) -> Vec<ShortcutEdit> {
    vec![ShortcutEdit {
        action,
        to: KeysTo::Default,
    }]
}

/// The overrides after `edits` (what the page's bridge does when it keeps
/// them).
pub fn apply(edits: &[ShortcutEdit], overrides: &mut Overrides) {
    for edit in edits {
        match &edit.to {
            KeysTo::Bindings(keys) => {
                overrides
                    .entry(edit.action.id().to_owned())
                    .or_default()
                    .clone_from(keys);
            }
            KeysTo::Default => overrides.retain(|id, _| id != edit.action.id()),
        }
    }
}
