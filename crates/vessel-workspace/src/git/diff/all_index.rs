//! All comparison restoration after checkout-git.ts:3533–3577.
//! A path removed from HEAD/index but recreated on disk must be compared to
//! its base blob once, rather than reported as deleted plus separately added.
//!
//! [Decision Log]
//! - Existing asset: protected runner and snapshot's private-index strategy.
//! - Choice: retain a copy of the real index, restoring only untracked paths
//!   present in the comparison base; every mutation targets the private copy.
//! - Alternative: stage the person's index, or interpret patches in the app.
//!   Both change authority and lose rename/mode behaviour.
//! - Repeated cacheinfo arguments avoid extending the runner with stdin.
//! - The node runs comparisons on its task tracker: losing the requesting
//!   connection never abandons the private index before explicit cleanup.
//!   Both success and read failure close the lease on a blocking worker.

use std::io::{Read as _, Write as _};
use std::path::Path;

use vessel_proto::workspace::{
    CommitId, DiffSummary, FileDiff, GitComparison, GitHunkOffset, GitWhitespace, InvalidRelPath,
    InvalidText, RelPath, WorktreeBase,
};

use super::{MAX_FILES, Resolved};
use crate::git::{Git, GitError};

/// Bound both argv size (including worst-case quote expansion on Windows)
/// and path count; ordinary short paths still share one subprocess.
const MAX_BATCH_ARG_BYTES: usize = 16 * 1024;
const MAX_BATCH_PATHS: usize = 128;
/// Git rewrites the index through a lock file, independently of the copied
/// file's mode. A per-command override keeps that private replacement0600,
/// even in repositories configured for group sharing; nothing is persisted.
const PRIVATE_INDEX_SHARING: &str = "core.sharedRepository=0600";
/// Leave space for executable, flags, mode and object ids in each batch.
const ARG_FIXED_BYTES: usize = 1024;
/// --cacheinfo, mode, object id and separators, independent of path length.
const CACHEINFO_ENTRY_BYTES: usize = 128;

/// Keep disk and git failures typed without erasing their source.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// A protected git command failed.
    #[error("git could not complete the comparison")]
    Git(#[from] GitError),
    /// Creating, copying or removing the private index failed.
    #[error("the comparison index could not be prepared")]
    Io(#[from] std::io::Error),
    /// A blocking copy/cleanup task failed to join.
    #[error("the comparison index worker stopped")]
    Task(#[from] tokio::task::JoinError),
    /// A base record contains invalid UTF-8.
    #[error("git printed a base index entry that is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    /// A base record contains an invalid object id.
    #[error("git printed an invalid base object id")]
    Object(#[from] InvalidText),
    /// A base record contains an escaping or invalid path.
    #[error("git printed an invalid base index path")]
    Path(#[from] InvalidRelPath),
    /// Both the read and removing its private index failed.
    #[error("the All comparison failed and its private index could not be removed: {cleanup}")]
    Cleanup {
        /// The original git failure.
        #[source]
        read: GitError,
        /// The removal failure, retained alongside the original cause.
        cleanup: std::io::Error,
    },
    /// Both the read and joining its cleanup task failed.
    #[error("the All comparison failed and its cleanup worker stopped: {cleanup}")]
    CleanupTask {
        /// The original git failure.
        #[source]
        read: GitError,
        /// The cleanup task failure, retained alongside the original cause.
        cleanup: tokio::task::JoinError,
    },
}

/// Private lease: neither its path nor a cloneable scope escapes the read.
/// The tracked node operation must await completion, including cleanup.
#[must_use]
#[derive(Debug)]
pub(crate) struct Prepared {
    scope: Resolved,
    directory: Option<tempfile::TempDir>,
}

impl Prepared {
    /// Finish every read through one boundary; a failed read still waits for
    /// cleanup, retaining both errors if removing its private index fails.
    pub(super) async fn finish<T>(mut self, read: Result<T, GitError>) -> Result<T, IndexError> {
        match read {
            Ok(value) => {
                self.close().await?;
                Ok(value)
            }
            Err(read) => match self.directory.take() {
                Some(directory) => Err(close_failed_read(directory, read).await),
                None => Err(read.into()),
            },
        }
    }

    pub(super) async fn changes(
        &self,
        git: &Git,
        dir: &Path,
        whitespace: GitWhitespace,
    ) -> Result<DiffSummary, GitError> {
        git.changes(dir, &self.scope, whitespace.ignored()).await
    }

    pub(super) async fn file(
        &self,
        git: &Git,
        dir: &Path,
        path: &RelPath,
        whitespace: GitWhitespace,
        offset: GitHunkOffset,
    ) -> Result<FileDiff, GitError> {
        git.file_diff(dir, &self.scope, path, whitespace.ignored(), offset.get())
            .await
    }

    pub(super) async fn close(self) -> Result<(), IndexError> {
        if let Some(directory) = self.directory {
            tokio::task::spawn_blocking(move || directory.close()).await??;
        }
        Ok(())
    }
}

impl Git {
    /// Only All may restore entries; other comparisons stay on the original
    /// read-only index and all reads borrow the lease until they finish.
    pub(crate) async fn prepare_comparison(
        &self,
        dir: &Path,
        comparison: &GitComparison,
        base: Option<&WorktreeBase>,
        scratch: &Path,
    ) -> Result<Option<Prepared>, IndexError> {
        if !scratch.is_absolute() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "comparison scratch directory must be absolute",
            )
            .into());
        }
        let Some(scope) = self.resolve_comparison(dir, comparison, base).await? else {
            return Ok(None);
        };
        match comparison {
            GitComparison::All { .. } => {
                self.prepare_all_index(dir, scope, scratch).await.map(Some)
            }
            GitComparison::Uncommitted
            | GitComparison::Committed { .. }
            | GitComparison::Commit { .. } => Ok(Some(Prepared {
                scope,
                directory: None,
            })),
        }
    }

    /// Restore base entries only when a current untracked path needs one.
    async fn prepare_all_index(
        &self,
        dir: &Path,
        mut scope: Resolved,
        scratch: &Path,
    ) -> Result<Prepared, IndexError> {
        let Some(base) = &scope.from else {
            return Ok(Prepared {
                scope,
                directory: None,
            });
        };
        let paths = self.untracked(dir, None).await?;
        if paths.len() > MAX_FILES {
            return Err(GitError::OutputLimit.into());
        }
        let entries = self.base_entries(dir, base, &paths).await?;
        if entries.is_empty() {
            return Ok(Prepared {
                scope,
                directory: None,
            });
        }
        let index = self
            .command(dir)
            .args(["rev-parse", "--git-path", "index"])
            .output()
            .await?
            .checked()?;
        let text = String::from_utf8_lossy(&index.stdout);
        let source = dir.join(text.trim_end_matches(['\r', '\n']));
        let scratch = scratch.to_owned();
        let directory =
            tokio::task::spawn_blocking(move || copy_index(&source, IndexCopyLimit::MAX, &scratch))
                .await??;
        let destination = directory.path().join("index");
        let installed = self.restore_entries(dir, &destination, &entries).await;
        if let Err(error) = installed {
            // Close even when git refuses restoration. A cleanup failure must
            // retain the original git error as well; no success is reported.
            return Err(close_failed_read(directory, error).await);
        }
        scope.index = Some(destination);
        Ok(Prepared {
            scope,
            directory: Some(directory),
        })
    }

    async fn base_entries(
        &self,
        dir: &Path,
        base: &CommitId,
        paths: &[RelPath],
    ) -> Result<Vec<Entry>, IndexError> {
        let mut entries = Vec::new();
        for batch in batches(paths) {
            let output = self.command(dir).args(["ls-tree", "-r", "-z"])
                .arg(base.as_str()).arg("--")
                // The shared runner already sets GIT_LITERAL_PATHSPECS=1.
                .args(batch.iter().map(RelPath::as_str))
                .output().await?.checked()?;
            entries.extend(parse_entries(&output.stdout)?);
        }
        Ok(entries)
    }

    async fn restore_entries(
        &self,
        dir: &Path,
        index: &Path,
        entries: &[Entry],
    ) -> Result<(), GitError> {
        // Reuse path batching for the larger cacheinfo argv with an allowance
        // for mode/object fields accounted separately in each path's cost.
        let paths: Vec<_> = entries.iter().map(|entry| entry.path.clone()).collect();
        let mut offset = 0;
        for batch in batches(&paths) {
            let mut command = self.command(dir).env("GIT_INDEX_FILE", index).args([
                "-c",
                PRIVATE_INDEX_SHARING,
                "update-index",
                "--add",
            ]);
            for entry in entries.iter().skip(offset).take(batch.len()) {
                command = command.arg("--cacheinfo").arg(format!(
                    "{},{},{}",
                    entry.mode.text(),
                    entry.object,
                    entry.path.as_str()
                ));
            }
            command.run().await?;
            offset = offset.saturating_add(batch.len());
        }
        Ok(())
    }
}

async fn close_failed_read(directory: tempfile::TempDir, read: GitError) -> IndexError {
    match tokio::task::spawn_blocking(move || directory.close()).await {
        Ok(Ok(())) => IndexError::Git(read),
        Ok(Err(cleanup)) => IndexError::Cleanup { read, cleanup },
        Err(cleanup) => IndexError::CleanupTask { read, cleanup },
    }
}

/// A separate budget from a file preview: a repository index can describe
/// many unchanged files. Bound copied bytes without trusting a pre-open stat.
#[derive(Debug, Clone, Copy)]
struct IndexCopyLimit(u64);

impl IndexCopyLimit {
    const MAX: Self = Self(64 * 1024 * 1024);
}

fn copy_index(
    source: &Path,
    limit: IndexCopyLimit,
    scratch: &Path,
) -> Result<tempfile::TempDir, std::io::Error> {
    let source = match crate::regular::open(source) {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "git index is not a regular file",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let mut builder = tempfile::Builder::new();
    let _prefix = builder.prefix("vessel-git-comparison-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        // Temp directories otherwise start at 0777 masked by the process
        // umask. Restrict at creation, before any repository index is copied.
        let _mode = builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let directory = builder.tempdir_in(scratch)?;
    if let Some(source) = source {
        let mut options = std::fs::OpenOptions::new();
        let _write = options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            let _mode = options.mode(0o600);
        }
        let mut destination = options.open(directory.path().join("index"))?;
        let copied = std::io::copy(
            &mut source.take(limit.0.saturating_add(1)),
            &mut destination,
        )?;
        if copied > limit.0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "git index exceeds the comparison budget",
            ));
        }
        destination.flush()?;
    }
    Ok(directory)
}

#[derive(Debug)]
struct Entry {
    mode: Mode,
    object: CommitId,
    path: RelPath,
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Regular,
    Executable,
    Link,
    Gitlink,
}

impl Mode {
    const fn text(self) -> &'static str {
        match self {
            Self::Regular => "100644",
            Self::Executable => "100755",
            Self::Link => "120000",
            Self::Gitlink => "160000",
        }
    }
}

fn parse_entries(bytes: &[u8]) -> Result<Vec<Entry>, IndexError> {
    let unexpected = || GitError::Unexpected {
        what: "base index entry",
    };
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let text = std::str::from_utf8(record)?;
            let (metadata, path) = text.split_once('\t').ok_or_else(unexpected)?;
            let mut fields = metadata.split(' ');
            let mode = fields.next().ok_or_else(unexpected)?;
            let kind = fields.next().ok_or_else(unexpected)?;
            let object = fields.next().ok_or_else(unexpected)?.parse()?;
            if fields.next().is_some() {
                return Err(unexpected().into());
            }
            let mode = match (mode, kind) {
                ("100644", "blob") => Mode::Regular,
                ("100755", "blob") => Mode::Executable,
                ("120000", "blob") => Mode::Link,
                ("160000", "commit") => Mode::Gitlink,
                _ => return Err(unexpected().into()),
            };
            let path: RelPath = path.parse()?;
            if path.is_root() {
                return Err(unexpected().into());
            }
            Ok::<_, IndexError>(Entry { mode, object, path })
        })
        .collect()
}

fn batches(paths: &[RelPath]) -> Vec<&[RelPath]> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut bytes = ARG_FIXED_BYTES;
    for (index, path) in paths.iter().enumerate() {
        // Double quotes can double during Windows escaping. An additional
        // fixed allowance per entry covers --cacheinfo, mode and SHA-256 ids.
        let cost = path
            .as_str()
            .len()
            .saturating_mul(2)
            .saturating_add(CACHEINFO_ENTRY_BYTES);
        if index > start
            && (index.saturating_sub(start) >= MAX_BATCH_PATHS
                || bytes.saturating_add(cost) > MAX_BATCH_ARG_BYTES)
        {
            if let Some(batch) = paths.get(start..index) {
                result.push(batch);
            }
            start = index;
            bytes = ARG_FIXED_BYTES;
        }
        bytes = bytes.saturating_add(cost);
    }
    if let Some(batch) = paths.get(start..).filter(|batch| !batch.is_empty()) {
        result.push(batch);
    }
    result
}

#[cfg(test)]
mod tests;
