//! Whether a worktree's `.git` file still names the repository's own
//! record of it.
//!
//! [Decision Log] (the merge of milestone 11's tracks, its review's first
//! finding)
//! - Purpose: git finds a linked worktree's repository through the
//!   worktree's `.git` file (`gitdir: <path>`), and reads that
//!   repository's configuration on every run, filter drivers included,
//!   which even a read (`git.changes`, `git.status`, tidying's inspection)
//!   runs. An agent working in the worktree can rewrite that file to name
//!   a folder of its own whose configuration names a program; the node,
//!   asked by a person to show the diff, would run it as itself, outside
//!   whatever sandbox the agent's harness keeps it in.
//! - Choice: before git runs in a worktree, its `.git` must be a regular
//!   file (not a link) of one `gitdir:` line naming, once resolved, a
//!   folder of the project's own `.git/worktrees/`, whose `gitdir` file
//!   names this worktree back: what `git worktree add` wrote. Anything
//!   else and git is not run there.
//! - Not covered: a project's own `.git/config`, which the user writes,
//!   and git reads as it does for the user's own git (ADR 0023); and a
//!   project whose `.git` is itself a file (a project that is a worktree
//!   or a submodule), whose worktrees are not checked this way.
//! - Cost: two small reads and two path resolutions per call.

use std::io::Read as _;
use std::path::{Path, PathBuf};

/// The longest `.git` file read; git's own are one line.
const MAX_GITFILE: u64 = 4096;

/// Whether `worktree`'s `.git` names the record `git worktree add` made
/// for it in the repository at `project` (blocking). A project whose own
/// `.git` is not a folder is not checked (`true`).
#[must_use]
pub fn registered(worktree: &Path, project: &Path) -> bool {
    let Ok(common) = std::fs::canonicalize(project.join(".git")) else {
        return false;
    };
    if !common.is_dir() {
        return true;
    }
    let Some(named) = gitdir_line(&worktree.join(".git")) else {
        return false;
    };
    let named = if named.is_absolute() {
        named
    } else {
        worktree.join(named)
    };
    let Ok(record) = std::fs::canonicalize(&named) else {
        return false;
    };
    let in_worktrees = record
        .parent()
        .is_some_and(|parent| parent == common.join("worktrees"));
    if !in_worktrees {
        return false;
    }
    // The record names this worktree back (`<record>/gitdir`).
    let Some(back) = first_line(&record.join("gitdir")) else {
        return false;
    };
    let back = PathBuf::from(back);
    let (Ok(back), Ok(own)) = (
        std::fs::canonicalize(&back),
        std::fs::canonicalize(worktree.join(".git")),
    ) else {
        return false;
    };
    back == own
}

/// The path a `.git` file's `gitdir:` line names; `None` for anything but
/// a small regular file of that form.
fn gitdir_line(file: &Path) -> Option<PathBuf> {
    let line = first_line(file)?;
    line.strip_prefix("gitdir: ").map(PathBuf::from)
}

/// The first line of `file`, which must be a regular file (a link is not
/// followed) of at most [`MAX_GITFILE`] bytes.
fn first_line(file: &Path) -> Option<String> {
    let opened = crate::regular::open(file).ok()??;
    if opened.metadata().ok()?.len() > MAX_GITFILE {
        return None;
    }
    let mut text = String::new();
    let _read = opened.take(MAX_GITFILE).read_to_string(&mut text).ok()?;
    text.lines().next().map(str::to_owned)
}
