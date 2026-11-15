//! Classified complete branch history after checkout-git.ts listCheckoutCommits.
//!
//! [Decision Log]
//! - Purpose: a 200-commit legacy page cannot prove complete branch history or
//!   publication against all remotes. Unknown facts must not become local/zero.
//! - Reuse: the protected git runner, existing commit values and immutable
//!   comparison readers. No app patch parsing or new dependency is introduced.
//! - Choice: strict NUL metadata records, all workspace commits plus ten actual
//!   fork ancestors, and rev-list membership against every observed remote ref.
//! - Difference: per-commit files stay lazy through the existing commit diff
//!   API instead of repeating them in history. Existing legacy log is unchanged.
mod base;
mod decode;

use crate::git::{Git, GitError};
use std::collections::HashSet;
use std::path::Path;
use vessel_proto::workspace::{
    CommitId, GitCommitLocation, GitCommitPublication, GitHistory, GitHistoryCommit, InvalidText,
    RefName,
};

const BASE_CONTEXT: u32 = 10;
const FORMAT: &str = "--format=%H%x00%P%x00%h%x00%an%x00%at%x00%s";

/// Fields used in a protected history read; no raw diagnostic chooses behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryField {
    /// Parent object IDs must all be valid.
    Parents,
    /// The observed abbreviation must prefix the full object ID.
    Abbreviation,
    /// Every record needs its author field, including an empty author.
    Author,
    /// The author timestamp must be a supported Unix instant.
    Time,
    /// Every record needs its complete subject, including an empty subject.
    Subject,
    /// A truncated final record cannot become a complete history.
    Terminator,
    /// HEAD must resolve to a local symbolic branch.
    Branch,
    /// Most-ahead selection requires both exact distance counts.
    Distance,
    /// A full history identity must contain a complete SHA-1 or SHA-256 ID.
    Object,
}

/// Original command, UTF-8, value and numeric causes remain typed.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// The protected runner retains its original cause.
    #[error("git could not read branch history")]
    Git(#[from] GitError),
    /// History metadata must be valid UTF-8.
    #[error("git printed history that is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    /// Object IDs and ref names are validated at decoding.
    #[error("git printed an invalid history value")]
    Value(#[from] InvalidText),
    /// Counters and timestamps retain numeric parse failures.
    #[error("git printed an invalid history count or timestamp")]
    Number(#[from] std::num::ParseIntError),
    /// A missing or malformed field fails the complete read.
    #[error("git printed an incomplete or invalid history field: {0:?}")]
    Record(HistoryField),
    /// An explicitly qualified base is never silently substituted.
    #[error("the recorded base branch is unavailable: {0}")]
    MissingBase(RefName),
}

/// Node-resolved directory and base facts from the same registry snapshot.
#[derive(Debug, Clone, Copy)]
pub struct HistoryQuery<'a> {
    /// The admitted checkout, resolved again by the node for this read.
    pub directory: &'a Path,
    /// The recorded base, including its qualified identity when present.
    pub base: Option<&'a RefName>,
    /// The project default from the same authority snapshot, for stale names.
    pub project_default: Option<&'a RefName>,
}

impl Git {
    /// Read classified complete workspace history and bounded fork context.
    ///
    /// # Errors
    /// Refuses missing qualified bases, invalid metadata, protected command
    /// failures and output limits instead of returning incomplete history.
    pub async fn workspace_history(
        &self,
        query: HistoryQuery<'_>,
    ) -> Result<GitHistory, HistoryError> {
        let Some((head, branch)) = self.history_checkout(query.directory).await? else {
            return Ok(GitHistory {
                base: None,
                commits: Vec::new(),
            });
        };
        let base = self.history_base(query, &branch).await?;
        let workspace = match &base {
            Some(base) => {
                self.history_records(query.directory, &format!("{}..{head}", base.id), None)
                    .await?
            }
            None => Vec::new(),
        };
        let context = match &base {
            Some(base) => self.history_fork(query.directory, &head, &base.id).await?,
            None => Some(head.clone()),
        };
        let ancestors = match context {
            Some(commit) => {
                self.history_records(query.directory, commit.as_str(), Some(BASE_CONTEXT))
                    .await?
            }
            None => Vec::new(),
        };
        let local = self.history_unpublished(query.directory, &head).await?;
        let commits = workspace
            .into_iter()
            .map(|record| classified(record, GitCommitLocation::Workspace, &local))
            .chain(
                ancestors
                    .into_iter()
                    .map(|record| classified(record, GitCommitLocation::Base, &local)),
            )
            .collect();
        Ok(GitHistory {
            base: base.map(|base| base.name),
            commits,
        })
    }

    async fn history_records(
        &self,
        dir: &Path,
        revision: &str,
        limit: Option<u32>,
    ) -> Result<Vec<decode::Metadata>, HistoryError> {
        // revision is built only from parsed object IDs in this module; it
        // cannot be supplied as a free-form client argument.
        let mut command = self
            .command(dir)
            .args(["log", "-z", "--no-color", "--no-patch", FORMAT]);
        if let Some(limit) = limit {
            command = command.arg(format!("--max-count={limit}"));
        }
        let output = command
            .args(["--end-of-options", revision, "--"])
            .output()
            .await?
            .checked()?;
        decode::records(&output.stdout)
    }

    async fn history_unpublished(
        &self,
        dir: &Path,
        head: &CommitId,
    ) -> Result<HashSet<CommitId>, HistoryError> {
        let output = self
            .command(dir)
            .args(["rev-list", head.as_str(), "--not", "--remotes", "--"])
            .output()
            .await?
            .checked()?;
        std::str::from_utf8(&output.stdout)?
            .lines()
            .map(|line| line.parse().map_err(HistoryError::from))
            .collect()
    }

    async fn history_fork(
        &self,
        dir: &Path,
        head: &CommitId,
        base: &CommitId,
    ) -> Result<Option<CommitId>, HistoryError> {
        let output = self
            .command(dir)
            .args(["merge-base", head.as_str(), base.as_str()])
            .output()
            .await?;
        if output.code == Some(1) && !output.stdout_cut {
            return Ok(None);
        }
        let output = output.checked()?;
        Ok(Some(
            std::str::from_utf8(&output.stdout)?
                .trim_end_matches(['\n', '\r'])
                .parse()?,
        ))
    }
}

fn classified(
    record: decode::Metadata,
    location: GitCommitLocation,
    local: &HashSet<CommitId>,
) -> GitHistoryCommit {
    let publication = if local.contains(&record.info.id) {
        GitCommitPublication::Local
    } else {
        GitCommitPublication::Published
    };
    GitHistoryCommit {
        commit: record.info,
        short: record.short,
        location,
        publication,
    }
}

#[cfg(test)]
mod tests;
