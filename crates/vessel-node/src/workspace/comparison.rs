//! ADR0087 reads use the existing root authority and tracker. A reply receiver
//! owns no cleanup task; the node remains responsible after disconnection.

mod jobs;

use std::future::Future;
use std::path::PathBuf;

use vessel_proto::workspace::{
    CompareGit, CompareGitFile, DiffSummary, FileDiff, GitCompared, GitComparison, GitWhitespace,
    WorkspaceRoot, WorktreeBase,
};
use vessel_workspace::git::{Comparison, ComparisonError};
use vessel_workspace::{Git, GitError};

use super::{WorkspaceError, Workspaces};

pub(crate) use jobs::JobError;
pub(super) use jobs::Jobs;

/// Prepared from one freshly validated workspace, never a client path.
struct Operation {
    git: Git,
    directory: PathBuf,
    base: Option<WorktreeBase>,
    scratch: PathBuf,
}

impl Operation {
    fn query<'a>(&'a self, kind: &'a GitComparison, whitespace: GitWhitespace) -> Comparison<'a> {
        Comparison {
            directory: &self.directory,
            kind,
            base: self.base.as_ref(),
            whitespace,
            scratch: &self.scratch,
        }
    }
}

impl Workspaces {
    /// The pending comparison summary, with an honest absent-base/limit state.
    pub(crate) async fn git_compare(
        &self,
        request: CompareGit,
    ) -> Result<GitCompared<DiffSummary>, WorkspaceError> {
        let read = self
            .run_comparison(request.root, move |operation| async move {
                operation
                    .git
                    .comparison_changes(operation.query(&request.comparison, request.whitespace))
                    .await
            })
            .await;
        compared(read)
    }

    /// A bounded file body in the identical comparison/whitespace policy.
    pub(crate) async fn git_compare_file(
        &self,
        request: CompareGitFile,
    ) -> Result<GitCompared<FileDiff>, WorkspaceError> {
        if request.path.is_root() {
            return Err(WorkspaceError::Invalid(
                "a comparison body needs a file path".to_owned(),
            ));
        }
        let read = self
            .run_comparison(request.query.root, move |operation| async move {
                operation
                    .git
                    .comparison_file(
                        operation.query(&request.query.comparison, request.query.whitespace),
                        &request.path,
                        request.from_hunk,
                    )
                    .await
            })
            .await;
        compared(read)
    }

    async fn run_comparison<T, F, Make>(
        &self,
        root: WorkspaceRoot,
        make: Make,
    ) -> Result<T, WorkspaceError>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, ComparisonError>> + Send + 'static,
        Make: FnOnce(Operation) -> F,
    {
        // This precedes both reservation and I/O; a future cache hit must keep
        // the same revalidation rather than trusting a previously resolved root.
        let resolved = self.resolve_git(root).await?;
        let operation = Operation {
            git: self.git()?.cancellable(self.stop.child_token()),
            directory: resolved.root.path().to_owned(),
            base: resolved.base(),
            scratch: self.dirs.scratch.clone(),
        };
        let pending = self
            .comparisons
            .submit(&self.tasks, self.stop.clone(), make(operation))?;
        Ok(pending.result().await??)
    }
}

fn compared<T>(read: Result<Option<T>, WorkspaceError>) -> Result<GitCompared<T>, WorkspaceError> {
    match read {
        Ok(Some(data)) => Ok(GitCompared::Ready { data }),
        Ok(None) => Ok(GitCompared::NoBase),
        Err(WorkspaceError::Git(GitError::OutputLimit)) => Ok(GitCompared::TooLarge),
        Err(error) => Err(error),
    }
}
