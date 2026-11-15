//! A scope's changed files, by path, with their line counts: what a page
//! lists before it opens one. No file's patch is read here (`file`).
//!
//! [Decision Log]
//! - Choice: the summary is one `git diff --raw --numstat -z` run.
//!   Untracked files, which `git diff` does not see, are listed from
//!   `git ls-files --others --exclude-standard` and counted here from the
//!   file, the first 500 of them.

use std::path::Path;

use vessel_proto::workspace::{ChangeStatus, DiffSummary, FileChange, RelPath};

use super::{GITLINK, MAX_FILES, Resolved, line_count, read_small};
use crate::git::{Git, GitError, patch};

/// Untracked files counted line by line in a summary; past that their
/// counts are left out (Paseo counts 500).
const COUNTED_UNTRACKED: usize = 500;

impl Git {
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
            let untracked = self.untracked(dir, resolved.index.as_deref()).await?;
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

    /// Untracked files that are not ignored.
    pub(super) async fn untracked(
        &self,
        dir: &Path,
        index: Option<&Path>,
    ) -> Result<Vec<RelPath>, GitError> {
        let mut command =
            self.command(dir)
                .args(["ls-files", "--others", "--exclude-standard", "-z"]);
        if let Some(index) = index {
            command = command.env("GIT_INDEX_FILE", index);
        }
        let output = command.output().await?.checked()?;
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .filter_map(|path| patch::rel(&String::from_utf8_lossy(path)))
            .take(MAX_FILES.saturating_add(1))
            .collect())
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
