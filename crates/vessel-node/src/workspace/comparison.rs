//! ADR0087 reads use the existing root authority and tracker. A reply receiver
//! owns no cleanup task; the node remains responsible after disconnection.

mod jobs;
mod pinned_base;

use std::future::Future;
use std::path::PathBuf;

use vessel_proto::workspace::{
    CompareGit, CompareGitFile, DiffSummary, FileDiff, GitCompared, GitComparison, GitWhitespace,
    WorktreeBase,
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
    comparison: GitComparison,
    whitespace: GitWhitespace,
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
}

impl Workspaces {
    /// The pending comparison summary, with an honest absent-base/limit state.
    pub(crate) async fn git_compare(
        &self,
        request: CompareGit,
    ) -> Result<GitCompared<DiffSummary>, WorkspaceError> {
        self.run_comparison(request, move |operation| async move {
            operation.git.comparison_changes(operation.query()).await
        })
        .await
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
        self.run_comparison(request.query, move |operation| async move {
            operation
                .git
                .comparison_file(operation.query(), &request.path, request.from_hunk)
                .await
        })
        .await
    }

    async fn run_comparison<T, F, Make>(
        &self,
        request: CompareGit,
        make: Make,
    ) -> Result<GitCompared<T>, WorkspaceError>
    where
        T: Send + 'static,
        F: Future<Output = Result<Option<T>, ComparisonError>> + Send + 'static,
        Make: FnOnce(Operation) -> F,
    {
        // This precedes both reservation and I/O; a future cache hit must keep
        // the same revalidation rather than trusting a previously resolved root.
        let resolved = self.resolve_git(request.root).await?;
        let comparison = match pinned_base::admit(resolved.pinned_base.as_ref(), request.comparison)
        {
            Ok(comparison) => comparison,
            Err(pinned_base::BaseRefusal::Mismatch { stored, requested }) => {
                return Ok(GitCompared::BaseMismatch { stored, requested });
            }
        };
        let operation = Operation {
            git: self.git()?.cancellable(self.stop.child_token()),
            directory: resolved.root.path().to_owned(),
            base: resolved.base(),
            scratch: self.dirs.scratch.clone(),
            comparison,
            whitespace: request.whitespace,
        };
        let pending = self
            .comparisons
            .submit(&self.tasks, self.stop.clone(), make(operation))?;
        compared(pending.result().await?.map_err(WorkspaceError::from))
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
