//! GitHub repository search for Add project (ADR 0088, part 1): Paseo's
//! `workspace.github.search_repositories`, as `forge.repos`.
//!
//! [Decision Log]
//! - Purpose: the Add project sheet's "Clone from GitHub" page lists the
//!   signed-in account's repositories and searches GitHub (TRACKER 440);
//!   `project.clone` already clones what a row names.
//! - Constraint: the node reaches GitHub with its own client and the
//!   machine's token (ADR 0029); every value a forge supplies is bounded
//!   and decodes on every client.
//! - Choice: one read with Paseo's query and limit; the answer reuses
//!   `ForgeAccess` for why it could not read (Paseo's unavailable,
//!   unauthenticated and error statuses), so clients word one set of
//!   reasons. The clone URL is the node's (it knows `gh`'s protocol).
//! - Dropped from Paseo's row: the numeric id and the bare name (the
//!   owner/name path is unique in one answer, and the name is its last
//!   segment).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use super::access::ForgeAccess;
use super::names::{InvalidForgeValue, RepoDescription, RepoPath, RepoQuery};
use crate::ProtocolVersion;
use crate::workspace::CloneUrl;

/// `forge.repos` (ADR 0088). Drafted as 36, the next after impl P's 35;
/// the integrator assigns it in merge order.
pub const REPOSITORY_SEARCH_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(36));

/// `forge.repos`: the signed-in account's GitHub repositories, or a search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchRepositories {
    /// What to search for; empty lists the account's own repositories.
    #[serde(default)]
    pub query: RepoQuery,
    /// How many to answer at most.
    #[serde(default)]
    pub limit: RepoLimit,
}

/// How many repositories `forge.repos` answers: 1 to 50, as Paseo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct RepoLimit(u8);

impl RepoLimit {
    /// The fewest.
    pub const MIN: u8 = 1;
    /// The most (Paseo's wire bound).
    pub const MAX: u8 = 50;
    /// Paseo's server default.
    pub const DEFAULT: Self = Self(20);

    /// `count`, when it is from [`Self::MIN`] to [`Self::MAX`].
    ///
    /// # Errors
    /// Out of range.
    pub fn new(count: u8) -> Result<Self, InvalidForgeValue> {
        if (Self::MIN..=Self::MAX).contains(&count) {
            Ok(Self(count))
        } else {
            Err(InvalidForgeValue {
                kind: "repository limit",
                rule: "1 to 50",
            })
        }
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for RepoLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<'de> Deserialize<'de> for RepoLimit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u8::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Who may see a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Everyone.
    Public,
    /// Those given access.
    Private,
    /// An enterprise's members.
    Internal,
    /// A visibility a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// One repository as a search row names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Repository {
    /// `owner/name` on github.com.
    pub path: RepoPath,
    /// Its description, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<RepoDescription>,
    /// Who may see it.
    pub visibility: Visibility,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// What `project.clone` fetches: `https` or `ssh`, as the machine's
    /// `gh` is set up to clone.
    pub clone_url: CloneUrl,
}

/// What `forge.repos` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RepositoryList {
    /// `ready` when the node read GitHub, else why it could not.
    pub access: ForgeAccess,
    /// At most the asked limit, newest first; empty unless `ready`.
    pub repositories: Vec<Repository>,
}

impl RepositoryList {
    /// The most rows an answer holds ([`RepoLimit::MAX`]).
    pub const MAX: usize = 50;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limit_is_one_to_fifty_and_twenty_by_default() {
        assert!(serde_json::from_str::<RepoLimit>("0").is_err());
        assert!(serde_json::from_str::<RepoLimit>("51").is_err());
        assert_eq!(serde_json::from_str::<RepoLimit>("50").unwrap().get(), 50);
        assert_eq!(RepositoryList::MAX, usize::from(RepoLimit::MAX));
        let asked: SearchRepositories = serde_json::from_str("{}").unwrap();
        assert_eq!(asked.limit.get(), 20);
        assert_eq!(asked.query.trimmed(), "");
    }

    #[test]
    fn a_query_is_one_line_of_at_most_256_characters() {
        let asked: SearchRepositories =
            serde_json::from_str(r#"{"query":"  vessel ","limit":30}"#).unwrap();
        assert_eq!(asked.query.trimmed(), "vessel");
        assert!(serde_json::from_str::<SearchRepositories>(r#"{"query":"a\nb"}"#).is_err());
        let long = format!(r#"{{"query":"{}"}}"#, "a".repeat(257));
        assert!(serde_json::from_str::<SearchRepositories>(&long).is_err());
    }

    #[test]
    fn a_description_from_a_forge_is_one_bounded_line() {
        let text = format!("one\u{202e}\n{}", "x".repeat(400));
        let description = RepoDescription::lossy(&text).unwrap();
        assert!(description.as_str().starts_with("one x"));
        assert_eq!(description.as_str().chars().count(), 350);
        assert!(RepoDescription::lossy(" \n ").is_none());
    }

    #[test]
    fn an_unknown_visibility_still_decodes() {
        let row: Repository = serde_json::from_value(serde_json::json!({
            "path": "owner/name",
            "visibility": "secret",
            "updated_at": "2026-11-19T00:00:00Z",
            "clone_url": "git@github.com:owner/name.git",
        }))
        .unwrap();
        assert_eq!(row.visibility, Visibility::Unknown);
        assert!(row.description.is_none());
    }
}
