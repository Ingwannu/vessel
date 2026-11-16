//! Projects, git worktrees, git and files for a Vessel node (milestone 11,
//! track B; ADRs 0023 and 0027).
//!
//! Owns: running the `git` program safely ([`git`]), keeping every path a
//! client names inside its folder ([`paths`]), and reading a project's own
//! file. Must not: know about sessions, principals, the store or the RPC
//! (the node exposes this crate as calls and decides who may make them,
//! ADR 0001), read the process's environment or home (the node passes the
//! `git` it found), or run anything through a shell except a project's
//! approved commands.
//!
//! Everything here is tested against repositories the tests create in
//! temporary folders, with the real `git`; nothing reaches the network.

pub mod commands;
pub mod files;
pub mod find;
pub mod git;
pub mod paths;
#[cfg(target_os = "linux")]
pub mod physical;
pub mod project_file;
mod regular;
#[cfg(test)]
mod testing;
mod text;

pub use git::{Checkout, Found, Git, GitError, GitSetup, RemoteAddress};
