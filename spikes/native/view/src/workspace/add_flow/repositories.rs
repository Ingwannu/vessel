//! Clone from GitHub's search rows: the machine's `forge.repos` answer
//! (ADR 0088; TRACKER 440), Paseo's `add-project-flow.tsx` (pinned
//! 0.15.4) lines 447-457 (the query), 645-670 (the rows) and 186-200 (the
//! error), host as machine; the page's `add-project/repositories.ts`.
//!
//! [Decision Log]
//! - Purpose: the GitHub page lists the account's repositories and a
//!   search's, not only what was typed.
//! - Constraint: the node answers for the field as it was asked; the
//!   field may have changed since, and an older node has no such call.
//! - Choice: the answer is kept with its machine and text (as the
//!   folders are), and counts only while both still match; the typed rows
//!   come first unless the search already has that repository (Paseo's
//!   `hasExactSearchResult`).

use vessel_proto::ProtocolVersion;
use vessel_proto::forge::{
    ForgeAccess, GhState, REPOSITORY_SEARCH_PROTOCOL, Repository as Found, RepositoryList,
};

use super::model::{Origin, Repository};
use super::options::typed_repositories;

/// Paseo's rows per GitHub search.
pub const REPOSITORY_ROWS: u8 = 30;

/// Why the page has no search rows, in Paseo's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchError {
    /// "Unable to search GitHub repositories": the call itself failed.
    Failed,
    /// "GitHub search failed": the node could not search.
    Unavailable,
    /// "GitHub CLI (gh) is not installed or not in PATH".
    GhMissing,
    /// "GitHub CLI is not authenticated. Run gh auth login on the machine."
    SignedOut,
}

/// What the machine's search found for the field as it is now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Searched {
    /// The found rows, newest first.
    pub repositories: Vec<Repository>,
    /// Still asking.
    pub loading: bool,
    /// Why there are none.
    pub error: Option<SearchError>,
}

/// Whether a machine at `protocol` answers `forge.repos`.
#[must_use]
pub fn can_search(protocol: Option<ProtocolVersion>) -> bool {
    matches!((protocol, REPOSITORY_SEARCH_PROTOCOL), (Some(has), Some(needs)) if has >= needs)
}

/// Paseo's server words for a search the node could not make: a missing
/// or signed-out `gh`, else its generic failure.
#[must_use]
pub const fn access_error(access: &ForgeAccess) -> Option<SearchError> {
    match access {
        ForgeAccess::Ready { .. } => None,
        ForgeAccess::SignedOut {
            gh: GhState::Missing,
            ..
        } => Some(SearchError::GhMissing),
        ForgeAccess::SignedOut {
            gh: GhState::SignedOut | GhState::Unknown,
            ..
        } => Some(SearchError::SignedOut),
        ForgeAccess::SignedOut {
            gh: GhState::Failed,
            ..
        }
        | ForgeAccess::NotPlaced
        | ForgeAccess::NoBranch
        | ForgeAccess::OnBase
        | ForgeAccess::NoRemote
        | ForgeAccess::NotSupported { .. }
        | ForgeAccess::Refused { .. }
        | ForgeAccess::RateLimited { .. }
        | ForgeAccess::Unreachable { .. }
        | ForgeAccess::Limit
        | ForgeAccess::Ended
        | ForgeAccess::Unknown => Some(SearchError::Unavailable),
    }
}

/// A found repository as a row's choice: its owner/name as the title and
/// the node's clone URL as it is (no protocol word: the node chose it).
#[must_use]
pub fn found(row: &Found) -> Repository {
    Repository {
        id: format!("repository:{}", row.path),
        name_with_owner: row.path.as_str().to_owned(),
        clone_url: row.clone_url.as_str().to_owned(),
        protocol: None,
        origin: Origin::Found {
            description: row
                .description
                .as_ref()
                .map(|description| description.as_str().to_owned()),
        },
    }
}

/// One answer read: its rows, or why there are none (`None` for a call
/// that failed).
#[must_use]
pub fn searched(answer: Option<&RepositoryList>) -> Searched {
    let Some(list) = answer else {
        return Searched {
            error: Some(SearchError::Failed),
            ..Searched::default()
        };
    };
    Searched {
        repositories: list.repositories.iter().map(found).collect(),
        loading: false,
        error: access_error(&list.access),
    }
}

/// Paseo's GitHub rows: the typed choices first, unless the search already
/// has that repository, then the search's.
#[must_use]
pub fn github_choices(query: &str, found: &[Repository]) -> Vec<Repository> {
    let normalized = query.trim().to_lowercase();
    let exact = found.iter().any(|repository| {
        repository.name_with_owner.to_lowercase() == normalized
            || repository.clone_url.to_lowercase() == normalized
    });
    let typed = if exact {
        Vec::new()
    } else {
        typed_repositories(query)
    };
    typed.into_iter().chain(found.iter().cloned()).collect()
}

#[cfg(test)]
mod tests;
