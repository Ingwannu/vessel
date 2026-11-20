//! Versioned file fixtures for the existing `FakeForge` (ADR 0060).
//! Parse once before listening; no runtime selector, credentials or network.
//! Only this I/O edge holds raw bytes, never a domain or wire payload.
use std::collections::HashSet;
use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::forge::{
    CheckReport, CheckTally, Checks, ForgeKind, MergeMethod, PullRequest, RepoDescription, RepoRef,
    TimelineItem, Visibility,
};

use super::{Failure, FakeForge};

/// Maximum seed bytes; checked again after reading to handle growing files.
pub const MAX_BYTES: u64 = 1_048_576;
/// Maximum repositories in one file.
pub const MAX_REPOSITORIES: usize = 16;
/// Maximum initial/runtime pulls, and retained merge attempts.
pub const MAX_PULLS: usize = 128;

/// Safe seed-reading failure; Display never includes input or source text.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// An I/O operation failed; source retained for structured inspection.
    #[error("could not read the forge fixture seed")]
    Io(#[source] std::io::Error),
    /// A path is not an ordinary file; symlinks are refused too.
    #[error("forge fixture seed must be a regular, non-symlink file")]
    NotRegular,
    /// Too many bytes, without decoding any partial result.
    #[error("forge fixture seed exceeds 1 MiB")]
    TooLarge,
    /// Malformed or invalid typed JSON, retaining location but no contents.
    #[error("invalid forge fixture seed at line {} column {}", .0.line(), .0.column())]
    Format(#[source] serde_json::Error),
    /// A complete decoded document violates a static seed rule.
    #[error(transparent)]
    Rule(#[from] SeedRule),
    /// The platform cannot represent a measured byte count.
    #[error("forge fixture seed size is unsupported on this platform")]
    Size(#[source] std::num::TryFromIntError),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
struct Version;
impl From<Version> for u16 {
    fn from(_: Version) -> Self {
        1
    }
}
impl TryFrom<u16> for Version {
    type Error = SeedRule;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value == 1 {
            Ok(Self)
        } else {
            Err(SeedRule::Version)
        }
    }
}

/// A violated file rule; messages contain no file values.
#[derive(Debug, thiserror::Error)]
pub enum SeedRule {
    /// The format version is unsupported.
    #[error("unsupported fixture seed version")]
    Version,
    /// Repository or pull count exceeds its bound.
    #[error("fixture repository or pull count exceeds its limit")]
    Count,
    /// A repository is duplicated or not GitHub-shaped.
    #[error("fixture repository is duplicated or unsupported")]
    Repository,
    /// A pull is duplicated or names another repository.
    #[error("fixture pull is duplicated or belongs to another repository")]
    Pull,
    /// Checks disagree with the summary or exceed their bound.
    #[error("fixture checks disagree or contain too many named failures")]
    Checks,
    /// Merge methods are duplicated or exceed their bound.
    #[error("fixture merge methods must be distinct and at most three")]
    Methods,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    version: Version,
    repositories: Vec<Repository>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Repository {
    repo: RepoRef,
    #[serde(default)]
    failure: Option<Failure>,
    pulls: Vec<SeededPull>,
    /// How `forge.repos` lists it (ADR 0088); every seeded repository is
    /// listed, public and with no description unless said.
    #[serde(default)]
    listing: Listing,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Listing {
    #[serde(default)]
    description: Option<RepoDescription>,
    #[serde(default)]
    visibility: Option<Visibility>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeededPull {
    pull: PullRequest,
    #[serde(default)]
    merge: Merge,
    /// What `pr.timeline` answers (ADR 0088), oldest first.
    #[serde(default)]
    timeline: Vec<TimelineItem>,
    /// Whether the timeline had more than one read takes.
    #[serde(default)]
    truncated: bool,
    /// What `pr.check` answers for these runs (ADR 0088).
    #[serde(default)]
    reports: Vec<CheckReport>,
}
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Merge {
    #[default]
    Normal,
    Refused,
}

/// A complete validated seed; fields cannot be changed after decoding.
#[derive(Debug)]
pub struct Seed {
    repositories: Vec<Repository>,
}
impl Seed {
    fn validate(raw: Raw) -> Result<Self, SeedRule> {
        let Version = raw.version;
        if raw.repositories.len() > MAX_REPOSITORIES {
            return Err(SeedRule::Count);
        }
        let mut repositories = HashSet::new();
        let mut total: usize = 0;
        for entry in &raw.repositories {
            if entry.repo.kind != ForgeKind::Github || !repositories.insert(entry.repo.clone()) {
                return Err(SeedRule::Repository);
            }
            total = total.saturating_add(entry.pulls.len());
            if total > MAX_PULLS {
                return Err(SeedRule::Count);
            }
            let mut numbers = HashSet::new();
            for item in &entry.pulls {
                let pull = &item.pull;
                if pull.summary.repo != entry.repo || !numbers.insert(pull.summary.number) {
                    return Err(SeedRule::Pull);
                }
                if pull.summary.checks != pull.checks.state
                    || pull.checks.failing.len() > Checks::MAX_FAILING
                    || pull
                        .summary
                        .tally
                        .is_some_and(|tally| Some(tally) != CheckTally::of(&pull.checks))
                {
                    return Err(SeedRule::Checks);
                }
                let mut methods: Vec<MergeMethod> = Vec::new();
                for method in &pull.merge_methods {
                    if methods.contains(method) {
                        return Err(SeedRule::Methods);
                    }
                    methods.push(*method);
                }
                if methods.len() > 3 {
                    return Err(SeedRule::Methods);
                }
            }
        }
        Ok(Self {
            repositories: raw.repositories,
        })
    }
}

impl Seed {
    /// Decode a bounded byte slice into a complete fixture.
    /// # Errors
    /// Too many bytes, malformed JSON or a violated typed seed rule.
    pub fn decode(bytes: &[u8]) -> Result<Self, SeedError> {
        if u64::try_from(bytes.len()).map_err(SeedError::Size)? > MAX_BYTES {
            return Err(SeedError::TooLarge);
        }
        let raw: Raw = serde_json::from_slice(bytes).map_err(SeedError::Format)?;
        Self::validate(raw).map_err(SeedError::Rule)
    }

    /// Read once from an operator-owned regular, non-symlink file.
    /// Call from a blocking context; the launcher does so before listening.
    /// # Errors
    /// Unsafe file kind, read failure, size limit or invalid typed seed.
    pub fn load(path: &Path) -> Result<Self, SeedError> {
        let metadata = std::fs::symlink_metadata(path).map_err(SeedError::Io)?;
        if !metadata.file_type().is_file() {
            return Err(SeedError::NotRegular);
        }
        if metadata.len() > MAX_BYTES {
            return Err(SeedError::TooLarge);
        }
        let file = std::fs::File::open(path).map_err(SeedError::Io)?;
        let metadata = file.metadata().map_err(SeedError::Io)?;
        if !metadata.is_file() {
            return Err(SeedError::NotRegular);
        }
        let mut bytes = Vec::new();
        let _read = file
            .take(MAX_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(SeedError::Io)?;
        Self::decode(&bytes)
    }

    /// Seed the real fake's provider operations, with no external fallback.
    #[must_use]
    pub fn into_forge(self) -> Arc<FakeForge> {
        let known = self
            .repositories
            .iter()
            .map(|entry| entry.repo.clone())
            .collect();
        let forge = FakeForge::fixture(known);
        for entry in self.repositories {
            if let Some(listed) = listed(&entry) {
                forge.list_repository(listed);
            }
            forge.fail_repository(entry.repo.clone(), entry.failure);
            for item in entry.pulls {
                if matches!(item.merge, Merge::Refused) {
                    forge.refuse_merge(entry.repo.clone(), item.pull.summary.number);
                }
                forge.put_timeline(
                    entry.repo.clone(),
                    item.pull.summary.number,
                    crate::Timeline {
                        items: item.timeline,
                        truncated: item.truncated,
                    },
                );
                for report in item.reports {
                    forge.put_check(entry.repo.clone(), report);
                }
                // A seed may leave the tally out: it is its checks' counts.
                let mut pull = item.pull;
                pull.summary.tally = CheckTally::of(&pull.checks);
                forge.put(
                    pull.head_branch.clone(),
                    crate::Read {
                        pull,
                        // Discussion/feedback content is deliberately outside
                        // this fixture format; production tests cover it.
                        discussion: crate::Discussion::default(),
                    },
                );
            }
        }
        forge
    }
}

/// How `forge.repos` lists a seeded repository: cloned over `https` from
/// its own host; `None` when that makes no clone URL.
fn listed(entry: &Repository) -> Option<vessel_proto::forge::Repository> {
    let url = format!("https://{}/{}", entry.repo.host, entry.repo.path);
    Some(vessel_proto::forge::Repository {
        path: entry.repo.path.clone(),
        description: entry.listing.description.clone(),
        visibility: entry.listing.visibility.unwrap_or(Visibility::Public),
        updated_at: entry.listing.updated_at.unwrap_or(DateTime::UNIX_EPOCH),
        clone_url: url.parse().ok()?,
    })
}

#[cfg(test)]
mod tests;
