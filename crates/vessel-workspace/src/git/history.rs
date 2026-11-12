//! Branches, history and the stash: read with `for-each-ref`, `log` and
//! `stash list`, fields separated by NUL so no name or subject can break a
//! record.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use vessel_proto::workspace::{
    Branch, BranchList, CommitInfo, CommitPage, Distance, RefName, StashEntry, StashList,
    WorktreeBase,
};

use super::{Git, GitError};

/// The most branches listed.
pub(crate) const MAX_BRANCHES: usize = 1000;
/// The most commits a log page holds.
pub(crate) const MAX_LOG_PAGE: u32 = 200;
/// A log page's size when not asked.
pub(crate) const DEFAULT_LOG_PAGE: u32 = 50;
/// The most stash entries listed.
pub(crate) const MAX_STASHES: usize = 100;
/// The longest subject or message kept, in bytes.
const MAX_SUBJECT: usize = 1024;

impl Git {
    /// The local branches, then the remote-tracking ones, by name.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn branches(&self, dir: &Path) -> Result<BranchList, GitError> {
        let default = self.default_branch(dir).await?;
        let output = self
            .command(dir)
            .args([
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00%(committerdate:unix)%00%(upstream:short)%00%(upstream:track,nobracket)%00%(worktreepath)%00%(HEAD)",
                "refs/heads",
                "refs/remotes",
            ])
            .output()
            .await?
            .checked()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();
        let mut more = false;
        for line in text.lines() {
            let Some(branch) = branch_record(line, default.as_ref()) else {
                continue;
            };
            if branches.len() >= MAX_BRANCHES {
                more = true;
                break;
            }
            branches.push(branch);
        }
        Ok(BranchList { branches, more })
    }

    /// A page of history from `from` (`HEAD` when absent): newest first.
    ///
    /// # Errors
    /// [`GitError::Unexpected`] (`"base"`) when `from` does not resolve, and
    /// as the runs'.
    pub async fn log(
        &self,
        dir: &Path,
        from: Option<&WorktreeBase>,
        skip: u32,
        limit: Option<u32>,
    ) -> Result<CommitPage, GitError> {
        let start = match from {
            Some(base) => self
                .resolve(dir, base)
                .await?
                .ok_or(GitError::Unexpected { what: "base" })?,
            None => match self.head(dir).await? {
                Some(head) => head,
                None => {
                    return Ok(CommitPage {
                        commits: Vec::new(),
                        more: false,
                    });
                }
            },
        };
        let limit = limit.unwrap_or(DEFAULT_LOG_PAGE).clamp(1, MAX_LOG_PAGE);
        let output = self
            .command(dir)
            .args([
                "log",
                "--no-color",
                "--format=%H%x00%P%x00%an%x00%at%x00%s%x1e",
            ])
            .arg(format!("--skip={skip}"))
            .arg(format!("--max-count={}", limit.saturating_add(1)))
            .arg(start.as_str())
            .arg("--")
            .output()
            .await?
            .checked()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut commits: Vec<CommitInfo> = text
            .split('\u{1e}')
            .filter_map(|record| commit_record(record.trim_start_matches('\n')))
            .collect();
        let more = commits.len() > usize::try_from(limit).unwrap_or(usize::MAX);
        commits.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(CommitPage { commits, more })
    }

    /// The stash, newest first.
    ///
    /// # Errors
    /// As the run's.
    pub async fn stashes(&self, dir: &Path) -> Result<StashList, GitError> {
        let output = self
            .command(dir)
            .args(["stash", "list", "--format=%H%x00%ct%x00%gs"])
            .output()
            .await?
            .checked()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut more = false;
        for (index, line) in text.lines().enumerate() {
            if entries.len() >= MAX_STASHES {
                more = true;
                break;
            }
            let mut fields = line.split('\0');
            let (Some(commit), Some(time), Some(message)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let (Ok(commit), Ok(index)) = (commit.parse(), u32::try_from(index)) else {
                continue;
            };
            entries.push(StashEntry {
                index,
                commit,
                message: super::patch::cut_text(message, MAX_SUBJECT).0,
                time: unix(time),
            });
        }
        Ok(StashList { entries, more })
    }
}

/// One `for-each-ref` record; `None` for a symbolic `HEAD` of a remote or
/// a name this version cannot hold.
fn branch_record(line: &str, default: Option<&RefName>) -> Option<Branch> {
    let mut fields = line.split('\0');
    let refname = fields.next()?;
    let head = fields.next()?.parse().ok()?;
    let time = unix(fields.next()?);
    let upstream = fields.next()?;
    let track = fields.next()?;
    let worktree = fields.next()?;
    let current = fields.next()? == "*";
    let (name, remote) = if let Some(local) = refname.strip_prefix("refs/heads/") {
        (local, None)
    } else {
        let short = refname.strip_prefix("refs/remotes/")?;
        if short.ends_with("/HEAD") {
            return None;
        }
        let remote = short.split('/').next()?.parse().ok();
        (short, remote)
    };
    let name: RefName = name.parse().ok()?;
    let upstream = upstream
        .parse()
        .ok()
        .map(|upstream| distance(upstream, track));
    Some(Branch {
        default: default == Some(&name),
        name,
        remote,
        current,
        head,
        time,
        upstream,
        worktree: (!worktree.is_empty()).then(|| PathBuf::from(worktree)),
    })
}

/// `ahead 1, behind 2` (or either alone, or nothing) against `upstream`.
fn distance(upstream: RefName, track: &str) -> Distance {
    let count = |word: &str| {
        track
            .split(", ")
            .find_map(|part| part.strip_prefix(word))
            .and_then(|number| number.trim().parse().ok())
            .unwrap_or(0)
    };
    Distance {
        name: upstream,
        ahead: count("ahead "),
        behind: count("behind "),
    }
}

/// One `log` record.
fn commit_record(record: &str) -> Option<CommitInfo> {
    let mut fields = record.split('\0');
    let id = fields.next()?.parse().ok()?;
    let parents = fields
        .next()?
        .split(' ')
        .filter_map(|parent| parent.parse().ok())
        .collect();
    let author = fields.next()?.to_owned();
    let time = unix(fields.next()?);
    let subject = super::patch::cut_text(fields.next()?, MAX_SUBJECT).0;
    Some(CommitInfo {
        id,
        parents,
        author,
        time,
        subject,
    })
}

/// Seconds since the epoch as a time; the epoch when unreadable.
fn unix(text: &str) -> DateTime<Utc> {
    text.trim()
        .parse::<i64>()
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .unwrap_or(DateTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[test]
    fn tracking_reads_both_counts_either_and_none() {
        let up: RefName = "origin/main".parse().unwrap();
        let both = distance(up.clone(), "ahead 3, behind 1");
        assert_eq!((both.ahead, both.behind), (3, 1));
        let ahead = distance(up.clone(), "ahead 2");
        assert_eq!((ahead.ahead, ahead.behind), (2, 0));
        let none = distance(up, "");
        assert_eq!((none.ahead, none.behind), (0, 0));
    }

    #[tokio::test]
    async fn branches_history_and_the_stash_of_a_real_repository() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        sandbox.run(&repo, &["branch", "topic"]);
        for index in 0..3 {
            Sandbox::write(&repo, "README.md", &format!("{index}\n"));
            sandbox.run(&repo, &["commit", "-q", "-am", &format!("change {index}")]);
        }
        let branches = git.branches(&repo).await.unwrap();
        let names: Vec<_> = branches
            .branches
            .iter()
            .map(|branch| (branch.name.as_str(), branch.current, branch.default))
            .collect();
        assert_eq!(names, [("main", true, true), ("topic", false, false)]);
        assert!(branches.branches.first().unwrap().worktree.is_some());

        let page = git.log(&repo, None, 0, Some(2)).await.unwrap();
        assert_eq!(page.commits.len(), 2);
        assert!(page.more);
        assert_eq!(page.commits.first().unwrap().subject, "change 2");
        let rest = git.log(&repo, None, 2, Some(10)).await.unwrap();
        assert_eq!(
            rest.commits
                .iter()
                .map(|commit| commit.subject.as_str())
                .collect::<Vec<_>>(),
            ["change 0", "first"]
        );
        assert!(!rest.more);
        assert!(rest.commits.last().unwrap().parents.is_empty());
        let topic = git
            .log(
                &repo,
                Some(&WorktreeBase::Ref {
                    name: "topic".parse().unwrap(),
                }),
                0,
                None,
            )
            .await
            .unwrap();
        assert_eq!(topic.commits.len(), 1);

        Sandbox::write(&repo, "README.md", "stashed\n");
        sandbox.run(&repo, &["stash", "push", "-q", "-m", "keep this"]);
        let stashes = git.stashes(&repo).await.unwrap();
        let entry = stashes.entries.first().unwrap();
        assert_eq!(entry.index, 0);
        assert!(entry.message.contains("keep this"), "{}", entry.message);
    }
}
