//! Keyboard shortcuts as the page stores them: [`ActionId`] names one of
//! the page's actions, [`KeyBinding`] the keys that run it. Both are
//! parsed when a frame or the preferences file is decoded, so a handler
//! never holds a binding the page could not match.
//!
//! [Decision Log]
//! - Purpose: the page matches a key press against what is kept. Two
//!   spellings of one press (`Mod+?` and `Mod+Shift+/`, `k` and `K`) would
//!   be two bindings that look different and clash silently.
//! - Choice: one grammar with one spelling per press (ADR 0028): a key is
//!   a physical key (`KeyboardEvent.code`) written as it is on a US
//!   keyboard without Shift, the modifiers come in a fixed order, each at
//!   most once, and a binding is one chord or two. The parser accepts only
//!   that spelling, so equal presses are equal strings.
//! - Alternatives: a structured binding (`{ mods: [..], key: ".." }`),
//!   which is longer on the wire and in the file and still needs the same
//!   checks; accepting several spellings and normalising them (the page
//!   would still have to agree on the normal form, so it might as well
//!   send it).
//! - The bridge does not know the page's actions or which keys a browser
//!   keeps for itself: those rules are the page's (`web/src/data/
//!   shortcuts/`). It checks only the form.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// One of the page's actions: 1 to 48 characters of `a-z`, `0-9` and `-`,
/// starting with a letter. The page's own ids; the bridge does not know
/// the set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
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

/// The keys that run an action: one chord, or two pressed in sequence
/// (`G S`), separated by one space, at most 32 bytes.
///
/// A chord is its modifiers then its key, joined by `+`. The modifiers are
/// `Mod` (Command on a Mac, Control elsewhere), `Ctrl`, `Alt`, `Shift` and
/// `Meta`, each at most once and in that order. A key names a physical key
/// (`KeyboardEvent.code`) as it is written on a US keyboard without Shift:
/// `A` to `Z`, `0` to `9`, `` ` - = [ ] \ ; ' , . / ``, or one of `Enter`,
/// `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`,
/// `Right`, `Home`, `End`, `PageUp`, `PageDown`, `F1` to `F12`. So `?` is
/// `Shift+/`, and each key press has exactly one spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
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
    /// Longer than [`MAX_BINDING`] bytes.
    #[error("a key binding is at most {MAX_BINDING} bytes")]
    TooLong,
    /// Not one chord or two separated by one space.
    #[error("a key binding is one chord, or two separated by one space")]
    Chords,
    /// A chord without a key (`Mod+`, an empty chord).
    #[error("a chord ends with its key")]
    NoKey,
    /// A key this grammar does not name (`k`, `?`, `Plus`).
    #[error(
        "`{0}` is not a key: keys are A-Z, 0-9, the unshifted punctuation of a US keyboard, or a named key"
    )]
    Key(String),
    /// A modifier this grammar does not name.
    #[error("`{0}` is not a modifier: Mod, Ctrl, Alt, Shift or Meta")]
    Modifier(String),
    /// A modifier repeated or out of order (`Shift+Mod+K`, `Mod+Mod+K`).
    #[error("modifiers come once each, in the order Mod, Ctrl, Alt, Shift, Meta")]
    Order,
}

impl KeyBinding {
    /// The binding as the page writes it.
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
    // Read right to left, each modifier must rank below the one after it.
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

/// Whether `key` names a key of the grammar.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_have_one_spelling() {
        for good in [
            "Mod+K",
            "Alt+[",
            "G S",
            "/",
            "Shift+/",
            "Mod+Shift+B",
            "Ctrl+Alt+Shift+Meta+F12",
            "Mod+,",
            "Alt+1",
            "N",
            "Mod+-",
            "Mod+=",
            "Mod+\\",
            "Mod+'",
            "`",
            "Mod+Enter",
            "Shift+PageDown",
            "G M",
            "Mod+K Mod+S",
        ] {
            let binding: KeyBinding = good.parse().unwrap();
            assert_eq!(binding.as_str(), good);
        }
        for (bad, why) in [
            ("k", InvalidKeyBinding::Key("k".to_owned())),
            ("?", InvalidKeyBinding::Key("?".to_owned())),
            ("Shift+Mod+K", InvalidKeyBinding::Order),
            ("Mod+", InvalidKeyBinding::NoKey),
            ("Mod+Mod+K", InvalidKeyBinding::Order),
            ("A B C", InvalidKeyBinding::Chords),
            ("Plus", InvalidKeyBinding::Key("Plus".to_owned())),
            ("Mod+K ", InvalidKeyBinding::NoKey),
            ("", InvalidKeyBinding::NoKey),
            (" ", InvalidKeyBinding::NoKey),
            ("G  S", InvalidKeyBinding::NoKey),
            ("Cmd+K", InvalidKeyBinding::Modifier("Cmd".to_owned())),
            ("mod+K", InvalidKeyBinding::Modifier("mod".to_owned())),
            ("Mod+F13", InvalidKeyBinding::Key("F13".to_owned())),
            ("Mod+Shift+?", InvalidKeyBinding::Key("?".to_owned())),
            ("Mod++", InvalidKeyBinding::NoKey),
            (
                "Ctrl+Alt+Shift+Meta+F12 Mod+Shift+B",
                InvalidKeyBinding::TooLong,
            ),
        ] {
            assert_eq!(bad.parse::<KeyBinding>(), Err(why), "{bad:?}");
        }
        // Decoded from a frame the same way.
        let binding: KeyBinding = serde_json::from_str("\"Alt+]\"").unwrap();
        assert_eq!(serde_json::to_string(&binding).unwrap(), "\"Alt+]\"");
        assert!(serde_json::from_str::<KeyBinding>("\"Alt+}\"").is_err());
    }

    #[test]
    fn action_ids_are_the_pages_own_shape() {
        for good in ["palette", "go-sessions", "session-9", &"a".repeat(48)] {
            assert_eq!(good.parse::<ActionId>().unwrap().as_str(), good);
        }
        for bad in [
            "",
            "9-lives",
            "-x",
            "Palette",
            "go_sessions",
            "a b",
            &"a".repeat(49),
        ] {
            assert_eq!(bad.parse::<ActionId>(), Err(InvalidActionId), "{bad:?}");
        }
    }
}
