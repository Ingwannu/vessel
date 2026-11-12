//! GitHub's reads: who a token is, a branch's pull requests, whether one
//! changed (three conditional requests) and one read whole (GraphQL).

use chrono::{DateTime, Utc};
use reqwest::Method;
use vessel_proto::forge::{ForgeHost, ForgeLogin, PullNumber, PullState, RefusedReason, RepoRef};
use vessel_proto::workspace::{BranchName, CommitId};

use super::{Ask, GitHub, decode, graphql, no_access, owner_name, rest};
use crate::http::{Reply, Resource};
use crate::provider::{ForgeError, Found, Fresh, Priority, Read};

impl GitHub {
    /// `GET /user`: `None` for an installation's token, which has no user.
    pub(super) async fn login(&self, host: &ForgeHost) -> Result<Option<ForgeLogin>, ForgeError> {
        let url = self.rest(host, "user")?;
        match self.send(host, Ask::get(url, Priority::Background)).await? {
            Reply::Body { status, body } if status.is_success() => {
                let user: rest::User = decode(&body, "a user")?;
                Ok(ForgeLogin::lossy(&user.login))
            }
            Reply::Body { .. } | Reply::NotModified => Ok(None),
        }
    }

    /// `GET pulls?head=owner:branch&state=all`, conditional.
    pub(super) async fn pulls_of(
        &self,
        repo: &RepoRef,
        branch: &BranchName,
        priority: Priority,
    ) -> Result<Fresh<Vec<Found>>, ForgeError> {
        let (owner, _name) = owner_name(repo)?;
        let mut url = self.rest(&repo.host, &format!("repos/{}/pulls", repo.path))?;
        let _query = url
            .query_pairs_mut()
            .append_pair("head", &format!("{owner}:{branch}"))
            .append_pair("state", "all")
            .append_pair("sort", "updated")
            .append_pair("direction", "desc")
            .append_pair("per_page", "10");
        let body = match self
            .send(&repo.host, Ask::conditional(url, priority))
            .await?
        {
            Reply::NotModified => return Ok(Fresh::Same),
            Reply::Body { status, body } if status.is_success() => body,
            Reply::Body { status, .. } => {
                return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                    what: "a list of pull requests",
                }));
            }
        };
        let listed: Vec<rest::ListedPull> = decode(&body, "a list of pull requests")?;
        Ok(Fresh::Changed(listed.iter().filter_map(found).collect()))
    }

    /// The pull request, its head's check runs and its statuses, each
    /// conditional: `true` when any of them answered anew.
    pub(super) async fn moved(
        &self,
        repo: &RepoRef,
        number: PullNumber,
        head: &CommitId,
        priority: Priority,
    ) -> Result<bool, ForgeError> {
        let mut changed = false;
        for path in [
            format!("repos/{}/pulls/{number}", repo.path),
            format!("repos/{}/commits/{head}/check-runs?per_page=100", repo.path),
            format!("repos/{}/commits/{head}/status", repo.path),
        ] {
            let url = self.rest(&repo.host, &path)?;
            match self
                .send(&repo.host, Ask::conditional(url, priority))
                .await?
            {
                Reply::NotModified => {}
                Reply::Body { status, .. } if status.is_success() => changed = true,
                Reply::Body { status, .. } => {
                    return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                        what: "a pull request's change",
                    }));
                }
            }
        }
        Ok(changed)
    }

    /// The one GraphQL read (`graphql::READ`).
    pub(super) async fn whole(
        &self,
        repo: &RepoRef,
        number: PullNumber,
        priority: Priority,
    ) -> Result<Read, ForgeError> {
        let (owner, name) = owner_name(repo)?;
        let query = serde_json::json!({
            "query": graphql::READ,
            "variables": { "owner": owner, "name": name, "number": number.get() },
        });
        let url = self.graphql(&repo.host)?;
        let ask = Ask::json(Method::POST, url, Resource::Graphql, priority, &query)?;
        let Reply::Body { status, body } = self.send(&repo.host, ask).await? else {
            return Err(ForgeError::Unexpected {
                what: "a GraphQL answer",
            });
        };
        if !status.is_success() {
            return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                what: "a GraphQL answer",
            }));
        }
        let answer: graphql::Answer = decode(&body, "a GraphQL answer")?;
        if answer.limited() {
            let until = self.http.limited(&repo.host);
            return Err(ForgeError::RateLimited { until });
        }
        let repository =
            answer
                .data
                .and_then(|data| data.repository)
                .ok_or(ForgeError::Refused {
                    reason: RefusedReason::NoAccess,
                })?;
        graphql::read(repo, repository, Utc::now())
    }
}

/// One listed pull request, `None` when GitHub's answer is not one this
/// provider can name (a number or head out of range).
fn found(pull: &rest::ListedPull) -> Option<Found> {
    Some(Found {
        number: PullNumber::new(pull.number).ok()?,
        state: match (pull.state.as_str(), pull.merged_at.is_some()) {
            ("open", _) => PullState::Open,
            (_, true) => PullState::Merged,
            ("closed", false) => PullState::Closed,
            _ => PullState::Unknown,
        },
        head: pull.head.sha.parse().ok()?,
        updated_at: pull.updated_at.parse::<DateTime<Utc>>().ok()?,
    })
}
