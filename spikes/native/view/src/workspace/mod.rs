//! Projects, worktrees, git and files: what the workspace's screens mean
//! (Settings > Projects, the approval, the code panel's Changes and Files,
//! where a new session runs), ported from the page's `data/workspace/`
//! with its test cases as the tests here (ADR 0036: one behaviour, checked
//! on both sides). No UI, no I/O: the calls are the link's, the drawing
//! the app's.
//!
//! One module per subject, as the page has them; the words of all of them
//! are [`words`]'s (`text/workspace/`), one table per language.

pub mod branch_name;
pub mod changes;
pub mod command_changes;
pub mod diff_rows;
pub mod editor;
pub mod file_panels;
pub mod file_scope;
pub mod file_ticket;
pub mod file_tree;
pub mod find;
pub mod git;
pub mod git_quick;
pub mod marks;
pub mod pace;
pub mod place;
pub mod problem;
pub mod projects;
pub mod pull;
pub mod reference;
pub mod root;
pub mod save;
pub mod setup_log;
pub mod size;
pub mod start;
pub mod syntax;
pub mod watch;
pub mod worktrees;

/// The area's words: `text/workspace/`, by the app's convention.
pub use crate::text::workspace as words;
