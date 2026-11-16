//! What the app's screens mean, with no UI and no I/O in it: the machines
//! and the board read from nodes, and a session's conversation folded
//! from its records.
//!
//! The view-model crate the plan names (`vessel-view`), begun with the
//! slice the first build draws. A lane, a card's phrase and the fold of a
//! conversation are decided here, once, as `web/src/data` decides them for
//! the page, so every surface (desktop, phone, browser) draws the same
//! thing. Nothing of a UI toolkit's may be used here (ADR 0035): the
//! screens draw these values through Vessel's kit, whatever is under it.

pub mod about;
pub mod agent_mark;
pub mod agent_status;
pub mod ago;
pub mod alert;
pub mod attention;
pub mod badge;
pub mod board;
pub mod brand;
pub mod composer;
pub mod connection;
pub mod connections;
mod conversation;
pub mod devices;
pub mod diagnostics;
pub mod effort;
pub mod extras;
pub mod files;
pub mod fork;
pub mod history;
pub mod idle_release;
mod links;
pub mod machine_book;
pub mod machines;
pub mod mark_attributions;
pub mod matcher;
pub mod models;
pub mod notices;
pub mod offers;
mod options;
pub mod overview;
pub mod pairing;
pub mod palette;
pub mod panels;
pub mod prefs;
pub mod requests;
pub mod rewind;
pub mod search;
pub mod session;
pub mod session_lifecycle;
pub mod sessions;
pub mod settings;
pub mod shortcuts;
pub mod sidebar;
pub mod slider;
pub mod sync;
pub mod terminal;
#[cfg(test)]
mod tests;
mod text;
pub mod timeline;
pub mod toasts;
pub mod updates;
pub mod usage;
pub mod workspace;

pub use agent_mark::AgentMark;
pub use board::{
    Agent, Attention, Card, Fleet, Lane, Machine, NodeVersion, Phrase, Reach, Via, agent_name,
    rows_for_scroll,
};
pub use conversation::{Asked, Conversation, Entry, Notice, Speaker, ToolState};
pub use links::is_web_url;
pub use options::{
    Controls, Picks, SelectControl, ToggleControl, choices_for, controls_of, controls_with_effort,
    own_name, with_picks,
};
pub use text::localization;
pub use text::{
    BoardWords, ConversationWords, GitFailureText, GitHashText, GitHeaderText, GitLabels,
    GitStashText, GitSuccessCommits, GitSuccessText, Lang, MachinesText, MarkText, NoticeWords,
    OptionsText, OverviewText, PaletteWords, SettingsText, ShortcutWords, SidebarWords, Text,
    grouped, meter_tokens, short_tokens,
};

pub mod layout;
pub mod residents;
