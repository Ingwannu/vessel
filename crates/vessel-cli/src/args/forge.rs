//! `vessel pr` (protocol 11, ADR 0029): a session's pull request, opening
//! and merging one, its feedback switch, and the node's sign-in.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use crate::selector::SessionSelector;

/// `vessel pr …`. A pull request is reached through its session (the
/// branch its folder has checked out).
#[derive(Debug, Subcommand)]
pub(crate) enum PrCommand {
    /// Every watched session's pull request.
    List,
    /// A session's pull request, whole.
    Show {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// Look at the forge now instead of showing the last look.
        #[arg(long)]
        refresh: bool,
    },
    /// Push the session's branch and open a pull request of it.
    Open {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// Its title.
        #[arg(long)]
        title: String,
        /// Its description, read from this file (`-` for standard input).
        #[arg(long, value_name = "FILE")]
        body_file: Option<PathBuf>,
        /// The branch to merge into (default: the project's default
        /// branch).
        #[arg(long)]
        base: Option<String>,
        /// Open it as a draft.
        #[arg(long)]
        draft: bool,
    },
    /// Merge the session's pull request: exactly the one named, at exactly
    /// the head given, with the method given. Never deletes the branch.
    Merge {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// The pull request's number, as `vessel pr show` printed it.
        #[arg(long)]
        number: u64,
        /// Its head commit, in full, as `vessel pr show` printed it.
        #[arg(long)]
        head: String,
        /// How to merge it.
        #[arg(long, value_enum)]
        method: Method,
    },
    /// Turn sending the pull request's failing checks, requested changes
    /// and comments to the session on or off (off by default).
    Feedback {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// `on` or `off`.
        #[arg(value_enum)]
        state: Switch,
    },
    /// The forges the node reaches, and who it is signed in as.
    Status,
}

/// A merge method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Method {
    /// A merge commit.
    Merge,
    /// One commit of all its changes.
    Squash,
    /// Its commits replayed on the base.
    Rebase,
}

/// On or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Switch {
    On,
    Off,
}
