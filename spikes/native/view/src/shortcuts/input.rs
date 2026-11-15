//! Logical-key-first dispatch, preserving physical fallback only where Paseo
//! permits it: non-Latin letters, Option shortcuts, named keys and Digit groups.
//! A physical punctuation position never steals Dvorak's native copy/paste.

use super::engine::ShortcutIndex;
use super::keys::{logical_event_key, physical_key, shifted_key_matches};
use super::{Chord, KeyPress, Platform, same_chord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fallback {
    None,
    Letter,
    Alt,
}

/// A validated event keeps both layouts without a second physical-key binding.
#[derive(Debug, Clone)]
pub struct ShortcutInput {
    logical: Option<Chord>,
    physical: Option<Chord>,
    fallback: Fallback,
}

/// Modifier-only keys preserve a pending chord; composition cancels it.
#[derive(Debug, Clone)]
pub enum InputConversion {
    /// No action key was pressed.
    Modifier,
    /// The IME owns this event.
    Composing,
    /// No supported key is present.
    Unsupported,
    /// Ready for the one matching engine.
    Input(ShortcutInput),
}

impl ShortcutInput {
    /// Parse an external key event once at the platform boundary.
    #[must_use]
    pub fn from_press(press: &KeyPress, platform: Platform) -> InputConversion {
        if ["Process", "Dead"].contains(&press.key.as_str()) {
            return InputConversion::Composing;
        }
        if [
            "Shift", "Control", "Alt", "Meta", "Super", "Cmd", "ctrl", "alt", "shift", "cmd",
            "super", "control", "meta", "fn",
        ]
        .contains(&press.key.as_str())
            || [
                "ShiftLeft",
                "ShiftRight",
                "ControlLeft",
                "ControlRight",
                "AltLeft",
                "AltRight",
                "MetaLeft",
                "MetaRight",
            ]
            .contains(&press.code.as_str())
        {
            return InputConversion::Modifier;
        }
        let logical = logical_event_key(&press.key).map(|key| key.chord(press, platform));
        let physical = physical_key(&press.code).map(|key| key.chord(press, platform));
        if logical.is_none() && physical.is_none() {
            return InputConversion::Unsupported;
        }
        let mut characters = press.key.chars();
        let non_latin_letter = characters
            .next()
            .is_some_and(|character| character.is_alphabetic() && !character.is_ascii_alphabetic())
            && characters.next().is_none();
        let fallback = if press.alt {
            Fallback::Alt
        } else if non_latin_letter {
            Fallback::Letter
        } else {
            Fallback::None
        };
        InputConversion::Input(Self {
            logical,
            physical,
            fallback,
        })
    }

    /// Pure fixtures and the compatibility chord API already carry normalized keys.
    #[must_use]
    pub fn from_chord(chord: Chord) -> Self {
        Self {
            logical: Some(chord),
            physical: None,
            fallback: Fallback::None,
        }
    }

    pub(super) fn editing_chord(&self) -> Option<&Chord> {
        self.logical.as_ref()
    }

    /// The recorder keeps the same logical-first identity used by matching.
    /// Unsupported layout characters use the validated physical fallback.
    #[must_use]
    pub fn recording_chord(&self) -> Option<Chord> {
        let mut chord = self.logical.as_ref().or(self.physical.as_ref())?.clone();
        // Persist the base sign with Shift, as KEY_MAP's first logical match
        // does. Dvorak letters remain logical; Option keeps its code fallback.
        if chord.shift
            && let Some(key) = super::keys::logical_key(&chord.key)
        {
            let press = KeyPress::default();
            chord.key = key.chord(&press, Platform::Other).key;
        }
        Some(chord)
    }

    pub(super) fn index(&self) -> Option<ShortcutIndex> {
        self.physical
            .as_ref()
            .and_then(|chord| ShortcutIndex::from_key(&chord.key))
            .or_else(|| {
                self.logical
                    .as_ref()
                    .and_then(|chord| ShortcutIndex::from_key(&chord.key))
            })
    }

    pub(super) fn matches(&self, wanted: &Chord, platform: Platform) -> bool {
        if wanted.key == "Digit" {
            return self.index().is_some()
                && self
                    .logical
                    .as_ref()
                    .or(self.physical.as_ref())
                    .is_some_and(|pressed| modifiers_match(wanted, pressed, platform));
        }
        if self
            .logical
            .as_ref()
            .is_some_and(|pressed| same_chord(wanted, pressed, platform))
        {
            return true;
        }
        if wanted.shift
            && self.logical.as_ref().is_some_and(|pressed| {
                shifted_key_matches(&wanted.key, &pressed.key)
                    && modifiers_match(wanted, pressed, platform)
            })
        {
            return true;
        }
        let named = wanted.key.chars().count() > 1;
        let letter = matches!(wanted.key.as_bytes(), [letter] if letter.is_ascii_uppercase());
        let fallback = match self.fallback {
            Fallback::None => named,
            Fallback::Letter => named || letter,
            Fallback::Alt => true,
        };
        fallback
            && self
                .physical
                .as_ref()
                .is_some_and(|pressed| same_chord(wanted, pressed, platform))
    }
}

fn modifiers_match(wanted: &Chord, pressed: &Chord, platform: Platform) -> bool {
    let mut wanted = wanted.clone();
    wanted.key.clone_from(&pressed.key);
    same_chord(&wanted, pressed, platform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::sequence::KeySequence;

    fn chord(text: &str) -> Chord {
        KeySequence::parse(text)
            .unwrap()
            .chords()
            .next()
            .unwrap()
            .clone()
    }
    fn input(key: &str, code: &str, alt: bool) -> ShortcutInput {
        let InputConversion::Input(input) = ShortcutInput::from_press(
            &KeyPress {
                key: key.to_owned(),
                code: code.to_owned(),
                ctrl: true,
                alt,
                ..KeyPress::default()
            },
            Platform::Other,
        ) else {
            panic!("expected input")
        };
        input
    }

    #[test]
    fn dvorak_logical_copy_never_becomes_the_physical_period_shortcut() {
        let input = input("v", "Period", false);
        assert!(input.matches(&chord("Mod+V"), Platform::Other));
        assert!(!input.matches(&chord("Mod+."), Platform::Other));
    }

    #[test]
    fn non_latin_letters_and_option_have_narrow_physical_fallbacks() {
        assert!(input("ㅜ", "KeyN", false).matches(&chord("Mod+N"), Platform::Other));
        assert!(!input("ж", "BracketLeft", false).matches(&chord("Mod+["), Platform::Other));
        assert!(input("“", "BracketLeft", true).matches(&chord("Mod+Alt+["), Platform::Other));
    }

    #[test]
    fn numpad_indices_and_named_insert_are_supported() {
        let digits = input("3", "Numpad3", false);
        assert!(digits.matches(&chord("Mod+Digit"), Platform::Other));
        assert_eq!(digits.index().unwrap().get(), 3);
        assert!(input("Insert", "Insert", false).matches(&chord("Mod+Insert"), Platform::Other));
    }
}
