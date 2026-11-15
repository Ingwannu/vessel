//! Recording a key in Settings › Shortcuts, as Paseo's page records one
//! (`screens/settings/keyboard-shortcuts-section.tsx`): a row starts
//! recording from its menu; each press is shown as it is held; Backspace
//! with no modifier takes the last press back; Done saves; Cancel, leaving
//! the page or recording another row drops it.
//!
//! [Decision Log]
//! - Purpose: one place decides what a recorded key does to the person's
//!   keys, so the page draws and nothing else decides.
//! - Constraint: Paseo's recorder saves whatever is pressed; the page's
//!   (`features/settings/shortcuts-section.tsx`) refuses a key the system
//!   keeps or one with no modifier, and asks before taking a key from
//!   another action. Both surfaces keep the same overrides (ADR 0036), so
//!   a key recorded here must be one the page accepts.
//! - Choice: Paseo's flow (press, see it, Done or Cancel; Rebind replaces
//!   the action's keys whole; Clear unbinds; Reset restores the default;
//!   Reset all at the first section's head) with the page's rules checked
//!   on each press: a refused or clashing key is shown with its reason and
//!   Done stays off; a key another action has asks to reassign it at Done.
//! - One chord per recording: the page records one (its sequences, `G S`,
//!   are defaults only, since a chord without a modifier is refused), so
//!   a second press replaces the first instead of starting a sequence.
//! - Cost: an action with two keys (the palette's Mod+K and `/`) keeps one
//!   after a rebind, as Paseo's one binding per row does.

use std::collections::BTreeMap;

use super::edit::{Clash, Refusal, clash_of, refusal, restore, set_keys};
use super::keys::{Binding, Chord, Platform};
use super::registry::Action;
use crate::prefs::{ActionId, Bindings, ShortcutEdit};

/// A row recording a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// The action whose keys are being recorded.
    pub action: Action,
    /// The chord pressed, once one is.
    pub chord: Option<Chord>,
}

/// What a recorded chord would do, checked as it is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing pressed yet: the prompt shows.
    Waiting,
    /// Done saves it.
    Ready,
    /// Done saves it after asking: another action has the key.
    Reassign(Action),
    /// The key cannot be used here; Done stays off.
    Refused(Refusal),
    /// It is this action's key already, or a fixed one; Done stays off.
    Clashes(Clash),
}

/// What pressing Done does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// Nothing to save (nothing pressed, or a refused key).
    Nothing,
    /// Save these edits.
    Save(Vec<ShortcutEdit>),
    /// Ask first: the key belongs to `from`; saving takes it from there.
    Ask {
        /// Who has the key.
        from: Action,
        /// The edits that move it, once the person agrees.
        edits: Vec<ShortcutEdit>,
    },
}

impl Capture {
    /// Start recording `action`'s key; `None` for a fixed action, whose
    /// keys cannot change.
    #[must_use]
    pub const fn start(action: Action) -> Option<Self> {
        if action.fixed() {
            return None;
        }
        Some(Self {
            action,
            chord: None,
        })
    }

    /// A press while recording (a chord with its key: a modifier held
    /// alone is not a press and is shown by the page as held).
    pub fn press(&mut self, chord: Chord) {
        self.chord = Some(chord);
    }

    /// Backspace with no modifier: take the press back.
    pub fn back(&mut self) {
        self.chord = None;
    }

    /// What the chord pressed would do now.
    #[must_use]
    pub fn verdict(&self, overrides: &BTreeMap<ActionId, Bindings>, platform: Platform) -> Verdict {
        let Some(chord) = &self.chord else {
            return Verdict::Waiting;
        };
        if let Some(refused) = refusal(chord, platform) {
            return Verdict::Refused(refused);
        }
        match clash_of(self.action, &binding(chord), overrides, platform) {
            None => Verdict::Ready,
            Some(Clash::Taken(from)) => Verdict::Reassign(from),
            Some(clash @ (Clash::Same(_) | Clash::Fixed(_))) => Verdict::Clashes(clash),
        }
    }

    /// Pressing Done: the edits that give the action this key alone
    /// (Paseo's rebind replaces the binding whole), taking it from the
    /// action that has it.
    #[must_use]
    pub fn done(&self, overrides: &BTreeMap<ActionId, Bindings>, platform: Platform) -> Done {
        let Some(chord) = &self.chord else {
            return Done::Nothing;
        };
        let chosen = binding(chord);
        match self.verdict(overrides, platform) {
            Verdict::Waiting | Verdict::Refused(_) | Verdict::Clashes(_) => Done::Nothing,
            Verdict::Ready => Done::Save(set_keys(self.action, &[chosen]).into_iter().collect()),
            Verdict::Reassign(from) => {
                let left: Vec<Binding> = super::registry::bindings_of(from, overrides)
                    .into_iter()
                    .filter(|existing| !super::keys::overlaps(existing, &chosen, platform))
                    .collect();
                let edits = set_keys(self.action, &[chosen])
                    .into_iter()
                    .chain(set_keys(from, &left))
                    .collect();
                Done::Ask { from, edits }
            }
        }
    }
}

/// The edits that leave `action` with no key (Paseo's Clear: "Not set").
#[must_use]
pub fn clear(action: Action) -> Vec<ShortcutEdit> {
    set_keys(action, &[]).into_iter().collect()
}

/// The edits that give `action` its default keys again (Paseo's Reset).
#[must_use]
pub fn reset(action: Action) -> Vec<ShortcutEdit> {
    restore(action)
}

fn binding(chord: &Chord) -> Binding {
    Binding {
        first: chord.clone(),
        then: None,
    }
}
