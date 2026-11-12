//! Diffs by scope: the changed files, then one file's hunks (protocol 10).
//!
//! [Decision Log]
//! - Purpose: a page lists what changed and opens one file at a time, as
//!   all three references do; a whole-tree patch in one frame is what
//!   their caps (2 MiB, 1 MiB per file) exist to prevent.
//! - Choice: the summary is one `git diff --raw --numstat -z` run; a file's
//!   patch is one `git diff` of that path (with its old path, for a
//!   rename), parsed into typed hunks here (Paseo's split: the page never
//!   parses a patch). Untracked files, which `git diff` does not see, are
//!   listed from `git ls-files --others --exclude-standard` and their
//!   "patch" made here from the file: every line added.
//! - Every diff turns external diff programs and text conversion off
//!   (`--no-ext-diff --no-textconv`): a repository's configuration names
//!   them, and a read must not run them (ADR 0023).
//! - Cost: a rename is found by a whole-scope `--raw` run before the file's
//!   patch, a second git run per file opened.

use std::path::Path;

use vessel_proto::workspace::{
    ChangeStatus, CommitId, DiffContent, DiffScope, DiffSummary, FileChange, FileDiff, LineKind,
    ModeChange, RelPath, WorktreeBase,
};

use super::patch::{self, Raw};
use super::{Git, GitError, Limits};

/// The most files a summary lists.
pub(crate) const MAX_FILES: usize = 5000;
/// The most lines one `git.diff` call returns.
pub(crate) const MAX_LINES: usize = 5000;
/// The most of git's output read for one file's patch.
pub(crate) const MAX_PATCH_BYTES: usize = 4 * 1024 * 1024;
/// Untracked files counted line by line in a summary; past that their
/// counts are left out (Paseo counts 500).
const COUNTED_UNTRACKED: usize = 500;
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
}

impl Git {
    /// What `scope` compares in the repository at `dir`. `base` is the
    /// base to use when the scope names none (a worktree's own, or the
    /// project's default branch).
    ///
    /// # Errors
    /// [`GitError::Unexpected`] (`"base"`) when a base or a commit does not
    /// resolve, and as the runs'.
    pub async fn resolve_scope(
        &self,
        dir: &Path,
        scope: &DiffScope,
        base: Option<&WorktreeBase>,
    ) -> Result<Resolved, GitError> {
        let head = self.head(dir).await?;
        let empty = || async { self.empty_tree(dir).await };
        Ok(match scope {
            DiffScope::Unstaged => Resolved {
                sides: Vec::new(),
                from: head,
                untracked: true,
            },
            DiffScope::Staged => Resolved {
                sides: vec!["--cached".to_owned()],
                from: head,
                untracked: false,
            },
            DiffScope::Uncommitted => {
                let side = match &head {
                    Some(head) => head.to_string(),
                    None => empty().await?,
                };
                Resolved {
                    sides: vec![side],
                    from: head,
                    untracked: true,
                }
            }
            DiffScope::Base { base: asked } => {
                let base = asked
                    .as_ref()
                    .or(base)
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let commit = self
                    .resolve(dir, base)
                    .await?
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let fork = self.merge_base(dir, &commit).await?.unwrap_or(commit);
                Resolved {
                    sides: vec![fork.to_string()],
                    from: Some(fork),
                    untracked: true,
                }
            }
            DiffScope::Commit { id } => {
                let commit = self
                    .resolve(dir, &WorktreeBase::Commit { id: id.clone() })
                    .await?
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let parent = self.parent(dir, &commit).await?;
                let from = match &parent {
                    Some(parent) => parent.to_string(),
                    None => empty().await?,
                };
                Resolved {
                    sides: vec![from, commit.to_string()],
                    from: parent,
                    untracked: false,
                }
            }
        })
    }

    /// The first parent of `commit`; `None` for a root commit.
    async fn parent(&self, dir: &Path, commit: &CommitId) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{commit}^1^{{commit}}"))
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }

    /// Where `HEAD` and `other` forked; `None` when they share no history.
    async fn merge_base(&self, dir: &Path, other: &CommitId) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["merge-base", "HEAD"])
            .arg(other.as_str())
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }

    /// The empty tree's id in this repository's hash (SHA-1 or SHA-256):
    /// the side to compare against before the first commit.
    async fn empty_tree(&self, dir: &Path) -> Result<String, GitError> {
        let output = self
            .command(dir)
            .args(["hash-object", "-t", "tree", "--stdin"])
            .output()
            .await?
            .checked()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    /// The changed files of `scope`, by path.
    ///
    /// # Errors
    /// As [`resolve_scope`](Self::resolve_scope) and the runs'.
    pub async fn changes(
        &self,
        dir: &Path,
        resolved: &Resolved,
        ignore_whitespace: bool,
    ) -> Result<DiffSummary, GitError> {
        let output = self
            .diff(dir, resolved, ignore_whitespace)
            .args(["--raw", "--numstat", "-z"])
            .output()
            .await?;
        let cut = output.stdout_cut;
        let output = if cut { output } else { output.checked()? };
        let counts: std::collections::HashMap<String, Option<(u32, u32)>> =
            patch::numstat_records(&output.stdout).into_iter().collect();
        let mut files: Vec<FileChange> = patch::raw_records(&output.stdout)
            .into_iter()
            .filter_map(|raw| {
                let count = counts.get(&raw.path).copied().flatten();
                Some(FileChange {
                    path: patch::rel(&raw.path)?,
                    old_path: raw.old_path.as_deref().and_then(patch::rel),
                    status: raw.status,
                    additions: count.map(|(added, _)| added),
                    deletions: count.map(|(_, removed)| removed),
                    binary: count.is_none() && raw.new_mode != GITLINK && raw.old_mode != GITLINK,
                })
            })
            .collect();
        if resolved.untracked {
            let untracked = self.untracked(dir).await?;
            let counted = untracked_counts(dir, untracked.iter().take(COUNTED_UNTRACKED)).await;
            for (index, path) in untracked.into_iter().enumerate() {
                let count = counted.get(index).copied().flatten();
                files.push(FileChange {
                    path,
                    old_path: None,
                    status: ChangeStatus::Added,
                    additions: count.and_then(|count| count.lines),
                    deletions: count.and_then(|count| count.lines.map(|_| 0)),
                    binary: count.is_some_and(|count| count.lines.is_none()),
                });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let more = cut || files.len() > MAX_FILES;
        files.truncate(MAX_FILES);
        Ok(DiffSummary {
            from: resolved.from.clone(),
            files,
            more,
        })
    }

    /// One file's diff in `resolved`, from hunk `from` on.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn file_diff(
        &self,
        dir: &Path,
        resolved: &Resolved,
        path: &RelPath,
        ignore_whitespace: bool,
        from: u32,
    ) -> Result<FileDiff, GitError> {
        let records = self
            .diff(dir, resolved, ignore_whitespace)
            .args(["--raw", "-z"])
            .output()
            .await?
            .checked()?;
        let raw = patch::raw_records(&records.stdout)
            .into_iter()
            .find(|raw| raw.path == path.as_str());
        let Some(raw) = raw else {
            if resolved.untracked && self.is_untracked(dir, path).await? {
                return untracked_diff(dir, path, from).await;
            }
            return Ok(unchanged(path));
        };
        let mode = (raw.old_mode != 0 && raw.new_mode != 0 && raw.old_mode != raw.new_mode)
            .then_some(ModeChange {
                old: raw.old_mode,
                new: raw.new_mode,
            });
        let base = FileDiff {
            path: path.clone(),
            old_path: raw.old_path.as_deref().and_then(patch::rel),
            status: raw.status,
            mode,
            content: DiffContent::Unchanged,
            more_from: None,
        };
        if raw.old_mode == GITLINK || raw.new_mode == GITLINK {
            return Ok(FileDiff {
                content: DiffContent::Submodule {
                    from: raw.old_id,
                    to: raw.new_id,
                },
                ..base
            });
        }
        self.patch_of(dir, resolved, &raw, ignore_whitespace, from, base)
            .await
    }

    async fn patch_of(
        &self,
        dir: &Path,
        resolved: &Resolved,
        raw: &Raw,
        ignore_whitespace: bool,
        from: u32,
        base: FileDiff,
    ) -> Result<FileDiff, GitError> {
        let mut command = self
            .diff(dir, resolved, ignore_whitespace)
            .arg("--unified=3")
            .limits(Limits::LOCAL.with_stdout(MAX_PATCH_BYTES))
            .arg("--");
        if let Some(old) = &raw.old_path {
            command = command.arg(old);
        }
        let output = command.arg(&raw.path).output().await?;
        if output.stdout_cut {
            return Ok(FileDiff {
                content: DiffContent::TooLarge {
                    bytes: u64::try_from(MAX_PATCH_BYTES).unwrap_or(u64::MAX),
                },
                ..base
            });
        }
        let output = output.checked()?;
        let parsed = patch::hunks(&output.stdout, from, MAX_LINES);
        let content = if parsed.binary {
            DiffContent::Binary
        } else {
            DiffContent::Hunks {
                hunks: parsed.hunks,
            }
        };
        Ok(FileDiff {
            content,
            more_from: parsed.more_from,
            ..base
        })
    }

    /// `git diff` with a scope's sides and the read-only flags.
    fn diff(&self, dir: &Path, resolved: &Resolved, ignore_whitespace: bool) -> super::GitCommand {
        let mut command = self.command(dir).args([
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--find-renames",
            "--ignore-submodules=none",
            "--submodule=short",
        ]);
        if ignore_whitespace {
            command = command.arg("--ignore-all-space");
        }
        command.args(&resolved.sides)
    }

    /// Untracked files that are not ignored.
    async fn untracked(&self, dir: &Path) -> Result<Vec<RelPath>, GitError> {
        let output = self
            .command(dir)
            .args(["ls-files", "--others", "--exclude-standard", "-z"])
            .output()
            .await?;
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .filter_map(|path| patch::rel(&String::from_utf8_lossy(path)))
            .take(MAX_FILES.saturating_add(1))
            .collect())
    }

    async fn is_untracked(&self, dir: &Path, path: &RelPath) -> Result<bool, GitError> {
        let output = self
            .command(dir)
            .args(["ls-files", "--others", "--exclude-standard", "-z", "--"])
            .arg(path.as_str())
            .output()
            .await?;
        Ok(!output.stdout.is_empty())
    }
}

fn unchanged(path: &RelPath) -> FileDiff {
    FileDiff {
        path: path.clone(),
        old_path: None,
        status: ChangeStatus::Modified,
        mode: None,
        content: DiffContent::Unchanged,
        more_from: None,
    }
}

/// An untracked file's line count; `None` lines for a binary or too large
/// file.
#[derive(Debug, Clone, Copy)]
struct Counted {
    lines: Option<u32>,
}

async fn untracked_counts<'a>(
    dir: &Path,
    paths: impl Iterator<Item = &'a RelPath>,
) -> Vec<Option<Counted>> {
    let files: Vec<_> = paths.map(|path| path.under(dir)).collect();
    tokio::task::spawn_blocking(move || {
        files
            .iter()
            .map(|file| {
                let bytes = read_small(file)?;
                Some(Counted {
                    lines: crate::text::is_text(&bytes).then(|| line_count(&bytes)),
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default()
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

/// An untracked file as a patch: every line added.
async fn untracked_diff(dir: &Path, path: &RelPath, from: u32) -> Result<FileDiff, GitError> {
    let file = path.under(dir);
    let bytes = tokio::task::spawn_blocking(move || read_small(&file))
        .await
        .map_err(|_panicked| GitError::Unexpected {
            what: "an untracked file",
        })?;
    let base = FileDiff {
        path: path.clone(),
        old_path: None,
        status: ChangeStatus::Added,
        mode: None,
        content: DiffContent::Unchanged,
        more_from: None,
    };
    let Some(bytes) = bytes else {
        return Ok(FileDiff {
            content: DiffContent::TooLarge {
                bytes: MAX_UNTRACKED_BYTES,
            },
            ..base
        });
    };
    if !crate::text::is_text(&bytes) {
        return Ok(FileDiff {
            content: DiffContent::Binary,
            ..base
        });
    }
    let lines: Vec<_> = bytes
        .split(|byte| *byte == b'\n')
        .take(usize::try_from(line_count(&bytes)).unwrap_or(usize::MAX))
        .take(MAX_LINES)
        .map(|line| patch::diff_line(LineKind::Added, line))
        .collect();
    let count = line_count(&bytes);
    let hunks = if from > 0 || lines.is_empty() {
        Vec::new()
    } else {
        vec![vessel_proto::workspace::Hunk {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: count,
            section: None,
            lines,
        }]
    };
    Ok(FileDiff {
        content: DiffContent::Hunks { hunks },
        ..base
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[tokio::test]
    async fn every_scope_lists_what_it_compares() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
        Sandbox::write(&repo, "committed.txt", "c\n");
        sandbox.run(&repo, &["add", "."]);
        sandbox.run(&repo, &["commit", "-q", "-m", "c"]);
        Sandbox::write(&repo, "README.md", "hello\nworld\n");
        Sandbox::write(&repo, "staged.txt", "s\n");
        sandbox.run(&repo, &["add", "staged.txt"]);
        Sandbox::write(&repo, "untracked.txt", "u1\nu2\n");
        let main = WorktreeBase::Ref {
            name: "main".parse().unwrap(),
        };
        let paths = |summary: &DiffSummary| {
            summary
                .files
                .iter()
                .map(|file| (file.path.as_str().to_owned(), file.status, file.additions))
                .collect::<Vec<_>>()
        };
        let (git, repo, main) = (&git, &repo, &main);
        let scope = |scope| async move {
            let resolved = git.resolve_scope(repo, &scope, Some(main)).await.unwrap();
            git.changes(repo, &resolved, false).await.unwrap()
        };
        let unstaged = scope(DiffScope::Unstaged).await;
        assert_eq!(
            paths(&unstaged),
            [
                ("README.md".to_owned(), ChangeStatus::Modified, Some(1)),
                ("untracked.txt".to_owned(), ChangeStatus::Added, Some(2)),
            ]
        );
        let staged = scope(DiffScope::Staged).await;
        assert_eq!(
            paths(&staged),
            [("staged.txt".to_owned(), ChangeStatus::Added, Some(1))]
        );
        let uncommitted = scope(DiffScope::Uncommitted).await;
        assert_eq!(uncommitted.files.len(), 3);
        let since_base = scope(DiffScope::Base { base: None }).await;
        assert_eq!(
            since_base
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["README.md", "committed.txt", "staged.txt", "untracked.txt"],
            "committed and uncommitted together"
        );
        let head = git.head(repo).await.unwrap().unwrap();
        let one = scope(DiffScope::Commit { id: head }).await;
        assert_eq!(
            paths(&one),
            [("committed.txt".to_owned(), ChangeStatus::Added, Some(1))]
        );
    }

    #[tokio::test]
    async fn a_file_s_diff_is_typed_hunks_and_untracked_files_are_all_added() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        Sandbox::write(&repo, "README.md", "hello\nworld\n");
        Sandbox::write(&repo, "new.txt", "a\nb");
        let resolved = git
            .resolve_scope(&repo, &DiffScope::Uncommitted, None)
            .await
            .unwrap();
        let readme = git
            .file_diff(&repo, &resolved, &"README.md".parse().unwrap(), false, 0)
            .await
            .unwrap();
        let DiffContent::Hunks { hunks } = &readme.content else {
            panic!("{readme:?}");
        };
        let lines: Vec<_> = hunks
            .first()
            .unwrap()
            .lines
            .iter()
            .map(|line| (line.kind, line.text.as_str()))
            .collect();
        assert_eq!(
            lines,
            [(LineKind::Context, "hello"), (LineKind::Added, "world")]
        );
        let new = git
            .file_diff(&repo, &resolved, &"new.txt".parse().unwrap(), false, 0)
            .await
            .unwrap();
        assert_eq!(new.status, ChangeStatus::Added);
        let DiffContent::Hunks { hunks } = &new.content else {
            panic!("{new:?}");
        };
        assert_eq!(hunks.first().unwrap().lines.len(), 2);
        let same = git
            .file_diff(&repo, &resolved, &"nothing.txt".parse().unwrap(), false, 0)
            .await
            .unwrap();
        assert_eq!(same.content, DiffContent::Unchanged);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_mode_change_and_a_binary_file_say_so() {
        use std::os::unix::fs::PermissionsExt;
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        Sandbox::write(&repo, "bin.dat", "a\0b");
        sandbox.run(&repo, &["add", "bin.dat"]);
        sandbox.run(&repo, &["commit", "-q", "-m", "bin"]);
        std::fs::write(repo.join("bin.dat"), b"c\0d").unwrap();
        std::fs::set_permissions(
            repo.join("README.md"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        let resolved = git
            .resolve_scope(&repo, &DiffScope::Uncommitted, None)
            .await
            .unwrap();
        let summary = git.changes(&repo, &resolved, false).await.unwrap();
        let binary = summary
            .files
            .iter()
            .find(|file| file.path.as_str() == "bin.dat")
            .unwrap();
        assert!(binary.binary);
        assert_eq!(binary.additions, None);
        let bin = git
            .file_diff(&repo, &resolved, &"bin.dat".parse().unwrap(), false, 0)
            .await
            .unwrap();
        assert_eq!(bin.content, DiffContent::Binary);
        let mode = git
            .file_diff(&repo, &resolved, &"README.md".parse().unwrap(), false, 0)
            .await
            .unwrap();
        assert_eq!(
            mode.mode,
            Some(ModeChange {
                old: 0o100_644,
                new: 0o100_755
            })
        );
    }
}
