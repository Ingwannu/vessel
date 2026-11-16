//! Multi-step recording and conflict projection for the existing Settings page.
//! Recording never mutates preferences: the UI can review every conflicting
//! binding before committing one atomic edit through the existing preference path.

use super::context::{Admission, CommandCenterState, FocusScope, HostKind, ShortcutContext, When};
use super::engine::{EffectiveBinding, ShortcutIndex};
use super::sequence::{KeySequence, SequenceError};
use super::{Binding, Chord, Platform, same_chord};
use crate::prefs::ActionId;

/// Index actions normalize each captured digit to the 1–9 wildcard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRole {
    /// Any valid chord may finish the sequence.
    Ordinary,
    /// A final digit is needed to supply a navigation index.
    Index,
}

/// The capture retains the attempted failure so a bound is never a silent drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptureFault {
    /// Native editing remains available on every surface and in preference files.
    #[error("shortcut is reserved for native editing")]
    Editing,
    /// Quit and app-switch accelerators remain with the native host.
    #[error("shortcut is reserved by the host")]
    HostReserved,
    /// The existing bounded sequence grammar refused this recording.
    #[error(transparent)]
    Sequence(#[from] SequenceError),
}

/// Every visible capture state is explicit and independent of the UI toolkit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureVerdict {
    /// No chord has been recorded yet.
    Waiting,
    /// Index actions cannot save a sequence ending in a letter or another key.
    IndexRequired,
    /// The attempted last press was refused, retaining the prior capture.
    Refused(CaptureFault),
    /// Ready to review and save; conflicts include all overlapping bindings.
    Ready {
        /// Canonical sequence, including digit wildcard normalization.
        keys: KeySequence,
        /// Every conflicting binding, in the current registry's order.
        conflicts: Vec<ActionId>,
    },
}

/// One Settings row records while the one global shortcut listener stands down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceCapture {
    /// Canonical binding id rather than an action-level alias.
    pub binding: ActionId,
    role: CaptureRole,
    steps: Vec<Chord>,
    fault: Option<CaptureFault>,
}

impl SequenceCapture {
    /// Start an ordinary or digit-index recording, dropping no other row's state.
    #[must_use]
    pub const fn new(binding: ActionId, role: CaptureRole) -> Self {
        Self {
            binding,
            role,
            steps: Vec::new(),
            fault: None,
        }
    }

    /// Capture a key. The interceptor has already excluded composing/`AltGr` input.
    /// Bare Backspace is handled by `back`; Escape is recordable, with a visible
    /// Cancel button and leaving the page providing cancellation, as in Paseo.
    pub fn press(&mut self, mut chord: Chord, context: ShortcutContext) {
        match context.admission {
            Admission::Available | Admission::Capturing => {}
            Admission::Composing
            | Admission::CharacterInput
            | Admission::Inactive
            | Admission::Overlay => return,
        }
        self.fault = capture_refusal(&chord, context);
        if self.fault.is_some() {
            return;
        }
        if self.role == CaptureRole::Index && is_digit(&chord.key) {
            "Digit".clone_into(&mut chord.key);
        }
        let mut next = self.steps.clone();
        next.push(chord);
        let text = capture_text(&next);
        match KeySequence::parse(&text) {
            Ok(_) => self.steps = next,
            Err(error) => self.fault = Some(error.into()),
        }
    }

    /// Bare Backspace removes the last accepted combo and clears a refusal.
    pub fn back(&mut self) {
        drop(self.steps.pop());
        self.fault = None;
    }

    /// Accepted chords, used for the same key caps as matching and help.
    pub fn chords(&self) -> impl Iterator<Item = &Chord> {
        self.steps.iter()
    }

    /// Evaluate against the current snapshot, so a passive registry/override
    /// change cannot make a previously prepared reassignment take a new key.
    #[must_use]
    pub fn verdict<A>(
        &self,
        when: When,
        context: ShortcutContext,
        bindings: &[EffectiveBinding<A>],
    ) -> CaptureVerdict {
        if let Some(fault) = self.fault {
            return CaptureVerdict::Refused(fault);
        }
        let Some(last) = self.steps.last() else {
            return CaptureVerdict::Waiting;
        };
        if self.role == CaptureRole::Index && last.key != "Digit" {
            return CaptureVerdict::IndexRequired;
        }
        let keys = match KeySequence::parse(&capture_text(&self.steps)) {
            Ok(keys) => keys,
            Err(error) => return CaptureVerdict::Refused(error.into()),
        };
        let mut conflicts = Vec::new();
        for binding in bindings {
            if binding.id != self.binding
                && contexts_overlap(when.overridden(), binding.when, context)
                && sequences_overlap(&keys, &binding.keys, context.platform)
                && !conflicts.contains(&binding.id)
            {
                conflicts.push(binding.id.clone());
            }
        }
        CaptureVerdict::Ready { keys, conflicts }
    }
}

fn capture_text(steps: &[Chord]) -> String {
    steps
        .iter()
        .map(|chord| {
            Binding {
                first: chord.clone(),
                then: None,
            }
            .text()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capture_refusal(chord: &Chord, context: ShortcutContext) -> Option<CaptureFault> {
    if !super::keys::primary_command(chord, context.platform) {
        return None;
    }
    if super::keys::native_editing(chord, context.platform) {
        return Some(CaptureFault::Editing);
    }
    let host_reserved = chord.key == "Q"
        || (context.platform == Platform::Mac && chord.key == "Tab")
        || (context.host == HostKind::Web
            && (["N", "T", "W", "Tab"].contains(&chord.key.as_str()) || is_digit(&chord.key)));
    host_reserved.then_some(CaptureFault::HostReserved)
}

fn is_digit(key: &str) -> bool {
    ShortcutIndex::from_key(key).is_some()
}

fn contexts_overlap(left: When, right: When, context: ShortcutContext) -> bool {
    [
        FocusScope::Other,
        FocusScope::MessageInput,
        FocusScope::Editable,
        FocusScope::Terminal,
        FocusScope::Browser,
        FocusScope::CommandCenter,
    ]
    .into_iter()
    .any(|focus| {
        [CommandCenterState::Open, CommandCenterState::Closed]
            .into_iter()
            .any(|command_center| {
                let context = ShortcutContext {
                    focus,
                    command_center,
                    admission: Admission::Available,
                    ..context
                };
                left.matches(context) && right.matches(context)
            })
    })
}

/// Identical sequences and either-direction prefixes conflict; concrete digits
/// also conflict with a wildcard, after modifiers resolve on the current host.
#[must_use]
pub fn sequences_overlap(left: &KeySequence, right: &KeySequence, platform: Platform) -> bool {
    left.chords().zip(right.chords()).all(|(left, right)| {
        if left.key == "Digit" && is_digit(&right.key) {
            let mut template = left.clone();
            template.key.clone_from(&right.key);
            same_chord(&template, right, platform)
        } else if right.key == "Digit" && is_digit(&left.key) {
            let mut template = right.clone();
            template.key.clone_from(&left.key);
            same_chord(left, &template, platform)
        } else {
            same_chord(left, right, platform)
        }
    })
}

#[cfg(test)]
mod tests;
