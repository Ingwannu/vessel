//! GitHub (and GitHub Enterprise) behind the [`Forge`] trait: REST for
//! what has conditional requests (finding a branch's pull request, whether
//! one changed, its checks and statuses) and for writes, GraphQL for the
//! one whole read (ADR 0029).
//!
//! [Decision Log]
//! - Purpose: the cheap path (nothing changed) must cost nothing against
//!   GitHub's quota: a 304 to a conditional REST request is free, and
//!   GraphQL has no conditional requests (agent-orchestrator's `ETag`
//!   checks, GOAT's fingerprint read).
//! - Choice: `changed` asks three conditional REST requests (the pull
//!   request, its head's check runs as a full page, its statuses); `read`
//!   is one GraphQL query (`graphql::READ`). Merging pins the head with
//!   REST's `sha` (409 when it moved); opening is REST's `POST pulls`, an
//!   existing one told by its 422. Reads are in `reads`, writes in
//!   `writes`; the trait's methods only box them.
//! - API bases: `github.com` is `api.github.com`; `*.ghe.com` is `api.` and
//!   the host; any other host is `https://<host>/api/v3` and
//!   `/api/graphql` (GitHub Enterprise Server), as GOAT reaches them.

mod facts;
mod graphql;
mod project;
mod reads;
mod rest;
mod writes;

use std::sync::Arc;

use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use vessel_proto::forge::{
    ForgeHost, ForgeKind, ForgeLogin, MergeMethod, OpenPullPage, PullCursor, PullNumber,
    RefusedReason, RepoRef,
};
use vessel_proto::workspace::{BranchName, CommitId};

use crate::http::{Call, Http, Reply, Resource};
use crate::provider::{
    Answer, Forge, ForgeError, Found, Fresh, Merged, Opened, Opening, Priority, Read,
};
use crate::token::{Credentials, Found as TokenFound, Token};

/// GitHub, through one HTTP client and the node's credentials.
#[derive(Debug)]
pub struct GitHub {
    http: Http,
    credentials: Arc<Credentials>,
    /// The API's base for every host, instead of GitHub's (the tests'
    /// loopback stand-in).
    api: Option<Url>,
}

/// One request as a read or a write asks it; [`GitHub::send`] adds the
/// host's token and every bound.
#[derive(Debug)]
struct Ask {
    method: Method,
    url: Url,
    resource: Resource,
    priority: Priority,
    /// Sent with the last `ETag` of its URL.
    conditional: bool,
    json: Option<Vec<u8>>,
}

impl Ask {
    /// A REST `GET`.
    fn get(url: Url, priority: Priority) -> Self {
        Self {
            method: Method::GET,
            url,
            resource: Resource::Core,
            priority,
            conditional: false,
            json: None,
        }
    }

    /// A REST `GET` with the URL's last `ETag`, for the background only: a
    /// person's read wants the answer itself, and a 304 would give it
    /// none (the `ETag`s are shared with the watcher's looks).
    fn conditional(url: Url, priority: Priority) -> Self {
        Self {
            conditional: priority == Priority::Background,
            ..Self::get(url, priority)
        }
    }

    /// A request with a JSON body.
    fn json(
        method: Method,
        url: Url,
        resource: Resource,
        priority: Priority,
        body: &impl serde::Serialize,
    ) -> Result<Self, ForgeError> {
        let json = serde_json::to_vec(body)
            .map_err(|_bad| ForgeError::Unexpected { what: "a request" })?;
        Ok(Self {
            method,
            url,
            resource,
            priority,
            conditional: false,
            json: Some(json),
        })
    }
}

impl GitHub {
    /// GitHub at its own addresses.
    ///
    /// # Errors
    /// The HTTP client could not be built.
    pub fn new(credentials: Arc<Credentials>) -> Result<Self, ForgeError> {
        Ok(Self {
            http: Http::new()?,
            credentials,
            api: None,
        })
    }

    /// GitHub's API at `api` for every host (`api/graphql` for GraphQL):
    /// for tests against a stand-in on loopback.
    ///
    /// # Errors
    /// The HTTP client could not be built.
    pub fn at(credentials: Arc<Credentials>, api: Url) -> Result<Self, ForgeError> {
        Ok(Self {
            http: Http::new()?,
            credentials,
            api: Some(api),
        })
    }

    /// A REST address on `host`'s API.
    fn rest(&self, host: &ForgeHost, path: &str) -> Result<Url, ForgeError> {
        let base = match &self.api {
            Some(api) => Ok(api.clone()),
            None if host.as_str() == "github.com" => Url::parse("https://api.github.com/"),
            None if host.as_str().ends_with(".ghe.com") => {
                Url::parse(&format!("https://api.{host}/"))
            }
            None => Url::parse(&format!("https://{host}/api/v3/")),
        };
        base.and_then(|base| base.join(path))
            .map_err(|_bad| address())
    }

    /// `host`'s GraphQL address.
    fn graphql(&self, host: &ForgeHost) -> Result<Url, ForgeError> {
        let url = match &self.api {
            Some(api) => api.join("graphql"),
            None if host.as_str() == "github.com" => Url::parse("https://api.github.com/graphql"),
            None if host.as_str().ends_with(".ghe.com") => {
                Url::parse(&format!("https://api.{host}/graphql"))
            }
            None => Url::parse(&format!("https://{host}/api/graphql")),
        };
        url.map_err(|_bad| address())
    }

    async fn token(&self, host: &ForgeHost) -> Result<Token, ForgeError> {
        match self.credentials.token(host).await {
            TokenFound::Token(token, _source) => Ok(token),
            TokenFound::None(gh) => Err(ForgeError::SignedOut { gh }),
        }
    }

    /// One request to `host`, its 401 dropping the cached token.
    async fn send(&self, host: &ForgeHost, ask: Ask) -> Result<Reply, ForgeError> {
        let token = self.token(host).await?;
        let sent = self
            .http
            .send(Call {
                method: ask.method,
                url: ask.url,
                host,
                token: &token,
                resource: ask.resource,
                priority: ask.priority,
                conditional: ask.conditional,
                json: ask.json,
            })
            .await;
        if let Err(ForgeError::Refused {
            reason: RefusedReason::Unauthorized,
        }) = &sent
        {
            self.credentials.refused(host);
        }
        sent
    }
}

fn address() -> ForgeError {
    ForgeError::Unexpected { what: "an address" }
}

/// `owner` and `name` of a GitHub repository (two segments).
fn owner_name(repo: &RepoRef) -> Result<(&str, &str), ForgeError> {
    repo.path
        .as_str()
        .split_once('/')
        .filter(|(_, name)| !name.contains('/'))
        .ok_or(ForgeError::Unexpected {
            what: "a GitHub repository is owner/name",
        })
}

fn decode<T: DeserializeOwned>(body: &[u8], what: &'static str) -> Result<T, ForgeError> {
    serde_json::from_slice(body).map_err(|_bad| ForgeError::Unexpected { what })
}

/// What an error answer says, at most 2 KiB of it.
fn error_said(body: &[u8]) -> String {
    let said = serde_json::from_slice::<rest::ErrorAnswer>(body)
        .map(|answer| answer.said())
        .unwrap_or_default();
    cut(&said, 2048)
}

/// At most `max` bytes of `text`, at a character boundary.
fn cut(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default().to_owned()
}

/// A 404 reading a repository: no access for this token (GitHub hides a
/// private repository's existence), or a 410 or 451.
fn no_access(status: StatusCode) -> Option<ForgeError> {
    matches!(status.as_u16(), 404 | 410 | 451).then_some(ForgeError::Refused {
        reason: RefusedReason::NoAccess,
    })
}

impl Forge for GitHub {
    fn list_open<'a>(
        &'a self,
        repo: &'a RepoRef,
        after: Option<&'a PullCursor>,
        priority: Priority,
    ) -> Answer<'a, OpenPullPage> {
        Box::pin(self.project_page(repo, after, priority))
    }
    fn kind(&self) -> ForgeKind {
        ForgeKind::Github
    }

    fn pins_head(&self, method: MergeMethod) -> bool {
        match method {
            MergeMethod::Merge | MergeMethod::Squash | MergeMethod::Rebase => true,
        }
    }

    fn whoami<'a>(&'a self, host: &'a ForgeHost) -> Answer<'a, Option<ForgeLogin>> {
        Box::pin(self.login(host))
    }

    fn find<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a BranchName,
        priority: Priority,
    ) -> Answer<'a, Fresh<Vec<Found>>> {
        Box::pin(self.pulls_of(repo, branch, priority))
    }

    fn changed<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        head: &'a CommitId,
        priority: Priority,
    ) -> Answer<'a, bool> {
        Box::pin(self.moved(repo, number, head, priority))
    }

    fn read<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        priority: Priority,
    ) -> Answer<'a, Read> {
        Box::pin(self.whole(repo, number, priority))
    }

    fn open<'a>(&'a self, repo: &'a RepoRef, opening: &'a Opening) -> Answer<'a, Opened> {
        Box::pin(self.create(repo, opening))
    }

    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        method: MergeMethod,
        head: &'a CommitId,
    ) -> Answer<'a, Merged> {
        Box::pin(self.merge_at(repo, number, method, head))
    }

    fn can_write<'a>(&'a self, repo: &'a RepoRef, login: &'a ForgeLogin) -> Answer<'a, bool> {
        Box::pin(self.writes_to(repo, login))
    }
}

#[cfg(test)]
mod tests;
