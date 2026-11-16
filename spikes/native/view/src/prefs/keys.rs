//! Keyboard shortcuts as they are kept (the bridge's `wire/prefs/keys.rs`,
//! ADR 0028): [`ActionId`] names an action, [`KeyBinding`] the keys that
//! run it, in one spelling per press. Only the form is checked here; which
//! actions exist and which keys a surface keeps for itself are the
//! shortcut registry's rules (track S).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::MAX_BINDINGS;

/// One action: 1 to 96 characters of `a-z`, `0-9` and `-`, starting with
/// a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionId(String);

/// The longest [`ActionId`], in characters (all ASCII).
const MAX_ACTION: usize = 96;

/// Text that is not an [`ActionId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("an action id is 1 to {MAX_ACTION} characters of a-z, 0-9 and -, starting with a letter")]
pub struct InvalidActionId;

impl ActionId {
    /// The id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ActionId {
    type Err = InvalidActionId;

    fn from_str(text: &str) -> Result<Self, InvalidActionId> {
        let starts = text
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase());
        let rest = text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if starts && rest && text.len() <= MAX_ACTION {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidActionId)
        }
    }
}

impl TryFrom<String> for ActionId {
    type Error = InvalidActionId;

    fn try_from(text: String) -> Result<Self, InvalidActionId> {
        text.parse()
    }
}

impl From<ActionId> for String {
    fn from(id: ActionId) -> Self {
        id.0
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The keys that run a binding: up to 16 chords (`G S`) separated by one
/// space, at most 256 bytes. Each chord joins modifiers and one key with `+`.
/// The shared grammar canonicalizes Cmd/Meta, arrow names and Digit templates.
/// Matching prefers logical characters (including `?`); permitted physical
/// fallbacks belong to `shortcuts::input`, never to persisted preference data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyBinding(String);

/// The parser and persisted bound are the same as the active shortcut engine.
const MAX_BINDING: usize = crate::shortcuts::sequence::MAX_SEQUENCE_BYTES;

/// Why text is not a [`KeyBinding`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidKeyBinding {
    /// The shared engine grammar refused this persisted shortcut.
    #[error(transparent)]
    Sequence(#[from] crate::shortcuts::sequence::SequenceError),
    /// Legacy decoding reason retained for callers of older settings formats.
    #[error("a key binding is at most {MAX_BINDING} bytes")]
    TooLong,
    /// Not one chord or two separated by one space.
    #[error("a key binding is one chord, or two separated by one space")]
    Chords,
    /// A chord without a key.
    #[error("a chord ends with its key")]
    NoKey,
    /// A key the grammar does not name.
    #[error("`{0}` is not a key")]
    Key(String),
    /// A modifier the grammar does not name.
    #[error("`{0}` is not a modifier: Mod, Ctrl, Alt, Shift or Meta")]
    Modifier(String),
    /// A modifier repeated or out of order.
    #[error("modifiers come once each, in the order Mod, Ctrl, Alt, Shift, Meta")]
    Order,
}

impl KeyBinding {
    /// The binding as kept.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for KeyBinding {
    type Err = InvalidKeyBinding;

    fn from_str(text: &str) -> Result<Self, InvalidKeyBinding> {
        Ok(Self(
            crate::shortcuts::sequence::KeySequence::parse(text)?.text(),
        ))
    }
}

impl TryFrom<String> for KeyBinding {
    type Error = InvalidKeyBinding;

    fn try_from(text: String) -> Result<Self, InvalidKeyBinding> {
        text.parse()
    }
}

impl From<KeyBinding> for String {
    fn from(binding: KeyBinding) -> Self {
        binding.0
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An action's keys: at most [`MAX_BINDINGS`]; none is "no keys".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<KeyBinding>", into = "Vec<KeyBinding>")]
pub struct Bindings(Vec<KeyBinding>);

/// More items than a list takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("at most {max}")]
pub struct TooMany {
    /// The most it takes.
    pub max: usize,
}

impl Bindings {
    /// The keys.
    #[must_use]
    pub fn keys(&self) -> &[KeyBinding] {
        &self.0
    }
}

impl TryFrom<Vec<KeyBinding>> for Bindings {
    type Error = TooMany;

    fn try_from(keys: Vec<KeyBinding>) -> Result<Self, TooMany> {
        if keys.len() <= MAX_BINDINGS {
            Ok(Self(keys))
        } else {
            Err(TooMany { max: MAX_BINDINGS })
        }
    }
}

impl From<Bindings> for Vec<KeyBinding> {
    fn from(bindings: Bindings) -> Self {
        bindings.0
    }
}
