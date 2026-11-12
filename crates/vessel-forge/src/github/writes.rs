//! GitHub's writes, each a person's (or an agent's) call: opening a pull
//! request, merging one at a pinned head, and the one permission read
//! feedback asks before quoting someone.

use reqwest::{Method, StatusCode};
use vessel_proto::forge::{ForgeLogin, MergeMethod, PullNumber, RefusedReason, RepoRef};
use vessel_proto::workspace::CommitId;

use super::{Ask, GitHub, decode, error_said, no_access, rest};
use crate::http::{Reply, Resource};
use crate::provider::{ForgeError, Merged, Opened, Opening, Priority};

impl GitHub {
    /// `POST pulls`; an existing one and an empty one are told by the
    /// words of their 422 (GitHub has no code for either).
    pub(super) async fn create(
        &self,
        repo: &RepoRef,
        opening: &Opening,
    ) -> Result<Opened, ForgeError> {
        let request = serde_json::json!({
            "title": opening.title.as_str(),
            "head": opening.head.as_str(),
            "base": opening.base.as_str(),
            "body": opening.body.as_str(),
            "draft": opening.draft,
        });
        let url = self.rest(&repo.host, &format!("repos/{}/pulls", repo.path))?;
        let ask = Ask::json(
            Method::POST,
            url,
            Resource::Core,
            Priority::Person,
            &request,
        )?;
        let Reply::Body { status, body } = self.send(&repo.host, ask).await? else {
            return Err(ForgeError::Unexpected {
                what: "a created pull request",
            });
        };
        if status.is_success() {
            let created: rest::Created = decode(&body, "a created pull request")?;
            return PullNumber::new(created.number)
                .map(Opened::Created)
                .map_err(|_bad| ForgeError::Unexpected { what: "a number" });
        }
        let said = error_said(&body);
        let lower = said.to_ascii_lowercase();
        if status == StatusCode::UNPROCESSABLE_ENTITY && lower.contains("already exists") {
            return Ok(Opened::Exists);
        }
        if status == StatusCode::UNPROCESSABLE_ENTITY && lower.contains("no commits between") {
            return Ok(Opened::NoCommits);
        }
        if let Some(error) = no_access(status) {
            return Err(error);
        }
        Ok(Opened::Rejected(said))
    }

    /// `PUT pulls/{n}/merge` with `sha`: GitHub answers 409 when the head
    /// is no longer `head`, so nothing but the commit a person saw merges.
    pub(super) async fn merge_at(
        &self,
        repo: &RepoRef,
        number: PullNumber,
        method: MergeMethod,
        head: &CommitId,
    ) -> Result<Merged, ForgeError> {
        let method_name = match method {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        };
        let request = serde_json::json!({ "merge_method": method_name, "sha": head.as_str() });
        let url = self.rest(
            &repo.host,
            &format!("repos/{}/pulls/{number}/merge", repo.path),
        )?;
        let ask = Ask::json(Method::PUT, url, Resource::Core, Priority::Person, &request)?;
        let Reply::Body { status, body } = self.send(&repo.host, ask).await? else {
            return Err(ForgeError::Unexpected {
                what: "a merge's answer",
            });
        };
        if status.is_success() {
            let answer: rest::MergeAnswer = decode(&body, "a merge's answer")?;
            return Ok(match answer.sha.and_then(|sha| sha.parse().ok()) {
                Some(commit) if answer.merged => Merged::Done(commit),
                Some(_) | None => Merged::NotMergeable(None),
            });
        }
        let said = error_said(&body);
        let lower = said.to_ascii_lowercase();
        Ok(match status.as_u16() {
            409 => Merged::HeadMoved,
            405 if lower.contains("merge queue") => Merged::MergeQueue,
            405 if lower.contains("not allowed") => Merged::MethodNotAllowed,
            405 | 422 => Merged::NotMergeable((!said.is_empty()).then_some(said)),
            404 | 410 | 451 => {
                return Err(ForgeError::Refused {
                    reason: RefusedReason::NoAccess,
                });
            }
            _ => Merged::Rejected(said),
        })
    }

    /// `GET collaborators/{login}/permission`: write, maintain or admin.
    /// Not a collaborator (404), or no right to ask: no write.
    pub(super) async fn writes_to(
        &self,
        repo: &RepoRef,
        login: &ForgeLogin,
    ) -> Result<bool, ForgeError> {
        let url = self.rest(
            &repo.host,
            &format!("repos/{}/collaborators/{login}/permission", repo.path),
        )?;
        match self
            .send(&repo.host, Ask::get(url, Priority::Background))
            .await?
        {
            Reply::Body { status, body } if status.is_success() => {
                let permission: rest::Permission = decode(&body, "a permission")?;
                Ok(matches!(
                    permission.permission.as_str(),
                    "admin" | "maintain" | "write"
                ))
            }
            Reply::Body { .. } | Reply::NotModified => Ok(false),
        }
    }
}
