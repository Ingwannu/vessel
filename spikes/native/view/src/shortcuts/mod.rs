//! Shortcuts (the page's `data/shortcuts/`): the keys' grammar, the
//! registry of actions and their defaults, matching a press, changing a
//! key, and Settings' rows. GPUI-free, so the page's cases run as they
//! are; the app's one listener maps a keystroke to a [`KeyPress`] and
//! asks [`matched`].

mod capture;
mod edit;
mod held;
mod keys;
mod listing;
mod matching;
mod registry;

pub use capture::{Capture, Done, Verdict, clear, reset};
pub use edit::{Clash, Refusal, assign, clash_of, refusal, remove_key, restore};
pub use held::Held;
pub use keys::{
    Binding, Chord, KeyPress, MAX_BINDING, Platform, binding_words, chord_caps, chord_of, code_of,
    commanding, key_of, overlaps, same_chord,
};
pub use listing::{ShortcutFilter, ShortcutRow, modified_count, shortcut_row, shortcut_rows};
pub use matching::{Matched, Pending, SEQUENCE_MS, matched};
pub use registry::{
    Action, ShortcutGroup, ShortcutScope, bindings_of, jump_of, jump_shown, modified,
};
