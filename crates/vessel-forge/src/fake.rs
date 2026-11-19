//! A forge in memory (feature `fake`): the test double the node's tests
//! use, so every path of watching, opening, merging and feedback runs
//! with no network (the plan's rule). It keeps pull requests as `Read`s,
//! answers the trait's calls from them, can be told to fail, and counts
//! what it was asked.

mod behaviour;
pub mod seed;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::Utc;
use vessel_proto::forge::{
    Checks, ChecksState, ForgeHost, ForgeKind, ForgeLogin, ForgeUrl, MergeMethod, Mergeable,
    PullNumber, PullRequest, PullState, PullSummary, PullTitle, RefusedReason, RepoLimit, RepoRef,
    Repository, ReviewDecision, Reviews,
};
use vessel_proto::workspace::{BranchName, CommitId};

use crate::provider::{
    Answer, Discussion, Forge, ForgeError, Found, Fresh, Merged, Opened, Opening, Priority, Read,
};

/// How the fake fails, when told to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// As a forge that does not answer.
    Down,
    /// As a forge whose rate limit holds for a minute.
    RateLimited,
    /// As a forge with no token.
    SignedOut,
    /// As a repository the login cannot access.
    NoAccess,
}

#[derive(Debug, Clone)]
struct Pull {
    branch: BranchName,
    read: Read,
    version: u64,
}

#[derive(Debug, Default)]
struct State {
    pulls: HashMap<(RepoRef, PullNumber), Pull>,
    seen: HashMap<(RepoRef, PullNumber), u64>,
    found: HashMap<(RepoRef, BranchName), u64>,
    version: u64,
    next: u64,
    failure: Option<Failure>,
    repo_failures: HashMap<RepoRef, Failure>,
    allowed: Option<HashSet<RepoRef>>,
    limit: Option<NonZeroUsize>,
    refused_merges: HashSet<(RepoRef, PullNumber)>,
    writers: HashSet<ForgeLogin>,
    calls: HashMap<&'static str, usize>,
    merged: VecDeque<(PullNumber, MergeMethod, CommitId)>,
    /// What the forge has each branch at (`pushed`).
    heads: HashMap<BranchName, CommitId>,
    /// What `forge.repos` lists (ADR 0088), newest first when asked.
    repositories: Vec<Repository>,
}

impl State {
    // Admission and insertion share a lock to enforce capacity under concurrent opens.
    fn put(&mut self, branch: BranchName, read: Read) {
        self.version = self.version.saturating_add(1);
        let version = self.version;
        // Seeded numbers reserve the allocator too: opening another branch
        // must never overwrite a file's existing pull request.
        self.next = self.next.max(read.pull.summary.number.get());
        let key = (read.pull.summary.repo.clone(), read.pull.summary.number);
        let _previous = self.pulls.insert(
            key,
            Pull {
                branch,
                read,
                version,
            },
        );
    }
}

/// A forge in memory.
#[derive(Debug, Default)]
pub struct FakeForge {
    state: Mutex<State>,
}

impl FakeForge {
    /// An empty one.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Put `read` (a new pull request, or a change to one) for `branch`.
    pub fn put(&self, branch: BranchName, read: Read) {
        self.state().put(branch, read);
    }

    /// The forge has `branch` at `head`, as a push leaves it: a pull
    /// request opened of it has that head (otherwise a made-up one).
    pub fn pushed(&self, branch: BranchName, head: CommitId) {
        let _previous = self.state().heads.insert(branch, head);
    }

    /// Fail every call from now on, or stop failing.
    pub fn fail(&self, failure: Option<Failure>) {
        self.state().failure = failure;
    }

    /// Fail calls for this repository, or clear its explicit failure.
    /// A global failure still takes precedence.
    pub fn fail_repository(&self, repo: RepoRef, failure: Option<Failure>) {
        let mut state = self.state();
        match failure {
            Some(failure) => {
                let _old = state.repo_failures.insert(repo, failure);
            }
            None => {
                let _old = state.repo_failures.remove(&repo);
            }
        }
    }

    /// List `repository` in the account's repositories and in searches
    /// (ADR 0088); the same path again replaces it.
    pub fn list_repository(&self, repository: Repository) {
        let mut state = self.state();
        state
            .repositories
            .retain(|known| known.path != repository.path);
        state.repositories.push(repository);
    }

    /// Refuse a correctly pinned merge of this pull request, without
    /// changing its facts; used by a file fixture's explicit failure.
    pub fn refuse_merge(&self, repo: RepoRef, number: PullNumber) {
        let _new = self.state().refused_merges.insert((repo, number));
    }

    fn fixture(repositories: HashSet<RepoRef>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                allowed: Some(repositories),
                limit: NonZeroUsize::new(seed::MAX_PULLS),
                ..State::default()
            }),
        })
    }

    /// `login` may write to every repository.
    pub fn writer(&self, login: ForgeLogin) {
        let _new = self.state().writers.insert(login);
    }

    /// How many times `method` (`find`, `changed`, `read`, `open`,
    /// `merge`, `can_write`, `whoami`) was called.
    #[must_use]
    pub fn calls(&self, method: &str) -> usize {
        self.state().calls.get(method).copied().unwrap_or(0)
    }

    /// The merges asked of it, in order. File fixtures retain only their
    /// latest `MAX_PULLS` attempts, dropping the oldest when full.
    #[must_use]
    pub fn merges(&self) -> Vec<(PullNumber, MergeMethod, CommitId)> {
        self.state().merged.iter().cloned().collect()
    }

    /// The pull request `number` of `repo` as it holds it now.
    #[must_use]
    pub fn get(&self, repo: &RepoRef, number: PullNumber) -> Option<Read> {
        self.state()
            .pulls
            .get(&(repo.clone(), number))
            .map(|pull| pull.read.clone())
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn enter_repo(
        &self,
        method: &'static str,
        repo: &RepoRef,
    ) -> Result<MutexGuard<'_, State>, ForgeError> {
        let state = self.enter(method)?;
        if state
            .allowed
            .as_ref()
            .is_some_and(|known| !known.contains(repo))
        {
            return Err(ForgeError::NotFound);
        }
        match state.repo_failures.get(repo) {
            Some(failure) => Err(failure.error()),
            None => Ok(state),
        }
    }

    fn enter(&self, method: &'static str) -> Result<MutexGuard<'_, State>, ForgeError> {
        let mut state = self.state();
        *state.calls.entry(method).or_insert(0) = state
            .calls
            .get(method)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        match state.failure {
            None => Ok(state),
            Some(failure) => Err(failure.error()),
        }
    }
}

impl Failure {
    fn error(self) -> ForgeError {
        match self {
            Self::Down => ForgeError::Down,
            Self::RateLimited => ForgeError::RateLimited {
                until: Utc::now()
                    .checked_add_signed(chrono::TimeDelta::seconds(60))
                    .unwrap_or_else(Utc::now),
            },
            Self::SignedOut => ForgeError::SignedOut {
                gh: vessel_proto::forge::GhState::SignedOut,
            },
            Self::NoAccess => ForgeError::Refused {
                reason: RefusedReason::NoAccess,
            },
        }
    }
}

/// An open pull request of `repo` numbered `number`, head `branch` at
/// `head`, base `main`, with nothing known of its checks and reviews: what
/// a test starts from and changes. `None` only for a host or path no
/// address can hold (the types make that impossible in practice).
#[must_use]
pub fn pull(
    repo: &RepoRef,
    number: PullNumber,
    branch: &BranchName,
    head: &CommitId,
) -> Option<Read> {
    let url = ForgeUrl::lossy(&format!(
        "https://{}/{}/pull/{number}",
        repo.host, repo.path
    ))?;
    let now = Utc::now();
    Some(Read {
        pull: PullRequest {
            summary: PullSummary {
                repo: repo.clone(),
                number,
                url,
                title: PullTitle::lossy("A change"),
                state: PullState::Open,
                draft: false,
                head_commit: head.clone(),
                checks: ChecksState::None,
                review: ReviewDecision::None,
                mergeable: Mergeable::Unknown,
                updated_at: now,
            },
            author: None,
            head_branch: branch.clone(),
            base: "main".parse().ok()?,
            checks: Checks::unknown(),
            reviews: Reviews::default(),
            created_at: now,
            merged_at: None,
            closed_at: None,
            merge_methods: vec![MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase],
            read_at: now,
        },
        discussion: Discussion::default(),
    })
}
