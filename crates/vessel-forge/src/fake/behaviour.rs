//! The same in-memory Forge operations for tests and seeded fixture nodes.
//! Conditional state retains only real pull/branch keys; explicit refusals
//! never mutate facts, and file fixtures bound open/merge history.
use super::{
    Answer, BranchName, CheckReport, CommitId, FakeForge, Forge, ForgeError, ForgeHost, ForgeKind,
    ForgeLogin, Found, Fresh, MergeMethod, Merged, Opened, Opening, Priority, PullNumber,
    PullState, Read, RepoLimit, RepoRef, Repository, RunId, Timeline, Utc, pull,
};

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
            let mut state = self.enter_repo("find", repo)?;
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
            // Empty branch probes must not accumulate arbitrary cache keys.
            if newest > 0 {
                let _seen = state.found.insert(key, newest);
            }
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
            let mut state = self.enter_repo("changed", repo)?;
            let key = (repo.clone(), number);
            let version = state.pulls.get(&key).map_or(0, |pull| pull.version);
            if version == 0 {
                return Ok(true);
            }
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
            let state = self.enter_repo("read", repo)?;
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
            let mut state = self.enter_repo("open", repo)?;
            let open = state.pulls.values().any(|pull| {
                pull.branch == opening.head
                    && pull.read.pull.summary.repo == *repo
                    && pull.read.pull.summary.state == PullState::Open
            });
            if open {
                return Ok(Opened::Exists);
            }
            if state
                .limit
                .is_some_and(|limit| state.pulls.len() >= limit.get())
            {
                return Ok(Opened::Rejected(
                    "the pull request limit has been reached".to_owned(),
                ));
            }
            let next = state
                .next
                .max(100)
                .checked_add(1)
                .ok_or(ForgeError::Unexpected {
                    what: "a new pull request number",
                })?;
            let number = PullNumber::new(next).map_err(|_bad| ForgeError::Unexpected {
                what: "a new pull request number",
            })?;
            state.next = next;
            let pushed = state.heads.get(&opening.head).cloned();
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
            state.put(opening.head.clone(), read);
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
            let mut state = self.enter_repo("merge", repo)?;
            if state
                .limit
                .is_some_and(|limit| state.merged.len() >= limit.get())
            {
                let _oldest = state.merged.pop_front();
            }
            state.merged.push_back((number, method, head.clone()));
            let key = (repo.clone(), number);
            let Some(pull) = state.pulls.get(&key) else {
                return Err(ForgeError::NotFound);
            };
            if pull.read.pull.summary.head_commit != *head {
                return Ok(Merged::HeadMoved);
            }
            if state.refused_merges.contains(&key) {
                return Ok(Merged::Rejected(
                    "the repository refused this merge".to_owned(),
                ));
            }
            state.version = state.version.saturating_add(1);
            let version = state.version;
            let Some(pull) = state.pulls.get_mut(&key) else {
                return Err(ForgeError::NotFound);
            };
            pull.read.pull.summary.state = PullState::Merged;
            pull.read.pull.merged_at = Some(Utc::now());
            pull.version = version;
            Ok(Merged::Done(head.clone()))
        })
    }

    fn can_write<'a>(&'a self, repo: &'a RepoRef, login: &'a ForgeLogin) -> Answer<'a, bool> {
        Box::pin(async move {
            let state = self.enter_repo("can_write", repo)?;
            Ok(state.writers.contains(login))
        })
    }

    /// The seeded activity, empty when none was put.
    fn timeline<'a>(&'a self, repo: &'a RepoRef, number: PullNumber) -> Answer<'a, Timeline> {
        Box::pin(async move {
            let state = self.enter_repo("timeline", repo)?;
            Ok(state
                .timelines
                .get(&(repo.clone(), number))
                .cloned()
                .unwrap_or_default())
        })
    }

    /// The seeded check run; `NotFound` when none was put.
    fn check<'a>(
        &'a self,
        repo: &'a RepoRef,
        run: RunId,
        _workflow_run: Option<RunId>,
    ) -> Answer<'a, CheckReport> {
        Box::pin(async move {
            let state = self.enter_repo("check", repo)?;
            state
                .reports
                .get(&(repo.clone(), run))
                .cloned()
                .ok_or(ForgeError::NotFound)
        })
    }

    /// Every listed repository for an empty query, else those whose path
    /// or description holds it (any case); newest first, at most `limit`.
    fn repositories<'a>(
        &'a self,
        _host: &'a ForgeHost,
        query: &'a str,
        limit: RepoLimit,
    ) -> Answer<'a, Vec<Repository>> {
        Box::pin(async move {
            let state = self.enter("repositories")?;
            let wanted = query.trim().to_lowercase();
            let mut found: Vec<Repository> = state
                .repositories
                .iter()
                .filter(|repository| {
                    wanted.is_empty()
                        || repository.path.as_str().to_lowercase().contains(&wanted)
                        || repository.description.as_ref().is_some_and(|description| {
                            description.as_str().to_lowercase().contains(&wanted)
                        })
                })
                .cloned()
                .collect();
            found.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            found.truncate(usize::from(limit.get()));
            Ok(found)
        })
    }
}
