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

mod board;
mod conversation;
pub mod session;
pub mod timeline;

pub use board::{Card, Fleet, Lane, Machine, Reach, rows_for_scroll};
pub use conversation::{Asked, Conversation, Entry, Speaker, ToolState};
