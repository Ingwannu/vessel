//! One file's diff in a scope, from a hunk on: its typed hunks, or what
//! stands in for them (a submodule's two commits, a binary file, a patch
//! too large). Listing the changed files is `summary`'s.
//!
//! [Decision Log]
//! - Choice: a file's patch is one `git diff` of that path (with its old
//!   path, for a rename), parsed into typed hunks here (Paseo's split: the
//!   page never parses a patch). An untracked file's "patch" is made here
//!   from the file: every line added.
//! - Cost: a rename is found by a whole-scope `--raw` run before the file's
//!   patch, a second git run per file opened.

use std::path::Path;

use vessel_proto::workspace::{ChangeStatus, DiffContent, FileDiff, LineKind, ModeChange, RelPath};

use super::{
    GITLINK, MAX_LINES, MAX_PATCH_BYTES, MAX_UNTRACKED_BYTES, Resolved, line_count, read_small,
};
use crate::git::patch::{self, Raw};
use crate::git::{Git, GitError, Limits};

impl Git {
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
            if resolved.untracked
                && self
                    .is_untracked(dir, path, resolved.index.as_deref())
                    .await?
            {
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

    async fn is_untracked(
        &self,
        dir: &Path,
        path: &RelPath,
        index: Option<&Path>,
    ) -> Result<bool, GitError> {
        let mut command = self
            .command(dir)
            .args(["ls-files", "--others", "--exclude-standard", "-z", "--"])
            .arg(path.as_str());
        if let Some(index) = index {
            command = command.env("GIT_INDEX_FILE", index);
        }
        let output = command.output().await?;
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
