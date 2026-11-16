//! A shortcut's keys (the page's `data/shortcuts/keys.ts` and
//! `platform.ts`): how a binding is written (the text an override keeps),
//! how a key press becomes a chord, and how a binding is shown on each
//! platform.
//!
//! [Decision Log] Keys by position, `Mod` for the primary modifier (the
//! page's, milestone 11)
//! - Purpose: one grammar for the registry's defaults, the person's
//!   overrides, the listener's matching and Settings' key caps, so the
//!   four cannot drift.
//! - Choice: a key is its position (the DOM's `code`; the app maps its
//!   own key names onto it), written as a US keyboard shows it without
//!   Shift, so `Alt+[`, `Alt+1` and `N` work with Option held, with an
//!   input method on and on any QWERTY-shaped layout; each press has one
//!   spelling (`Shift+/`, never `?`). `Mod` names Command on a Mac and
//!   Control elsewhere; `Ctrl` and `Meta` name the keys themselves, and
//!   chords are compared after `Mod` is resolved. A binding is one chord
//!   or two in sequence (`G S`).
//! - Display: a Mac's glyphs in Apple's order (⌃ ⌥ ⇧ ⌘) with no
//!   separator; elsewhere "Ctrl+Shift+K".

use serde::{Deserialize, Serialize};

mod toolkit;
pub use toolkit::code_of;

/// The two conventions shortcuts are shown and matched by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    /// Command is the primary modifier, keys shown as glyphs.
    Mac,
    /// Control is.
    Other,
}

impl Platform {
    /// The convention of the system this build runs on.
    #[must_use]
    pub const fn native() -> Self {
        if cfg!(any(target_os = "macos", target_os = "ios")) {
            Self::Mac
        } else {
            Self::Other
        }
    }

    /// A browser's `platform` and `userAgent` as a convention: Apple's
    /// desktops and tablets with a keyboard are a Mac (the browser build).
    #[must_use]
    pub fn of(platform: &str, user_agent: &str) -> Self {
        let platform = platform.to_lowercase();
        let user_agent = user_agent.to_lowercase();
        let apple = ["mac", "iphone", "ipad", "ipod"]
            .iter()
            .any(|name| platform.contains(name))
            || ["macintosh", "mac os x"]
                .iter()
                .any(|name| user_agent.contains(name));
        if apple { Self::Mac } else { Self::Other }
    }
}

/// One step of a binding: its modifiers and its key.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "one flag per modifier key")]
pub struct Chord {
    /// `Mod`: Command on a Mac, Control elsewhere.
    pub primary: bool,
    /// Control itself.
    pub ctrl: bool,
    /// Alt (Option).
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Meta: the Windows key, or Command named as itself.
    pub meta: bool,
    /// A letter in upper case, a digit, another printable character, or
    /// one of the named keys.
    pub key: String,
}

/// A binding: one chord, or two pressed one after the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The chord, or a sequence's first.
    pub first: Chord,
    /// A sequence's second.
    pub then: Option<Chord>,
}

/// The keys written by name.
const NAMED: [&str; 28] = [
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
    "Insert",
    "Digit",
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

/// The modifiers, in the order a binding writes them.
const MODIFIERS: [&str; 5] = ["Mod", "Ctrl", "Alt", "Shift", "Meta"];

/// The longest binding an override keeps.
pub const MAX_BINDING: usize = super::sequence::MAX_SEQUENCE_BYTES;

/// The printable keys, as a US keyboard shows them without Shift.
const SIGNS: &str = "`-=[]\\;',./?";

fn single(key: &str) -> Option<char> {
    let mut chars = key.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

fn is_key(key: &str) -> bool {
    NAMED.contains(&key)
        || single(key).is_some_and(|one| {
            one.is_ascii_uppercase() || one.is_ascii_digit() || SIGNS.contains(one)
        })
}

fn parse_chord(text: &str) -> Option<Chord> {
    let mut parts: Vec<&str> = text.split('+').collect();
    let key = parts.pop().filter(|key| is_key(key))?;
    let mut chord = Chord {
        primary: false,
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        key: key.to_owned(),
    };
    let mut order = 0;
    for part in parts {
        let at = MODIFIERS.iter().position(|name| *name == part)?;
        if at < order {
            return None;
        }
        order = at.saturating_add(1);
        match part {
            "Mod" => chord.primary = true,
            "Ctrl" => chord.ctrl = true,
            "Alt" => chord.alt = true,
            "Shift" => chord.shift = true,
            _ => chord.meta = true,
        }
    }
    Some(chord)
}

impl Binding {
    /// `text` as a binding, if it is one: the grammar an override keeps.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.is_empty() || text.chars().count() > MAX_BINDING {
            return None;
        }
        let steps: Vec<&str> = text.split(' ').collect();
        match steps.as_slice() {
            [one] => Some(Self {
                first: parse_chord(one)?,
                then: None,
            }),
            [one, two] => Some(Self {
                first: parse_chord(one)?,
                then: Some(parse_chord(two)?),
            }),
            _ => None,
        }
    }

    /// The binding as an override keeps it.
    #[must_use]
    pub fn text(&self) -> String {
        self.then.as_ref().map_or_else(
            || chord_text(&self.first),
            |then| format!("{} {}", chord_text(&self.first), chord_text(then)),
        )
    }

    /// Its chords, in the order they are pressed.
    pub fn chords(&self) -> impl Iterator<Item = &Chord> {
        std::iter::once(&self.first).chain(self.then.as_ref())
    }
}

fn chord_text(chord: &Chord) -> String {
    let held = [
        (chord.primary, "Mod"),
        (chord.ctrl, "Ctrl"),
        (chord.alt, "Alt"),
        (chord.shift, "Shift"),
        (chord.meta, "Meta"),
    ];
    let mut names: Vec<&str> = held
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| *name)
        .collect();
    names.push(&chord.key);
    names.join("+")
}

/// The modifiers a chord holds on `platform`, `Mod` resolved: Command and
/// Control on a Mac, Control and the Windows key elsewhere.
pub(super) const fn held(chord: &Chord, platform: Platform) -> [bool; 4] {
    let (command, control) = match platform {
        Platform::Mac => (chord.primary || chord.meta, chord.ctrl),
        Platform::Other => (chord.meta, chord.primary || chord.ctrl),
    };
    [command, control, chord.alt, chord.shift]
}

/// Primary-command identity after Mod/Ctrl/Meta resolution; Shift is separate
/// because host-reserved chords and editing chords have different shift rules.
pub(super) fn primary_command(chord: &Chord, platform: Platform) -> bool {
    let [command, control, alt, _shift] = held(chord, platform);
    match platform {
        Platform::Mac => command && !control && !alt,
        Platform::Other => control && !command && !alt,
    }
}

pub(super) fn native_editing(chord: &Chord, platform: Platform) -> bool {
    primary_command(chord, platform)
        && !chord.shift
        && ["A", "C", "V", "X", "Z"].contains(&chord.key.as_str())
}

/// Whether two chords are the same keys on `platform`.
#[must_use]
pub fn same_chord(one: &Chord, two: &Chord, platform: Platform) -> bool {
    one.key == two.key && held(one, platform) == held(two, platform)
}

/// Whether pressing one of the two would also start or be the other on
/// `platform`: the same keys, or one a sequence that begins with the
/// other.
#[must_use]
pub fn overlaps(one: &Binding, two: &Binding, platform: Platform) -> bool {
    super::sequence_capture::sequences_overlap(
        &super::sequence::KeySequence::from_binding(one),
        &super::sequence::KeySequence::from_binding(two),
        platform,
    )
}

/// Whether a chord holds a modifier that makes it a command rather than
/// text (so it may fire while a field has focus).
#[must_use]
pub const fn commanding(chord: &Chord) -> bool {
    chord.primary || chord.ctrl || chord.alt || chord.meta
}

/// What the listener reads of a key press, in the DOM's terms (the app
/// writes its own key names as their positions).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "one flag per modifier key")]
pub struct KeyPress {
    /// What it typed (`"k"`, `"“"`, `"Process"`).
    pub key: String,
    /// Its position (`"KeyK"`, `"BracketLeft"`); empty when unknown.
    pub code: String,
    /// Control held.
    pub ctrl: bool,
    /// Alt (Option) held.
    pub alt: bool,
    /// Shift held.
    pub shift: bool,
    /// Command or the Windows key held.
    pub meta: bool,
}

/// Positions whose key is not their name's last letter.
const CODES: [(&str, &str); 16] = [
    ("Backquote", "`"),
    ("Minus", "-"),
    ("Equal", "="),
    ("BracketLeft", "["),
    ("BracketRight", "]"),
    ("Backslash", "\\"),
    ("Semicolon", ";"),
    ("Quote", "'"),
    ("Comma", ","),
    ("Period", "."),
    ("Slash", "/"),
    ("ArrowUp", "Up"),
    ("ArrowDown", "Down"),
    ("ArrowLeft", "Left"),
    ("ArrowRight", "Right"),
    ("NumpadEnter", "Enter"),
];

/// The key at a press's position; `None` for a modifier alone or a key no
/// binding names. A press without a position falls back to its character
/// when that is a letter or a digit.
#[must_use]
pub fn key_of(press: &KeyPress) -> Option<String> {
    logical_key(&press.key)
        .or_else(|| physical_key(&press.code))
        .map(|key| key.0)
}

/// A validated key name stays typed between input-boundary helpers.
#[derive(Debug, Clone)]
pub(super) struct KeyName(String);

impl KeyName {
    pub(super) fn chord(self, press: &KeyPress, platform: Platform) -> Chord {
        let mac = platform == Platform::Mac;
        Chord {
            primary: if mac { press.meta } else { press.ctrl },
            ctrl: mac && press.ctrl,
            alt: press.alt,
            shift: press.shift,
            meta: !mac && press.meta,
            key: self.0,
        }
    }
}

pub(super) fn logical_key(text: &str) -> Option<KeyName> {
    if text == " " {
        return Some(KeyName("Space".to_owned()));
    }
    if let Some((_, name)) = CODES.iter().find(|(code, _)| *code == text) {
        return Some(KeyName((*name).to_owned()));
    }
    if NAMED.contains(&text) {
        return Some(KeyName(text.to_owned()));
    }
    if let Some(value) = single(text) {
        if value.is_ascii_alphabetic() {
            return Some(KeyName(value.to_ascii_uppercase().to_string()));
        }
        if let Some((_, code)) = toolkit::SHIFTED.iter().find(|(sign, _)| *sign == value) {
            return physical_key(code);
        }
        if value.is_ascii_digit() || SIGNS.contains(value) {
            return Some(KeyName(value.to_string()));
        }
    }
    None
}

pub(super) fn logical_event_key(text: &str) -> Option<KeyName> {
    if single(text).is_some_and(|value| toolkit::SHIFTED.iter().any(|(sign, _)| *sign == value)) {
        return Some(KeyName(text.to_owned()));
    }
    logical_key(text)
}

pub(super) fn shifted_key_matches(wanted: &str, actual: &str) -> bool {
    single(actual)
        .and_then(|value| toolkit::SHIFTED.iter().find(|(sign, _)| *sign == value))
        .and_then(|(_, code)| physical_key(code))
        .is_some_and(|key| key.0 == wanted)
}

pub(super) fn physical_key(code: &str) -> Option<KeyName> {
    let letter = code
        .strip_prefix("Key")
        .and_then(single)
        .filter(char::is_ascii_uppercase);
    let digit = code
        .strip_prefix("Digit")
        .or_else(|| code.strip_prefix("Numpad"))
        .and_then(single)
        .filter(char::is_ascii_digit);
    if let Some(found) = letter.or(digit) {
        return Some(KeyName(found.to_string()));
    }
    if let Some((_, key)) = CODES.iter().find(|(name, _)| *name == code) {
        return Some(KeyName((*key).to_owned()));
    }
    NAMED.contains(&code).then(|| KeyName(code.to_owned()))
}

/// A key press as a chord on `platform`; `None` for a modifier alone or a
/// key no binding can name.
#[must_use]
pub fn chord_of(press: &KeyPress, platform: Platform) -> Option<Chord> {
    logical_key(&press.key)
        .or_else(|| physical_key(&press.code))
        .map(|key| key.chord(press, platform))
}

/// One chord's caps on `platform`: a Mac's glyphs (⌃⌥⇧⌘ then the key),
/// else the names, as separate caps.
#[must_use]
pub fn chord_caps(chord: &Chord, platform: Platform) -> Vec<String> {
    let key = match chord.key.as_str() {
        "Up" => "↑",
        "Down" => "↓",
        "Left" => "←",
        "Right" => "→",
        "Escape" => "Esc",
        "Digit" => "1–9",
        other => other,
    };
    let held = match platform {
        Platform::Mac => [
            (chord.ctrl, "⌃"),
            (chord.alt, "⌥"),
            (chord.shift, "⇧"),
            (chord.primary || chord.meta, "⌘"),
        ],
        Platform::Other => [
            (chord.primary || chord.ctrl, "Ctrl"),
            (chord.alt, "Alt"),
            (chord.shift, "Shift"),
            (chord.meta, "Win"),
        ],
    };
    held.iter()
        .filter(|(on, _)| *on)
        .map(|(_, cap)| *cap)
        .chain(std::iter::once(key))
        .map(str::to_owned)
        .collect()
}

/// A binding in words for a sentence ("⌘K", "Ctrl+Shift+B", "G then S"),
/// `then` being the language's word between a sequence's two chords.
#[must_use]
pub fn binding_words(binding: &Binding, platform: Platform, then: &str) -> String {
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    binding
        .chords()
        .map(|chord| chord_caps(chord, platform).join(joiner))
        .collect::<Vec<_>>()
        .join(then)
}
