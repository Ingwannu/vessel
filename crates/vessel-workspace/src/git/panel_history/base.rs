//! History deliberately uses the most-ahead stream, unlike comparison's
//! origin-first selection. Qualified refs always retain their exact identity.
use std::path::Path;

use vessel_proto::workspace::{BranchName, CommitId, RefName};

use super::{HistoryError, HistoryField, HistoryQuery};
use crate::git::Git;

pub(super) struct Base {
    pub(super) name: RefName,
    pub(super) id: CommitId,
}

impl Git {
    pub(super) async fn history_checkout(
        &self,
        dir: &Path,
    ) -> Result<Option<(CommitId, BranchName)>, HistoryError> {
        let output = self
            .command(dir)
            .args(["symbolic-ref", "--quiet", "HEAD"])
            .output()
            .await?;
        if output.code == Some(1) && !output.stdout_cut {
            return Ok(None);
        }
        let output = output.checked()?;
        let name = std::str::from_utf8(&output.stdout)?.trim_end_matches(['\r', '\n']);
        let reference: RefName = name.parse()?;
        let branch: BranchName = name
            .strip_prefix("refs/heads/")
            .ok_or(HistoryError::Record(HistoryField::Branch))?
            .parse()?;
        // An unborn checkout has a symbolic branch, but no commit history.
        Ok(self
            .exact_branch_commit(dir, &reference)
            .await?
            .map(|head| (head, branch)))
    }

    pub(super) async fn history_base(
        &self,
        query: HistoryQuery<'_>,
        current: &BranchName,
    ) -> Result<Option<Base>, HistoryError> {
        let initial = query.base.or(query.project_default);
        let Some(initial) = initial else {
            return Ok(None);
        };
        if branch_name(initial) == current.as_str() {
            return Ok(None);
        }
        if let Some(base) = self.history_resolve_base(query.directory, initial).await? {
            return Ok(Some(base));
        }
        // Only a missing unqualified saved name may fall back. A qualified
        // missing ref already returned a typed refusal and cannot get here.
        match query.project_default {
            Some(default) if branch_name(default) != current.as_str() && default != initial => {
                self.history_resolve_base(query.directory, default).await
            }
            Some(_) | None => Ok(None),
        }
    }

    async fn history_resolve_base(
        &self,
        dir: &Path,
        name: &RefName,
    ) -> Result<Option<Base>, HistoryError> {
        let text = name.as_str();
        if text.starts_with("refs/heads/") || text.starts_with("refs/remotes/") {
            return self
                .exact_branch_commit(dir, name)
                .await?
                .map(|id| Base {
                    name: name.clone(),
                    id,
                })
                .map(Some)
                .ok_or_else(|| HistoryError::MissingBase(name.clone()));
        }
        let local = branch_name(name);
        let origin = format!("origin/{local}");
        let local_ref: RefName = format!("refs/heads/{local}").parse()?;
        let origin_ref: RefName = format!("refs/remotes/{origin}").parse()?;
        let (local_id, origin_id) = tokio::try_join!(
            self.exact_branch_commit(dir, &local_ref),
            self.exact_branch_commit(dir, &origin_ref),
        )?;
        let selected = match (local_id, origin_id) {
            (None, None) => None,
            (Some(id), None) => Some((local, id)),
            (None, Some(id)) => Some((origin.as_str(), id)),
            (Some(local_id), Some(origin_id)) => {
                if self
                    .history_origin_ahead(dir, &local_id, &origin_id)
                    .await?
                {
                    Some((origin.as_str(), origin_id))
                } else {
                    Some((local, local_id))
                }
            }
        };
        selected
            .map(|(name, id)| {
                Ok(Base {
                    name: name.parse()?,
                    id,
                })
            })
            .transpose()
    }

    async fn history_origin_ahead(
        &self,
        dir: &Path,
        local: &CommitId,
        origin: &CommitId,
    ) -> Result<bool, HistoryError> {
        let range = format!("{local}...{origin}");
        let output = self
            .command(dir)
            .args(["rev-list", "--left-right", "--count", &range, "--"])
            .output()
            .await?
            .checked()?;
        let mut parts = std::str::from_utf8(&output.stdout)?.split_whitespace();
        let local = parts
            .next()
            .ok_or(HistoryError::Record(HistoryField::Distance))?
            .parse::<u64>()?;
        let remote = parts
            .next()
            .ok_or(HistoryError::Record(HistoryField::Distance))?
            .parse::<u64>()?;
        if parts.next().is_some() {
            return Err(HistoryError::Record(HistoryField::Distance));
        }
        Ok(remote > local)
    }
}

fn branch_name(name: &RefName) -> &str {
    let text = name.as_str();
    if let Some(local) = text.strip_prefix("refs/heads/") {
        return local;
    }
    if let Some(remote) = text.strip_prefix("refs/remotes/") {
        return remote.split_once('/').map_or(remote, |(_, branch)| branch);
    }
    text.strip_prefix("origin/").unwrap_or(text)
}
