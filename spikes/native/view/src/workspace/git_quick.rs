//! The git button's quick action and why each action is off (GOAT's
//! `GitActionsControl.logic.ts`, the page's `data/workspace/git-quick.ts`):
//! one button names the action that makes sense now, its menu holds the
//! rest, and an action that cannot run says why instead of failing after a
//! round trip.

use vessel_proto::workspace::GitStatus;

/// The actions of the git menu that depend on the repository's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitAction {
    /// Commit…
    Commit,
    /// Push.
    Push,
    /// Pull.
    Pull,
    /// Fetch.
    Fetch,
}

/// Why an action is off (GOAT's words where it has them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// Nothing changed.
    Clean,
    /// No branch is checked out; pushing.
    DetachedPush,
    /// No branch is checked out; pulling.
    DetachedPull,
    /// Ahead and behind.
    Diverged,
    /// Behind only.
    Behind,
    /// Nothing ahead.
    NothingToPush,
    /// No upstream to pull from.
    NoUpstream,
}

/// What the quick button does now: commit when there are changes, else
/// push when ahead, pull when behind, else fetch.
#[must_use]
pub fn quick_action(status: &GitStatus) -> GitAction {
    if !status.files.is_empty() {
        return GitAction::Commit;
    }
    match &status.upstream {
        None if status.branch.is_none() => GitAction::Fetch,
        None => GitAction::Push,
        Some(upstream) if upstream.ahead > 0 => GitAction::Push,
        Some(upstream) if upstream.behind > 0 => GitAction::Pull,
        Some(_) => GitAction::Fetch,
    }
}

/// Why `action` cannot run on `status` (`None`: it can).
#[must_use]
pub fn action_blocked(action: GitAction, status: &GitStatus) -> Option<Blocked> {
    let detached = status.branch.is_none();
    let upstream = status.upstream.as_ref();
    match action {
        GitAction::Commit => status.files.is_empty().then_some(Blocked::Clean),
        GitAction::Push => {
            if detached {
                return Some(Blocked::DetachedPush);
            }
            let upstream = upstream?;
            if upstream.ahead > 0 && upstream.behind > 0 {
                Some(Blocked::Diverged)
            } else if upstream.behind > 0 {
                Some(Blocked::Behind)
            } else {
                (upstream.ahead == 0).then_some(Blocked::NothingToPush)
            }
        }
        GitAction::Pull => {
            if detached {
                Some(Blocked::DetachedPull)
            } else {
                upstream.is_none().then_some(Blocked::NoUpstream)
            }
        }
        GitAction::Fetch => None,
    }
}

#[cfg(test)]
mod tests {
    //! The page's `panel-model.test.ts`, "the git button".

    use vessel_proto::workspace::{Distance, FileState, StatusEntry};

    use super::*;

    fn status() -> GitStatus {
        GitStatus {
            branch: Some("feature".parse().unwrap()),
            head: None,
            upstream: Some(distance(0, 0)),
            base: None,
            operation: None,
            files: Vec::new(),
            more: false,
            origin: None,
        }
    }

    fn distance(ahead: u32, behind: u32) -> Distance {
        Distance {
            name: "origin/feature".parse().unwrap(),
            ahead,
            behind,
        }
    }

    #[test]
    fn commits_when_there_are_changes_else_pushes_pulls_or_fetches() {
        let changed = GitStatus {
            files: vec![StatusEntry {
                path: "a".parse().unwrap(),
                old_path: None,
                index: FileState::Modified,
                worktree: FileState::Unmodified,
            }],
            ..status()
        };
        assert_eq!(quick_action(&changed), GitAction::Commit);
        let ahead = GitStatus {
            upstream: Some(distance(2, 0)),
            ..status()
        };
        assert_eq!(quick_action(&ahead), GitAction::Push);
        let behind = GitStatus {
            upstream: Some(distance(0, 1)),
            ..status()
        };
        assert_eq!(quick_action(&behind), GitAction::Pull);
        assert_eq!(quick_action(&status()), GitAction::Fetch);
        let alone = GitStatus {
            upstream: None,
            ..status()
        };
        assert_eq!(quick_action(&alone), GitAction::Push);
    }

    #[test]
    fn says_why_an_action_is_off() {
        assert_eq!(
            action_blocked(GitAction::Commit, &status()),
            Some(Blocked::Clean)
        );
        let detached = GitStatus {
            branch: None,
            ..status()
        };
        assert_eq!(
            action_blocked(GitAction::Push, &detached),
            Some(Blocked::DetachedPush)
        );
        let diverged = GitStatus {
            upstream: Some(distance(1, 1)),
            ..status()
        };
        assert_eq!(
            action_blocked(GitAction::Push, &diverged),
            Some(Blocked::Diverged)
        );
        assert_eq!(
            action_blocked(GitAction::Push, &status()),
            Some(Blocked::NothingToPush)
        );
        assert_eq!(action_blocked(GitAction::Fetch, &status()), None);
    }
}
