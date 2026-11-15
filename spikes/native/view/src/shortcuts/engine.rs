//! Stateful chord resolution for the existing shortcut registry. Only admitted
//! input advances candidates; every step rechecks current focus. The owner
//! replaces this state on blur, visibility loss, capture and binding changes.

use std::time::Duration;

use super::context::{Admission, FocusScope, ShortcutContext, When};
use super::input::{InputConversion, ShortcutInput};
use super::sequence::KeySequence;
use super::{Chord, KeyPress};
use crate::prefs::ActionId;

/// Paseo's fixed timeout, measured from the most recent accepted chord step.
pub const CHORD_TIMEOUT: Duration = Duration::from_millis(CHORD_TIMEOUT_MS);
/// Reference timeout in milliseconds for compatibility callers.
pub const CHORD_TIMEOUT_MS: u64 = 1500;

/// Keyboard repeat is explicit because sending/voice/mode changes act once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    /// Held keys may repeat navigation.
    Allow,
    /// Only an initial keydown may perform this action.
    Deny,
}

/// The host reports actual repeat separately from a binding's repeat policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRepeat {
    /// A new keydown after release.
    Initial,
    /// The system repeated a held key.
    Repeated,
}

/// Native editing and agent interruption have explicit propagation semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consume {
    /// Leave the event's default or propagation intact.
    No,
    /// Consume the event's default or propagation once actually handled.
    Yes,
}

/// Payload shape belongs to the action definition, even after rebinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload {
    /// Ordinary action with no digit argument.
    None,
    /// Extract the pressed 1–9 digit, including a concrete-digit override.
    Index,
}

/// A validated effective binding emitted by the one registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBinding<A> {
    /// Stable persistence identity, independent of the current combo.
    pub id: ActionId,
    /// Typed feature action.
    pub action: A,
    /// Original action payload definition, retained when keys change.
    pub payload: Payload,
    /// A nonempty validated sequence.
    pub keys: KeySequence,
    /// Platform and focus predicate after applying an override.
    pub when: When,
    /// Last-step repeat behavior.
    pub repeat: Repeat,
    /// Whether a handled action consumes the native default.
    pub prevent_default: Consume,
    /// Whether a handled action stops further propagation.
    pub stop_propagation: Consume,
}

/// Digits are validated once before becoming navigation payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutIndex(u8);

const MIN_SHORTCUT_INDEX: u8 = 1;
const MAX_SHORTCUT_INDEX: u8 = 9;

impl ShortcutIndex {
    /// A workspace/tab number, restricted to the reference's 1–9 range.
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value >= MIN_SHORTCUT_INDEX && value <= MAX_SHORTCUT_INDEX {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Parse the one digit carried by an actual key press, never a template.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        let mut characters = key.chars();
        let digit = characters.next()?.to_digit(10)?;
        if characters.next().is_some() {
            return None;
        }
        u8::try_from(digit).ok().and_then(Self::new)
    }

    /// One-based index; conversion to a collection offset belongs to its owner.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A press's outcome; waiting consumes the prefix without dispatching an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution<A> {
    /// Leave the press with the focused control.
    Pass,
    /// A multi-step candidate owns this prefix.
    Waiting,
    /// The feature's dispatcher decides whether it actually performed this.
    Run {
        /// Original typed action.
        action: A,
        /// Final key's digit, when the binding ends with the Digit wildcard.
        index: Option<ShortcutIndex>,
        /// Native default policy.
        prevent_default: Consume,
        /// Propagation policy.
        stop_propagation: Consume,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    id: ActionId,
    /// Changes to an override invalidate a pending candidate even if its id stays.
    keys: KeySequence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    candidates: Vec<Candidate>,
    step: usize,
    at: Duration,
}

/// Pure engine state; the app supplies monotonic elapsed time without I/O here.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Engine {
    pending: Option<Pending>,
}

impl Engine {
    /// Blur, binding changes, composition, capture and top overlays cancel prefixes.
    pub fn reset(&mut self) {
        self.pending = None;
    }

    /// Whether an admitted prefix awaits another step. Browser and UI policy
    /// can query this without exposing candidate ids or duplicating the state.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Resolve a platform key event with logical-first and permitted physical
    /// fallbacks. Modifier-only events preserve pending chords; IME cancels them.
    #[must_use]
    pub fn resolve_keypress<A: Clone>(
        &mut self,
        press: &KeyPress,
        repeat: KeyRepeat,
        now: Duration,
        context: ShortcutContext,
        bindings: &[EffectiveBinding<A>],
    ) -> Resolution<A> {
        match ShortcutInput::from_press(press, context.platform) {
            InputConversion::Input(input) => {
                self.resolve_input(&input, repeat, now, context, bindings)
            }
            InputConversion::Modifier => {
                if context.admission != Admission::Available {
                    self.reset();
                }
                Resolution::Pass
            }
            InputConversion::Composing | InputConversion::Unsupported => {
                self.reset();
                Resolution::Pass
            }
        }
    }

    /// Compatibility callers already have a normalized chord; matching still
    /// goes through the exact same state machine and context policy.
    #[must_use]
    pub fn resolve<A: Clone>(
        &mut self,
        chord: &Chord,
        repeat: KeyRepeat,
        now: Duration,
        context: ShortcutContext,
        bindings: &[EffectiveBinding<A>],
    ) -> Resolution<A> {
        self.resolve_input(
            &ShortcutInput::from_chord(chord.clone()),
            repeat,
            now,
            context,
            bindings,
        )
    }

    fn resolve_input<A: Clone>(
        &mut self,
        input: &ShortcutInput,
        repeat: KeyRepeat,
        now: Duration,
        context: ShortcutContext,
        bindings: &[EffectiveBinding<A>],
    ) -> Resolution<A> {
        if context.admission != Admission::Available
            || (matches!(
                context.focus,
                FocusScope::MessageInput | FocusScope::Editable | FocusScope::CommandCenter
            ) && input
                .editing_chord()
                .is_some_and(|chord| super::keys::native_editing(chord, context.platform)))
        {
            self.reset();
            return Resolution::Pass;
        }
        let pending = self
            .pending
            .take()
            .filter(|pending| now.saturating_sub(pending.at) < CHORD_TIMEOUT);
        let mut advancing = Vec::new();
        let step = pending.as_ref().map_or(0, |pending| pending.step);
        let mut completed = None;
        for binding in bindings {
            if !binding.when.matches(context) {
                continue;
            }
            if let Some(pending) = &pending
                && !pending
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == binding.id && candidate.keys == binding.keys)
            {
                continue;
            }
            let Some(wanted) = binding.keys.step(step) else {
                continue;
            };
            if !input.matches(wanted, context.platform) {
                continue;
            }
            if step.saturating_add(1) == binding.keys.len() {
                if repeat == KeyRepeat::Repeated && binding.repeat == Repeat::Deny {
                    continue;
                }
                if completed.is_none() {
                    completed = Some(binding);
                }
            } else {
                advancing.push(Candidate {
                    id: binding.id.clone(),
                    keys: binding.keys.clone(),
                });
            }
        }
        // A pending mismatch is consumed as a failed sequence, never retried as
        // a fresh shortcut: Paseo cannot accidentally fire a second-step command.
        if pending.is_some()
            && let Some(binding) = completed
        {
            return run(binding, input);
        }
        if !advancing.is_empty() {
            self.pending = Some(Pending {
                candidates: advancing,
                step: step.saturating_add(1),
                at: now,
            });
            return Resolution::Waiting;
        }
        completed.map_or(Resolution::Pass, |binding| run(binding, input))
    }
}

fn run<A: Clone>(binding: &EffectiveBinding<A>, input: &ShortcutInput) -> Resolution<A> {
    let index = match binding.payload {
        Payload::None => None,
        Payload::Index => input.index(),
    };
    Resolution::Run {
        action: binding.action.clone(),
        index,
        prevent_default: binding.prevent_default,
        stop_propagation: binding.stop_propagation,
    }
}

#[cfg(test)]
mod tests;
