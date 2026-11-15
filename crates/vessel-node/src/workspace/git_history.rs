//! Complete history uses the existing root authority and retained job owner.
use super::{WorkspaceError, Workspaces};
use vessel_proto::workspace::{GitHistoryRead, ReadGitHistory};
use vessel_workspace::GitError;
use vessel_workspace::git::{HistoryError, HistoryQuery};

impl Workspaces {
    pub(crate) async fn git_history(
        &self,
        request: ReadGitHistory,
    ) -> Result<GitHistoryRead, WorkspaceError> {
        let resolved = self.resolve_git(request.root).await?;
        let directory = resolved.root.path().to_owned();
        let base = resolved.base;
        let project_default = resolved.project_default;
        let git = self.git()?.cancellable(self.stop.child_token());
        self.comparisons
            .owned(&self.tasks, self.stop.clone(), async move {
                let query = HistoryQuery {
                    directory: &directory,
                    base: base.as_ref(),
                    project_default: project_default.as_ref(),
                };
                match git.workspace_history(query).await {
                    Ok(data) => Ok(GitHistoryRead::Ready { data }),
                    Err(HistoryError::MissingBase(base)) => {
                        Ok(GitHistoryRead::BaseUnavailable { base })
                    }
                    Err(HistoryError::Git(GitError::OutputLimit)) => Ok(GitHistoryRead::TooLarge),
                    Err(HistoryError::Git(error)) => Err(WorkspaceError::from(error)),
                    Err(
                        error @ (HistoryError::Utf8(_)
                        | HistoryError::Value(_)
                        | HistoryError::Number(_)
                        | HistoryError::Record(_)),
                    ) => Err(WorkspaceError::History(error)),
                }
            })
            .await?
    }
}
