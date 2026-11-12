//! A forge in memory (feature `fake`): the test double the node's tests
//! use, so every path of watching, opening, merging and feedback runs
//! with no network (the plan's rule). It keeps pull requests as `Read`s,
//! answers the trait's calls from them, can be told to fail, and counts
//! what it was asked.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::Utc;
use vessel_proto::forge::{
    Checks, ChecksState, ForgeHost, ForgeKind, ForgeLogin, ForgeUrl, MergeMethod, Mergeable,
    PullNumber, PullRequest, PullState, PullSummary, PullTitle, RepoRef, ReviewDecision, Reviews,
};
use vessel_proto::workspace::{BranchName, CommitId};

use crate::provider::{
    Answer, Discussion, Forge, ForgeError, Found, Fresh, Merged, Opened, Opening, Priority, Read,
};

/// How the fake fails, when told to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// As a forge that does not answer.
    Down,
    /// As a forge whose rate limit holds for a minute.
    RateLimited,
    /// As a forge with no token.
    SignedOut,
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
    writers: HashSet<ForgeLogin>,
    calls: HashMap<&'static str, usize>,
    merged: Vec<(PullNumber, MergeMethod, CommitId)>,
    /// What the forge has each branch at (`pushed`).
    heads: HashMap<BranchName, CommitId>,
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
        let mut state = self.state();
        state.version = state.version.saturating_add(1);
        let version = state.version;
        let key = (read.pull.summary.repo.clone(), read.pull.summary.number);
        let _previous = state.pulls.insert(
            key,
            Pull {
                branch,
                read,
                version,
            },
        );
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

    /// The merges asked of it, in order.
    #[must_use]
    pub fn merges(&self) -> Vec<(PullNumber, MergeMethod, CommitId)> {
        self.state().merged.clone()
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
            Some(Failure::Down) => Err(ForgeError::Down),
            Some(Failure::RateLimited) => Err(ForgeError::RateLimited {
                until: Utc::now()
                    .checked_add_signed(chrono::TimeDelta::seconds(60))
                    .unwrap_or_else(Utc::now),
            }),
            Some(Failure::SignedOut) => Err(ForgeError::SignedOut {
                gh: vessel_proto::forge::GhState::SignedOut,
            }),
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

impl Forge for FakeForge {
    fn kind(&self) -> ForgeKind {
        ForgeKind::Github
    }

    fn pins_head(&self, _method: MergeMethod) -> bool {
        true
    }

    fn whoami<'a>(&'a self, _host: &'a ForgeHost) -> Answer<'a, Option<ForgeLogin>> {
        Box::pin(async move {
            let _state = self.enter("whoami")?;
            Ok(ForgeLogin::lossy("vessel-test"))
        })
    }

    fn find<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a BranchName,
        priority: Priority,
    ) -> Answer<'a, Fresh<Vec<Found>>> {
        Box::pin(async move {
            let mut state = self.enter("find")?;
            let mut found: Vec<(u64, Found)> = state
                .pulls
                .iter()
                .filter(|((pull_repo, _), pull)| pull_repo == repo && &pull.branch == branch)
                .map(|(_, pull)| {
                    let summary = &pull.read.pull.summary;
                    (
                        pull.version,
                        Found {
                            number: summary.number,
                            state: summary.state,
                            head: summary.head_commit.clone(),
                            updated_at: summary.updated_at,
                        },
                    )
                })
                .collect();
            let newest = found.iter().map(|(version, _)| *version).max().unwrap_or(0);
            let key = (repo.clone(), branch.clone());
            if priority == Priority::Background
                && state.found.get(&key) == Some(&newest)
                && newest > 0
            {
                return Ok(Fresh::Same);
            }
            let _seen = state.found.insert(key, newest);
            found.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
            Ok(Fresh::Changed(
                found.into_iter().map(|(_, found)| found).collect(),
            ))
        })
    }

    fn changed<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        _head: &'a CommitId,
        priority: Priority,
    ) -> Answer<'a, bool> {
        Box::pin(async move {
            let mut state = self.enter("changed")?;
            let key = (repo.clone(), number);
            let version = state.pulls.get(&key).map_or(0, |pull| pull.version);
            let seen = state.seen.insert(key, version);
            Ok(priority == Priority::Person || seen != Some(version))
        })
    }

    fn read<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        _priority: Priority,
    ) -> Answer<'a, Read> {
        Box::pin(async move {
            let state = self.enter("read")?;
            let mut read = state
                .pulls
                .get(&(repo.clone(), number))
                .map(|pull| pull.read.clone())
                .ok_or(ForgeError::NotFound)?;
            read.pull.read_at = Utc::now();
            Ok(read)
        })
    }

    fn open<'a>(&'a self, repo: &'a RepoRef, opening: &'a Opening) -> Answer<'a, Opened> {
        Box::pin(async move {
            let mut state = self.enter("open")?;
            let open = state.pulls.values().any(|pull| {
                pull.branch == opening.head
                    && pull.read.pull.summary.repo == *repo
                    && pull.read.pull.summary.state == PullState::Open
            });
            if open {
                return Ok(Opened::Exists);
            }
            state.next = state.next.saturating_add(1);
            let number = PullNumber::new(state.next.saturating_add(100))
                .map_err(|_bad| ForgeError::Unexpected { what: "a number" })?;
            let pushed = state.heads.get(&opening.head).cloned();
            drop(state);
            let head: CommitId = match pushed {
                Some(head) => head,
                None => "0000000000000000000000000000000000000001"
                    .parse()
                    .map_err(|_bad| ForgeError::Unexpected { what: "a commit" })?,
            };
            let mut read = pull(repo, number, &opening.head, &head)
                .ok_or(ForgeError::Unexpected { what: "an address" })?;
            read.pull.summary.title = opening.title.clone();
            read.pull.summary.draft = opening.draft;
            read.pull.base = opening.base.clone();
            let number = read.pull.summary.number;
            self.put(opening.head.clone(), read);
            Ok(Opened::Created(number))
        })
    }

    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        method: MergeMethod,
        head: &'a CommitId,
    ) -> Answer<'a, Merged> {
        Box::pin(async move {
            let mut state = self.enter("merge")?;
            state.merged.push((number, method, head.clone()));
            state.version = state.version.saturating_add(1);
            let version = state.version;
            let Some(pull) = state.pulls.get_mut(&(repo.clone(), number)) else {
                return Err(ForgeError::NotFound);
            };
            if pull.read.pull.summary.head_commit != *head {
                return Ok(Merged::HeadMoved);
            }
            pull.read.pull.summary.state = PullState::Merged;
            pull.read.pull.merged_at = Some(Utc::now());
            pull.version = version;
            Ok(Merged::Done(head.clone()))
        })
    }

    fn can_write<'a>(&'a self, _repo: &'a RepoRef, login: &'a ForgeLogin) -> Answer<'a, bool> {
        Box::pin(async move {
            let state = self.enter("can_write")?;
            Ok(state.writers.contains(login))
        })
    }
}
