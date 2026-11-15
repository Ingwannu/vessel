//! Project reads share the existing credential/HTTP boundary. A page is
//! never conditional, and GraphQL partial success is a whole refusal.

use chrono::Utc;
use reqwest::Method;
use serde::Serialize;
use vessel_proto::forge::{OpenPullPage, PullCursor, RepoRef};

use super::{Ask, GitHub, decode, graphql, no_access, owner_name};
use crate::http::{Reply, Resource};
use crate::provider::{ForgeError, Priority};

#[derive(Serialize)]
struct Variables<'a> {
    owner: &'a str,
    name: &'a str,
    first: graphql::QueryLimit,
    after: Option<&'a str>,
    contexts: graphql::QueryLimit,
    heads: graphql::QueryLimit,
}

impl GitHub {
    pub(super) async fn project_page(
        &self,
        repo: &RepoRef,
        after: Option<&PullCursor>,
        priority: Priority,
    ) -> Result<OpenPullPage, ForgeError> {
        let (owner, name) = owner_name(repo)?;
        let query = graphql::Query {
            query: graphql::LIST,
            variables: Variables {
                owner,
                name,
                first: graphql::PROJECT_PAGE_LIMIT,
                after: after.map(PullCursor::as_str),
                contexts: graphql::MAX_CONTEXTS,
                heads: graphql::HEAD_COMMITS,
            },
        };
        let ask = Ask::json(
            Method::POST,
            self.graphql(&repo.host)?,
            Resource::Graphql,
            priority,
            &query,
        )?;
        let Reply::Body { status, body } = self.send(&repo.host, ask).await? else {
            return Err(ForgeError::Unexpected {
                what: "a project pull-request page",
            });
        };
        if !status.is_success() {
            return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                what: "a project pull-request page",
            }));
        }
        let answer: graphql::Answer<graphql::PageData> =
            decode(&body, "a project pull-request page")?;
        if answer.limited() {
            return Err(ForgeError::RateLimited {
                until: self.http.limited(&repo.host),
            });
        }
        if !answer.errors.is_empty() {
            return Err(ForgeError::Unexpected {
                what: "a complete project pull-request page",
            });
        }
        let data = answer.data.ok_or(ForgeError::Unexpected {
            what: "project pull-request data",
        })?;
        graphql::page(repo, data, after, Utc::now())
    }
}
