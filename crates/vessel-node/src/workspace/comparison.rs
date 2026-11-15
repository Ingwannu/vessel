//! ADR0087 reads share bounded completed payloads and node-owned active work.
//! Every cache probe revalidates its root, including a retry after cancellation.
mod cache;
mod jobs;
mod operation;
mod pinned_base;

use super::{WorkspaceError, Workspaces};
use cache::{Owner, ReadReason, ReadStep, Snapshot, Subject};
use std::sync::Arc;
use vessel_proto::workspace::{
    CompareGit, CompareGitFile, DiffSummary, FileDiff, GitCompared, GitReadReason,
};
use vessel_workspace::GitError;

pub(super) use cache::{CacheError, Change, PayloadKind, Reader};
pub(crate) use jobs::JobError;

impl Workspaces {
    pub(crate) async fn git_compare(
        self: &Arc<Self>,
        request: CompareGit,
    ) -> Result<GitCompared<DiffSummary>, WorkspaceError> {
        let snapshot = self.run_comparison(request, Subject::Summary).await?;
        match snapshot.as_ref() {
            Snapshot::Summary(data) => Ok(data.clone()),
            Snapshot::File(_) => Err(CacheError::Payload {
                expected: PayloadKind::Summary,
                received: snapshot.kind(),
            }
            .into()),
        }
    }

    pub(crate) async fn git_compare_file(
        self: &Arc<Self>,
        request: CompareGitFile,
    ) -> Result<GitCompared<FileDiff>, WorkspaceError> {
        if request.path.is_root() {
            return Err(WorkspaceError::Invalid(
                "a comparison body needs a file path".to_owned(),
            ));
        }
        let snapshot = self
            .run_comparison(
                request.query,
                Subject::File {
                    path: request.path,
                    offset: request.from_hunk,
                },
            )
            .await?;
        match snapshot.as_ref() {
            Snapshot::File(data) => Ok(data.clone()),
            Snapshot::Summary(_) => Err(CacheError::Payload {
                expected: PayloadKind::File,
                received: snapshot.kind(),
            }
            .into()),
        }
    }

    async fn run_comparison(
        self: &Arc<Self>,
        mut request: CompareGit,
        subject: Subject,
    ) -> Result<Arc<Snapshot>, WorkspaceError> {
        loop {
            // A cached value is not root authority. After waiting for a cancelled
            // job's cleanup the next loop resolves the registry snapshot again.
            let prepared = self.prepare_comparison(&request, subject.clone()).await?;
            let read = match prepared {
                operation::Prepared::Refused(reply) => return Ok(reply),
                operation::Prepared::Read(read) => read,
            };
            let operation::ReadOperation { key, operation } = *read;
            let owner = Owner {
                tasks: &self.tasks,
                stop: &self.stop,
            };
            match self
                .comparisons
                .read(
                    key,
                    ReadReason::from(request.reason),
                    owner,
                    move |cancel| operation.run(cancel),
                )
                .await?
            {
                ReadStep::Complete(reply) => {
                    return reply.map_err(WorkspaceError::SharedComparison);
                }
                ReadStep::Retry => request.reason = GitReadReason::View,
            }
        }
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
