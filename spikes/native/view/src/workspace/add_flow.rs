//! Add project as Paseo's sheet (TRACKER 234): the page stack, what each
//! page lists, and the paths and remotes behind its rows. Paseo's
//! `add-project-flow/model.ts`, `options.ts`, `project-picker-options.ts`,
//! `working-directory-suggestions.ts`, `shorten-path.ts`,
//! `search/text-match.ts` and `protocol/src/git-remote.ts` (pinned 0.15.4),
//! host as machine; the page's port is `web/src/data/workspace/add-project/`
//! and both follow docs/plans/wp-add-project-flow.md.
//!
//! [Decision Log]
//! - Purpose: one Add project for the sidebar's "+", the composer's "Add
//!   project" row and Settings > Projects, replacing the add menu and its
//!   three inline forms, with Paseo's four methods in Paseo's order.
//! - Constraint: the app draws and calls; the behaviour (the back stack,
//!   the drafts kept across Back, the wrap-and-skip keys, the rows) must be
//!   testable without a window, and the same as the page's.
//! - Alternatives: keeping the inline forms and adding search under them
//!   (what was here; not Paseo's flow); the page's model shared through
//!   wasm (the app's views are Rust; one port per side is the convention).
//! - Choice: plain state (`model`), rows as typed values with their words
//!   chosen by the language (`rows`, `text::workspace`), and Paseo's helpers
//!   one for one (`paths`, `options`, `git_remote`).
//! - Cost: two ports of one reference; their tests are the same cases.

pub mod git_remote;
pub mod model;
pub mod options;
pub mod paths;
pub mod repositories;
pub mod rows;

#[cfg(test)]
mod tests;
