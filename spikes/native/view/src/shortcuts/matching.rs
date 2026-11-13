//! Matching a key press to an action (the page's
//! `data/shortcuts/match.ts`): a pure function of the press, the chord
//! pressed before it, the overrides and the time, so the one listener is
//! a thin layer over it.
//!
//! [Decision Log] Sequences wait 1.5 seconds, overrides win (milestone 11)
//! - Purpose: `G S` must run whether the second key comes quickly or
//!   slowly, and a person's own key must beat a default it collides with.
//! - Choice: a first chord that begins a sequence waits up to 1.5 seconds
//!   for its second; a completed sequence comes before a single chord;
//!   Alt and a digit is always "session N"; actions the person changed
//!   are tried before the others.

use std::collections::BTreeMap;

use super::keys::{Chord, Platform, same_chord};
use super::registry::{Action, bindings_of, jump_of, modified};
use crate::prefs::{ActionId, Bindings};

/// How long a sequence's first chord waits for its second, in
/// milliseconds.
pub const SEQUENCE_MS: u64 = 1500;

/// A sequence's first chord, and when it was pressed (milliseconds on the
/// caller's clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The chord.
    pub chord: Chord,
    /// When.
    pub at: u64,
}

/// What a press does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matched {
    /// Run an action; `index` is the session for "session N".
    Run {
        /// The action.
        action: Action,
        /// Session 1 to 9, for [`Action::JumpSession`].
        index: Option<u8>,
    },
    /// The press begins a sequence: keep it and wait for the next.
    Wait(Pending),
    /// Nothing is bound to it.
    None,
}

/// Every binding with its action, the person's changed actions first.
fn every(overrides: &BTreeMap<ActionId, Bindings>) -> Vec<(Action, super::keys::Binding)> {
    let changed = Action::ALL
        .into_iter()
        .filter(|action| modified(*action, overrides));
    let others = Action::ALL
        .into_iter()
        .filter(|action| !modified(*action, overrides));
    changed
        .chain(others)
        .flat_map(|action| {
            bindings_of(action, overrides)
                .into_iter()
                .map(move |binding| (action, binding))
        })
        .collect()
}

/// What `chord`, pressed at `now` after `pending`, does.
#[must_use]
pub fn matched(
    chord: &Chord,
    pending: Option<&Pending>,
    overrides: &BTreeMap<ActionId, Bindings>,
    now: u64,
    platform: Platform,
) -> Matched {
    let bound = every(overrides);
    if let Some(pending) = pending.filter(|pending| now.saturating_sub(pending.at) <= SEQUENCE_MS) {
        let completes = bound.iter().find(|(_, binding)| {
            binding.then.as_ref().is_some_and(|then| {
                same_chord(&binding.first, &pending.chord, platform)
                    && same_chord(then, chord, platform)
            })
        });
        if let Some((action, _)) = completes {
            return Matched::Run {
                action: *action,
                index: None,
            };
        }
    }
    if let Some(index) = jump_of(chord) {
        return Matched::Run {
            action: Action::JumpSession,
            index: Some(index),
        };
    }
    let single = bound
        .iter()
        .find(|(_, binding)| binding.then.is_none() && same_chord(&binding.first, chord, platform));
    if let Some((action, _)) = single {
        return Matched::Run {
            action: *action,
            index: None,
        };
    }
    let starts = bound
        .iter()
        .any(|(_, binding)| binding.then.is_some() && same_chord(&binding.first, chord, platform));
    if starts {
        Matched::Wait(Pending {
            chord: chord.clone(),
            at: now,
        })
    } else {
        Matched::None
    }
}
