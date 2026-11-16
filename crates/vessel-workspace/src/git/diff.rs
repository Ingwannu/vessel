//! Diffs by scope (protocol 10): what a scope compares (`scope`), the
//! changed files (`summary`), then one file's hunks (`file`). This module
//! keeps what they share: the resolved scope, the one way `git diff` is
//! run, and how an untracked file is read and counted.
//!
//! [Decision Log]
//! - Purpose: a page lists what changed and opens one file at a time, as
//!   all three references do; a whole-tree patch in one frame is what
//!   their caps (2 MiB, 1 MiB per file) exist to prevent.
//! - Every diff turns external diff programs and text conversion off
//!   (`--no-ext-diff --no-textconv`): a repository's configuration names
//!   them, and a read must not run them (ADR 0023).

mod base;
mod file;
mod scope;
mod summary;

mod all_index;
mod comparison;

pub use all_index::IndexError as ComparisonError;
pub use comparison::Comparison;

use std::path::{Path, PathBuf};

use vessel_proto::workspace::CommitId;

use super::Git;

/// The most files a summary lists.
pub(crate) const MAX_FILES: usize = 5000;
/// The most lines one `git.diff` call returns.
pub(crate) const MAX_LINES: usize = 5000;
/// The most of git's output read for one file's patch.
pub(crate) const MAX_PATCH_BYTES: usize = 4 * 1024 * 1024;
/// The largest untracked file made into a patch or counted.
const MAX_UNTRACKED_BYTES: u64 = 4 * 1024 * 1024;
/// The mode git gives a submodule.
const GITLINK: u32 = 0o160_000;

/// What a scope compares, resolved to git's arguments
/// ([`Git::resolve_scope`]).
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The arguments after `diff` that pick the two sides.
    sides: Vec<String>,
    /// The commit compared against.
    from: Option<CommitId>,
    /// Whether untracked files count as added.
    untracked: bool,
    /// Only a node-created private index; never a client path. Legacy scopes
    /// leave it absent, preserving their existing admitted behaviour.
    index: Option<PathBuf>,
}

impl Git {
    /// `git diff` with a scope's sides and the read-only flags.
    fn diff(&self, dir: &Path, resolved: &Resolved, ignore_whitespace: bool) -> super::GitCommand {
        let mut command = self.command(dir).args([
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--find-renames",
            "--ignore-submodules=dirty",
            "--submodule=short",
        ]);
        if ignore_whitespace {
            command = command.arg("--ignore-all-space");
        }
        if let Some(index) = &resolved.index {
            command = command.env("GIT_INDEX_FILE", index);
        }
        command.args(&resolved.sides)
    }
}

/// A file of at most [`MAX_UNTRACKED_BYTES`]; `None` when larger or
/// unreadable.
fn read_small(path: &Path) -> Option<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_UNTRACKED_BYTES {
        return None;
    }
    std::fs::read(path).ok()
}

fn line_count(bytes: &[u8]) -> u32 {
    let newlines = bytes.split(|byte| *byte == b'\n').count().saturating_sub(1);
    let last = usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"));
    u32::try_from(newlines.saturating_add(last)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod comparison_tests;
