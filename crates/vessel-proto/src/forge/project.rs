//! Values for project-wide open pull requests (ADR 0061).
//!
//! These values deliberately register no call yet: the shared protocol
//! admission for an unassigned feature must exist before a client can
//! send one. Keeping a page separate from session entries prevents a
//! repository read from manufacturing a session or changing `pr.list`.

mod cursor;

use std::collections::BTreeSet;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::{PullState, PullSummary, RepoRef};
use crate::ProtocolVersion;

pub use cursor::{InvalidPullCursor, ProjectPullCursor, PullCursor};

/// Unassigned until project listing has node, client, CLI and consuming PR
/// screen support. This does not register a call or expose a new field.
pub const PROJECT_PULL_REQUESTS: Option<ProtocolVersion> = None;

/// A complete, bounded page from one repository. Invalid provider pages
/// fail whole rather than silently disappearing or looking complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenPullPage {
    repo: RepoRef,
    pulls: Vec<PullSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<PullCursor>,
    read_at: DateTime<Utc>,
}

/// Why a page cannot be represented as a successful project read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidOpenPullPage {
    /// Retaining more would hide an unbounded provider result.
    #[error("an open pull-request page exceeds {max} items", max = OpenPullPage::MAX_PULLS)]
    TooMany,
    /// Every summary must belong to the repository selected by the node.
    #[error("an open pull-request page contains another repository")]
    Repository,
    /// This list must never imply a closed request is still open.
    #[error("an open pull-request page contains a request that is not open")]
    NotOpen,
    /// A repeated number is an inconsistent whole page.
    #[error("an open pull-request page repeats a number")]
    Duplicate,
    /// A continuation must advance past at least one returned request.
    #[error("an empty open pull-request page cannot have a next cursor")]
    EmptyContinuation,
}

impl OpenPullPage {
    /// Paseo's default listing limit, fixed here so callers cannot turn
    /// one project read into an unbounded repository enumeration.
    pub const MAX_PULLS: usize = 20;

    /// Build a whole page, retaining no partial result on failure.
    ///
    /// # Errors
    /// Too many, duplicate, non-open or foreign-repository entries, or a
    /// continuation without entries.
    pub fn new(
        repo: RepoRef,
        pulls: Vec<PullSummary>,
        next: Option<PullCursor>,
        read_at: DateTime<Utc>,
    ) -> Result<Self, InvalidOpenPullPage> {
        if pulls.len() > Self::MAX_PULLS {
            return Err(InvalidOpenPullPage::TooMany);
        }
        if pulls.is_empty() && next.is_some() {
            return Err(InvalidOpenPullPage::EmptyContinuation);
        }
        let mut numbers = BTreeSet::new();
        for pull in &pulls {
            if pull.repo != repo {
                return Err(InvalidOpenPullPage::Repository);
            }
            if pull.state != PullState::Open {
                return Err(InvalidOpenPullPage::NotOpen);
            }
            if !numbers.insert(pull.number) {
                return Err(InvalidOpenPullPage::Duplicate);
            }
        }
        Ok(Self {
            repo,
            pulls,
            next,
            read_at,
        })
    }

    /// The repository whose remote was resolved for this page.
    #[must_use]
    pub const fn repo(&self) -> &RepoRef {
        &self.repo
    }

    /// Open requests in provider order; no session id is implied.
    #[must_use]
    pub fn pulls(&self) -> &[PullSummary] {
        &self.pulls
    }

    /// Whether another page exists and where it starts. The association
    /// must be checked against the project's current remote before use.
    #[must_use]
    pub fn continuation(&self) -> Option<ProjectPullCursor> {
        self.next.clone().map(|after| ProjectPullCursor {
            repo: self.repo.clone(),
            after,
        })
    }

    /// When this node actually read the page, not when a cache reused it.
    #[must_use]
    pub const fn read_at(&self) -> DateTime<Utc> {
        self.read_at
    }
}

impl<'de> Deserialize<'de> for OpenPullPage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let page = EncodedPage::deserialize(deserializer)?;
        Self::new(page.repo, page.pulls, page.next, page.read_at).map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
struct EncodedPage {
    repo: RepoRef,
    #[serde(deserialize_with = "bounded_pulls")]
    pulls: Vec<PullSummary>,
    #[serde(default)]
    next: Option<PullCursor>,
    read_at: DateTime<Utc>,
}

// A length check after Vec::deserialize would already have allocated the
// provider's entire array. Decode only the admitted prefix, then reject
// on one extra element; construction separately checks the page's facts.
fn bounded_pulls<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<PullSummary>, D::Error> {
    struct Pulls;
    impl<'de> Visitor<'de> for Pulls {
        type Value = Vec<PullSummary>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {} open pull requests", OpenPullPage::MAX_PULLS)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut pulls = Vec::new();
            while pulls.len() < OpenPullPage::MAX_PULLS {
                let Some(pull) = seq.next_element()? else {
                    return Ok(pulls);
                };
                pulls.push(pull);
            }
            if seq.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom(InvalidOpenPullPage::TooMany));
            }
            Ok(pulls)
        }
    }
    deserializer.deserialize_seq(Pulls)
}

#[cfg(test)]
mod tests;
