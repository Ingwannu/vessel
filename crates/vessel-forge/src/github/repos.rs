//! GitHub repository search for Add project (ADR 0088): the account's own
//! repositories (`GET /user/repos`, as `gh repo list`) or GitHub's search
//! (`GET /search/repositories`, as `gh search repos --sort updated`), with
//! clone URLs in the protocol the machine's `gh` clones with.
//!
//! [Decision Log]
//! - Purpose: Paseo's "Clone from GitHub" page lists and searches the
//!   repositories the signed-in account can clone (TRACKER 440).
//! - Choice: REST through this crate's client (ADR 0029: no `gh` per
//!   request); a search counts against GitHub's search quota
//!   (`Resource::Search`), so a spent search quota holds only search.
//! - Every row a forge sends is checked: a row whose path, time or URL
//!   does not make a valid wire value is left out (the rest still list),
//!   and a description is made one bounded line.

use std::str::FromStr as _;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::forge::{
    ForgeHost, RepoDescription, RepoLimit, RepoPath, Repository, Visibility,
};
use vessel_proto::workspace::CloneUrl;

use super::{Ask, GitHub, decode, no_access};
use crate::gh::CloneProtocol;
use crate::http::{Reply, Resource};
use crate::provider::{ForgeError, Priority};

/// One repository as `/user/repos` and `/search/repositories` give it
/// (only the fields read; GitHub adds others).
#[derive(Debug, Deserialize)]
struct Listed {
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    visibility: Option<String>,
    updated_at: String,
    html_url: String,
    ssh_url: String,
}

/// `/search/repositories`'s answer.
#[derive(Debug, Deserialize)]
struct Found {
    items: Vec<Listed>,
}

impl GitHub {
    /// The account's repositories on `host` (an empty `query`) or GitHub's
    /// search for `query`, newest first, at most `limit`.
    pub(super) async fn repos_of(
        &self,
        host: &ForgeHost,
        query: &str,
        limit: RepoLimit,
    ) -> Result<Vec<Repository>, ForgeError> {
        let query = query.trim();
        let per_page = limit.get().to_string();
        let (url, resource) = if query.is_empty() {
            let mut url = self.rest(host, "user/repos")?;
            let _pairs = url
                .query_pairs_mut()
                .append_pair("affiliation", "owner")
                .append_pair("sort", "pushed")
                .append_pair("direction", "desc")
                .append_pair("per_page", &per_page);
            (url, Resource::Core)
        } else {
            let mut url = self.rest(host, "search/repositories")?;
            let _pairs = url
                .query_pairs_mut()
                .append_pair("q", query)
                .append_pair("sort", "updated")
                .append_pair("order", "desc")
                .append_pair("per_page", &per_page);
            (url, Resource::Search)
        };
        let ask = Ask {
            resource,
            ..Ask::get(url, Priority::Person)
        };
        // How the machine clones is asked beside the request (Paseo's
        // `Promise.all`), not after it.
        let protocol = async {
            match self.credentials.gh() {
                Some(gh) => gh.git_protocol(host).await,
                None => CloneProtocol::Https,
            }
        };
        let (protocol, sent) = tokio::join!(protocol, self.send(host, ask));
        let body = match sent? {
            Reply::Body { status, body } if status.is_success() => body,
            Reply::Body { status, .. } => {
                return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                    what: "a list of repositories",
                }));
            }
            Reply::NotModified => {
                return Err(ForgeError::Unexpected {
                    what: "a list of repositories",
                });
            }
        };
        let listed = if query.is_empty() {
            decode::<Vec<Listed>>(&body, "a list of repositories")?
        } else {
            decode::<Found>(&body, "a repository search")?.items
        };
        Ok(listed
            .iter()
            .filter_map(|row| repository(row, protocol))
            .take(usize::from(limit.get()))
            .collect())
    }
}

/// `row` as the wire's, or `None` when a value it needs is not valid.
fn repository(row: &Listed, protocol: CloneProtocol) -> Option<Repository> {
    let path = RepoPath::from_str(&row.full_name).ok()?;
    let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
        .ok()?
        .with_timezone(&Utc);
    let clone_url = match protocol {
        CloneProtocol::Ssh => &row.ssh_url,
        CloneProtocol::Https => &row.html_url,
    };
    Some(Repository {
        path,
        description: row.description.as_deref().and_then(RepoDescription::lossy),
        visibility: visibility(row),
        updated_at,
        clone_url: CloneUrl::from_str(clone_url).ok()?,
    })
}

/// GitHub's `visibility` when it gives one, else `private`'s.
fn visibility(row: &Listed) -> Visibility {
    match row.visibility.as_deref() {
        Some("public") => Visibility::Public,
        Some("private") => Visibility::Private,
        Some("internal") => Visibility::Internal,
        Some(_) => Visibility::Unknown,
        None if row.private => Visibility::Private,
        None => Visibility::Public,
    }
}
