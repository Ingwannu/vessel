//! One shortcut registry, key grammar, scope policy and override model, shared
//! by the active engine, Settings, help and hint chips. Pure models keep GPUI
//! and owner I/O outside resolution. The native interceptor uses `Engine`;
//! `matched` remains a compatibility adapter for existing callers.

pub mod badges;
mod capture;
pub mod composer;
pub mod context;
pub mod definitions;
pub mod desktop;
pub mod dispatcher;
mod edit;
pub mod engine;
mod held;
pub mod help;
pub mod input;
mod keys;
mod labels;
mod listing;
mod matching;
pub mod rebinding;
mod registry;
pub mod sequence;
pub mod sequence_capture;

pub use capture::{Capture, Done, Verdict, clear, reset};
pub use edit::{Clash, Refusal, assign, clash_of, refusal, remove_key, restore};
pub use held::Held;
pub use keys::{
    Binding, Chord, KeyPress, MAX_BINDING, Platform, binding_words, chord_caps, chord_of, code_of,
    commanding, key_of, overlaps, same_chord,
};
pub use labels::{action_label, group_label};
pub use listing::{ShortcutFilter, ShortcutRow, modified_count, shortcut_row, shortcut_rows};
pub use matching::{Matched, Pending, SEQUENCE_MS, matched};
pub use registry::{
    Action, PASEO_BINDING_COUNT, RegistryError, ShortcutGroup, ShortcutScope, binding_id,
    bindings_of, default_context, definitions, effective_bindings, jump_of, jump_shown,
    migrated_overrides, modified, primary_keys, visible_definitions,
};
