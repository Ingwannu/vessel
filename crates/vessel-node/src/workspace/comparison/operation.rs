//! The cache owns the same complete reads: no private index path or parser is
//! exposed to its callers. Its cancellation token reaches all preparation.
use super::cache::{Key, Snapshot, Subject};
use super::{compared, pinned_base};
use crate::workspace::{WorkspaceError, Workspaces};
use std::path::PathBuf;
use std::sync::Arc;
use vessel_proto::workspace::{
    CompareGit, GitCompared, GitComparison, GitWhitespace, WorktreeBase,
};
use vessel_workspace::Git;
use vessel_workspace::git::Comparison;

pub(super) struct Operation {
    git: Git,
    directory: PathBuf,
    base: Option<WorktreeBase>,
    scratch: PathBuf,
    comparison: GitComparison,
    whitespace: GitWhitespace,
    subject: Subject,
}

impl Operation {
    fn query(&self) -> Comparison<'_> {
        Comparison {
            directory: &self.directory,
            kind: &self.comparison,
            base: self.base.as_ref(),
            whitespace: self.whitespace,
            scratch: &self.scratch,
        }
    }

    pub(super) async fn run(
        mut self,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Snapshot, WorkspaceError> {
        self.git = self.git.cancellable(cancel);
        match &self.subject {
            Subject::Summary => compared(
                self.git
                    .comparison_changes(self.query())
                    .await
                    .map_err(WorkspaceError::from),
            )
            .map(Snapshot::Summary),
            Subject::File { path, offset } => compared(
                self.git
                    .comparison_file(self.query(), path, *offset)
                    .await
                    .map_err(WorkspaceError::from),
            )
            .map(Snapshot::File),
        }
    }
}

/// Keep the owned query context in one allocation until it is transferred to
/// tracked work; cache retries retain no borrowed registry snapshot.
pub(super) enum Prepared {
    Refused(Arc<Snapshot>),
    Read(Box<ReadOperation>),
}
pub(super) struct ReadOperation {
    pub(super) key: Key,
    pub(super) operation: Operation,
}

impl Workspaces {
    pub(super) async fn prepare_comparison(
        &self,
        request: &CompareGit,
        subject: Subject,
    ) -> Result<Prepared, WorkspaceError> {
        let resolved = self.resolve_git(request.root).await?;
        let comparison =
            match pinned_base::admit(resolved.pinned_base.as_ref(), request.comparison.clone()) {
                Ok(comparison) => comparison,
                Err(pinned_base::BaseRefusal::Mismatch { stored, requested }) => {
                    return Ok(Prepared::Refused(Arc::new(match subject {
                        Subject::Summary => {
                            Snapshot::Summary(GitCompared::BaseMismatch { stored, requested })
                        }
                        Subject::File { .. } => {
                            Snapshot::File(GitCompared::BaseMismatch { stored, requested })
                        }
                    })));
                }
            };
        let mut admitted = request.clone();
        admitted.comparison = comparison.clone();
        let key = Key::new(
            resolved.root.clone(),
            admitted,
            resolved.base(),
            subject.clone(),
        );
        let operation = Operation {
            git: self.git()?.clone(),
            directory: resolved.root.path().to_owned(),
            base: resolved.base(),
            scratch: self.dirs.scratch.clone(),
            comparison,
            whitespace: request.whitespace,
            subject,
        };
        Ok(Prepared::Read(Box::new(ReadOperation { key, operation })))
    }
}
