//! One lock per repository for the git runs that change its list of
//! worktrees: `git worktree add`, and `git worktree remove` with the
//! deletion of the branch the node made for it.
//!
//! [Decision Log]
//! - Purpose: creations at once in one repository failed under load with
//!   "failed to read .git/worktrees/<name>/commondir". git writes a new
//!   worktree's administrative files (`gitdir`, `commondir`, `HEAD`) one
//!   after the other, none atomically, and a run that lists the
//!   repository's worktrees (another `worktree add` looking for where a
//!   branch is checked out, `branch -D` doing the same) reads a
//!   `commondir` that exists but is still empty and stops. The node makes
//!   several worktrees at once (an agent's, a person's, a schedule's
//!   session), so the failure is the product's, not the test's that found
//!   it (`creations_at_once_never_pass_an_agents_limit` in `vessel-node`).
//! - Constraint: other repositories and every other git run stay
//!   concurrent; the node's accounting of worktrees being made (an
//!   agent's limit) is not touched.
//! - Alternatives: retrying that one failure (it depends on git's wording,
//!   hides the race rather than closing it, and still fails a creation
//!   that loses twice); one lock for all repositories (unrelated projects
//!   would wait for each other); git's own locks (each `worktree add`
//!   locks only its own new entry, never the list).
//! - Choice: a lock per repository's common directory, the `.git` folder
//!   all its worktrees share, so the key is the same whichever of its
//!   folders a run names; held only across the git runs themselves. The
//!   map keeps weak handles: a repository nobody is changing costs an
//!   entry until the next lookup sweeps it.
//! - Cost: creations and removals in one repository wait for each other
//!   (each takes a fraction of a second). Other runs that list worktrees
//!   (a switch inside a worktree) are not serialized against an add and
//!   can still meet the moment an add writes its files.
//! - Keep when changing: never hold the lock across anything but git runs
//!   on that repository (no network, no setup command, no other lock), or
//!   one slow project would hold up every creation in it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use tokio::sync::OwnedMutexGuard;

use super::gitfile;

/// The locks of the repositories whose worktrees are being changed.
#[derive(Debug, Default)]
pub(crate) struct RepositoryLocks {
    held: Mutex<HashMap<PathBuf, Weak<tokio::sync::Mutex<()>>>>,
}

impl RepositoryLocks {
    /// Wait for, then hold, the lock of the repository `repo` (any folder
    /// of it: its root, its `.git`, one of its worktrees) until the guard
    /// is dropped.
    pub(crate) async fn hold(&self, repo: &Path) -> OwnedMutexGuard<()> {
        let repo = repo.to_owned();
        let key = tokio::task::spawn_blocking(move || common_dir(&repo))
            .await
            .unwrap_or_else(|_lost| PathBuf::new());
        self.lock_of(key).lock_owned().await
    }

    /// The lock of `key`, made when nobody holds or waits for one.
    fn lock_of(&self, key: PathBuf) -> Arc<tokio::sync::Mutex<()>> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = held.get(&key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        let _none = held.insert(key, Arc::downgrade(&lock));
        lock
    }
}

/// The common directory of the repository `repo` is a folder of
/// (blocking): its `.git` folder; through a `.git` file (a worktree's, a
/// submodule's) the folder it names and the `commondir` that names; the
/// folder itself when it has no `.git` (a bare repository, a `.git`).
/// What cannot be resolved is keyed by the path as given, which only
/// costs serialization with other runs naming it the same way.
fn common_dir(repo: &Path) -> PathBuf {
    let base = canonical(repo.to_owned());
    let dot_git = base.join(".git");
    match std::fs::symlink_metadata(&dot_git) {
        Ok(found) if found.is_dir() => canonical(dot_git),
        Ok(found) if found.is_file() => {
            let Some(named) = gitfile::gitdir_line(&dot_git) else {
                return base;
            };
            let git_dir = canonical(base.join(named));
            match gitfile::first_line(&git_dir.join("commondir")) {
                Some(common) => canonical(git_dir.join(common)),
                None => git_dir,
            }
        }
        Ok(_) | Err(_) => base,
    }
}

fn canonical(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[test]
    fn a_repository_and_its_worktree_share_one_key_and_another_has_its_own() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let other = sandbox.repo("other");
        let worktree = sandbox.path().join("wt");
        sandbox.run(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "side",
                worktree.to_str().unwrap(),
            ],
        );
        let key = common_dir(&repo);
        assert_eq!(key, std::fs::canonicalize(repo.join(".git")).unwrap());
        assert_eq!(common_dir(&worktree), key);
        assert_eq!(common_dir(&repo.join(".git")), key);
        assert_ne!(common_dir(&other), key);
    }

    #[tokio::test]
    async fn a_lock_nobody_holds_is_swept_and_one_held_is_shared() {
        let locks = RepositoryLocks::default();
        let first = locks.lock_of(PathBuf::from("/a"));
        assert!(Arc::ptr_eq(&first, &locks.lock_of(PathBuf::from("/a"))));
        drop(first);
        let _other = locks.lock_of(PathBuf::from("/b"));
        let held = locks.held.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(held.len(), 1, "the unheld lock went");
    }
}
