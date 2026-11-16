//! Bounded multi-step shortcut grammar for per-binding overrides. Parsing reuses
//! the existing chord validator and canonical spelling; this does not introduce
//! another key vocabulary or a second matcher.

use super::{Binding, Chord};

/// A UI-generated sequence cannot retain unbounded pasted preference data.
pub const MAX_SEQUENCE_BYTES: usize = 256;
/// A finite chord sequence keeps capture and matching work bounded.
pub const MAX_SEQUENCE_STEPS: usize = 16;

/// A nonempty sequence of validated chords.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySequence(Vec<Chord>);

/// Invalid persisted or captured shortcut syntax, rejected at its boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SequenceError {
    /// Unassigned bindings use an explicit empty override, not an empty sequence.
    #[error("shortcut sequence is empty")]
    Empty,
    /// Bound preference-file and capture data before parsing.
    #[error("shortcut sequence is too long")]
    TooLong,
    /// Prevent an indefinitely growing capture.
    #[error("shortcut sequence has too many steps")]
    TooManySteps,
    /// A step uses a key or modifier outside the shared grammar.
    #[error("invalid shortcut chord")]
    InvalidChord,
}

impl KeySequence {
    /// Reuse a validated legacy binding during the registry migration without
    /// reparsing text or inventing another displayed shortcut spelling.
    #[must_use]
    pub fn from_binding(binding: &Binding) -> Self {
        Self(binding.chords().cloned().collect())
    }

    /// Parse reference spellings (Cmd/Meta, ArrowLeft/Left, Digit) into the
    /// existing canonical grammar. A digit wildcard is retained as a template.
    ///
    /// # Errors
    /// Empty, oversized and malformed sequences are refused before matching.
    pub fn parse(text: &str) -> Result<Self, SequenceError> {
        if text.is_empty() {
            return Err(SequenceError::Empty);
        }
        if text.len() > MAX_SEQUENCE_BYTES {
            return Err(SequenceError::TooLong);
        }
        let mut chords = Vec::new();
        for text in text.split(' ') {
            if chords.len() >= MAX_SEQUENCE_STEPS {
                return Err(SequenceError::TooManySteps);
            }
            chords.push(parse_step(text)?);
        }
        Ok(Self(chords))
    }

    /// Each step, in order; also the one source of shortcut hint caps.
    pub fn chords(&self) -> impl Iterator<Item = &Chord> {
        self.0.iter()
    }

    pub(super) fn step(&self, at: usize) -> Option<&Chord> {
        self.0.get(at)
    }

    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    /// Canonical persisted spelling, stable across key-order aliases.
    #[must_use]
    pub fn text(&self) -> String {
        self.0
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
}

fn parse_step(text: &str) -> Result<Chord, SequenceError> {
    let mut parts: Vec<_> = text.split('+').collect();
    let key = parts.pop().ok_or(SequenceError::InvalidChord)?;
    let key = match key {
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "Digit" => "1",
        other => other,
    };
    let mut modifiers = [false; 5];
    for part in parts {
        let at = match part {
            "Mod" => 0,
            "Ctrl" => 1,
            "Alt" => 2,
            "Shift" => 3,
            "Meta" | "Cmd" => 4,
            _ => return Err(SequenceError::InvalidChord),
        };
        let Some(slot) = modifiers.get_mut(at) else {
            return Err(SequenceError::InvalidChord);
        };
        if *slot {
            return Err(SequenceError::InvalidChord);
        }
        *slot = true;
    }
    let names = ["Mod", "Ctrl", "Alt", "Shift", "Meta"];
    let normalized = modifiers
        .iter()
        .zip(names)
        .filter_map(|(held, name)| held.then_some(name))
        .chain(std::iter::once(key))
        .collect::<Vec<_>>()
        .join("+");
    let mut chord = Binding::parse(&normalized)
        .ok_or(SequenceError::InvalidChord)?
        .first;
    if text.rsplit('+').next() == Some("Digit") {
        "Digit".clone_into(&mut chord.key);
    }
    Ok(chord)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_chords_share_the_existing_grammar_and_normalize_platform_aliases() {
        let sequence = KeySequence::parse("Cmd+K Ctrl+J Alt+Shift+ArrowRight").unwrap();
        assert_eq!(sequence.text(), "Meta+K Ctrl+J Alt+Shift+Right");
        assert_eq!(sequence.chords().count(), 3);
        assert_eq!(KeySequence::parse("Alt+Digit").unwrap().text(), "Alt+Digit");
        assert_eq!(
            KeySequence::parse("Ctrl+Ctrl+K"),
            Err(SequenceError::InvalidChord)
        );
        assert_eq!(
            KeySequence::parse("Ctrl+K  Ctrl+S"),
            Err(SequenceError::InvalidChord)
        );
        assert_eq!(KeySequence::parse(""), Err(SequenceError::Empty));
    }
}
