//! Keyboard shortcuts as they are kept (the bridge's `wire/prefs/keys.rs`,
//! ADR 0028): [`ActionId`] names an action, [`KeyBinding`] the keys that
//! run it, in one spelling per press. Only the form is checked here; which
//! actions exist and which keys a surface keeps for itself are the
//! shortcut registry's rules (track S).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::MAX_BINDINGS;

/// One action: 1 to 48 characters of `a-z`, `0-9` and `-`, starting with
/// a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionId(String);

/// The longest [`ActionId`], in characters (all ASCII).
const MAX_ACTION: usize = 48;

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

/// The keys that run an action: one chord, or two in sequence (`G S`),
/// separated by one space, at most 32 bytes. A chord is its modifiers
/// (`Mod`, `Ctrl`, `Alt`, `Shift`, `Meta`, each at most once, in that
/// order) then its key, joined by `+`; a key is a physical key as written
/// on a US keyboard without Shift (`A`–`Z`, `0`–`9`, `` ` - = [ ] \ ; ' ,
/// . / ``, or a named key), so `?` is `Shift+/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyBinding(String);

/// The longest [`KeyBinding`], in bytes.
const MAX_BINDING: usize = 32;

/// The modifiers, in the only order a chord may name them.
const MODIFIERS: [&str; 5] = ["Mod", "Ctrl", "Alt", "Shift", "Meta"];

/// The keys with a name rather than a character.
const NAMED_KEYS: [&str; 26] = [
    "Enter",
    "Escape",
    "Tab",
    "Space",
    "Backspace",
    "Delete",
    "Up",
    "Down",
    "Left",
    "Right",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
];

/// The punctuation keys of a US keyboard, unshifted.
const PUNCTUATION: &[u8] = b"`-=[]\\;',./";

/// Why text is not a [`KeyBinding`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidKeyBinding {
    /// Longer than 32 bytes.
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
        if text.len() > MAX_BINDING {
            return Err(InvalidKeyBinding::TooLong);
        }
        let mut chords = 0_usize;
        for chord in text.split(' ') {
            chords = chords.saturating_add(1);
            if chords > 2 {
                return Err(InvalidKeyBinding::Chords);
            }
            check_chord(chord)?;
        }
        Ok(Self(text.to_owned()))
    }
}

/// One chord: modifiers in order, then a key.
fn check_chord(chord: &str) -> Result<(), InvalidKeyBinding> {
    // `+` is not a key (it is `Shift+=`), so splitting on it is exact.
    let mut parts = chord.split('+').rev();
    let key = parts.next().unwrap_or_default();
    if key.is_empty() {
        return Err(InvalidKeyBinding::NoKey);
    }
    if !is_key(key) {
        return Err(InvalidKeyBinding::Key(key.to_owned()));
    }
    let mut after = MODIFIERS.len();
    for modifier in parts {
        let rank = MODIFIERS
            .iter()
            .position(|known| *known == modifier)
            .ok_or_else(|| InvalidKeyBinding::Modifier(modifier.to_owned()))?;
        if rank >= after {
            return Err(InvalidKeyBinding::Order);
        }
        after = rank;
    }
    Ok(())
}

fn is_key(key: &str) -> bool {
    match key.as_bytes() {
        [byte] => byte.is_ascii_uppercase() || byte.is_ascii_digit() || PUNCTUATION.contains(byte),
        _ => NAMED_KEYS.contains(&key),
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
