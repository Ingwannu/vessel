//! Which commits a served diff stat compares, and the line total.

use std::path::Path;

use vessel_proto::registry::DiffStat;
use vessel_proto::workspace::{BranchName, DiffScope, FileChange, RefName, WorktreeBase};
use vessel_workspace::Git;

/// What a checkout is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Compared {
    /// The merge base with the workspace base.
    Base,
    /// `origin/<branch>`, and only when that ref exists.
    Origin,
}

/// The count for `dir`.
///
/// None when git cannot answer or there is no comparison.
#[must_use]
pub(super) async fn stat_in(git: &Git, dir: &Path, base: Option<&RefName>) -> Option<DiffStat> {
    let status = match git.status(dir).await {
        Ok(status) => status,
        Err(error) => {
            tracing::debug!(%error, "diff stat: git status failed");
            return None;
        }
    };
    let branch = status.branch.as_ref().map(BranchName::as_str);
    let base_name = base.map(RefName::as_str);
    let scope = match compared(branch, base_name)? {
        Compared::Base => DiffScope::Base { base: None },
        Compared::Origin => origin_scope(git, dir, branch?).await?,
    };
    let fallback = base.cloned().map(|name| WorktreeBase::Ref { name });
    let resolved = match git.resolve_scope(dir, &scope, fallback.as_ref()).await {
        Ok(resolved) => resolved,
        Err(error) => {
            tracing::debug!(%error, "diff stat: the comparison did not resolve");
            return None;
        }
    };
    match git.changes(dir, &resolved, false).await {
        Ok(summary) => counted(&summary.files),
        Err(error) => {
            tracing::debug!(%error, "diff stat: git diff failed");
            None
        }
    }
}

async fn origin_scope(git: &Git, dir: &Path, branch: &str) -> Option<DiffScope> {
    let name = origin_name(branch)?;
    match git
        .resolve(dir, &WorktreeBase::Ref { name: name.clone() })
        .await
    {
        Ok(Some(_commit)) => Some(DiffScope::Base {
            base: Some(WorktreeBase::Ref { name }),
        }),
        Ok(None) => None,
        Err(error) => {
            tracing::debug!(%error, "diff stat: the upstream ref did not resolve");
            None
        }
    }
}

fn origin_name(branch: &str) -> Option<RefName> {
    match format!("origin/{}", bare(branch)).parse() {
        Ok(name) => Some(name),
        Err(error) => {
            tracing::debug!(%error, "diff stat: the branch is not a comparison ref");
            None
        }
    }
}

/// No branch is none.
///
/// Any other branch than the base uses the base. The base itself, and a
/// missing base, use `origin/<branch>`.
#[must_use]
pub(super) const fn compared(branch: Option<&str>, base: Option<&str>) -> Option<Compared> {
    match (branch, base) {
        (None, None | Some(_)) => None,
        (Some(branch), Some(base)) if !on_base(branch, base) => Some(Compared::Base),
        (Some(_), None | Some(_)) => Some(Compared::Origin),
    }
}

const fn on_base(branch: &str, base: &str) -> bool {
    bare(branch) == bare(base)
}

/// `refs/heads/main` matches `main`. `refs/remotes/origin/main` does not.
const fn bare(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix("refs/heads/") {
        return rest;
    }
    if let Some(rest) = name.strip_prefix("refs/remotes/") {
        return rest;
    }
    name
}

/// The numstat sum.
///
/// A binary file contributes nothing. Both sides zero is none.
#[must_use]
pub(super) fn counted(files: &[FileChange]) -> Option<DiffStat> {
    let mut additions = 0;
    let mut deletions = 0;
    for file in files {
        additions = additions.saturating_add(file.additions.unwrap_or(0));
        deletions = deletions.saturating_add(file.deletions.unwrap_or(0));
    }
    (additions > 0 || deletions > 0).then_some(DiffStat {
        additions,
        deletions,
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::process::Command;

    use vessel_proto::registry::DiffStat;
    use vessel_proto::workspace::{ChangeStatus, FileChange, RefName};
    use vessel_workspace::{Git, GitSetup};

    use super::{Compared, compared, counted};

    fn change(additions: Option<u32>, deletions: Option<u32>) -> FileChange {
        FileChange {
            path: "a".parse().unwrap(),
            old_path: None,
            status: ChangeStatus::Modified,
            additions,
            deletions,
            binary: additions.is_none(),
        }
    }

    #[test]
    fn zero_and_a_binary_file_are_hidden() {
        assert!(counted(&[]).is_none());
        assert!(counted(&[change(Some(0), Some(0))]).is_none());
        assert!(counted(&[change(None, None)]).is_none());
        assert_eq!(
            counted(&[change(Some(2), Some(1))]),
            Some(DiffStat {
                additions: 2,
                deletions: 1
            })
        );
        assert_eq!(
            counted(&[
                change(Some(u32::MAX), Some(1)),
                change(Some(1), Some(u32::MAX))
            ]),
            Some(DiffStat {
                additions: u32::MAX,
                deletions: u32::MAX
            })
        );
    }

    #[test]
    fn comparison_follows_the_branch_and_detached_is_none() {
        assert_eq!(compared(None, Some("main")), None);
        assert_eq!(compared(Some("main"), Some("main")), Some(Compared::Origin));
        assert_eq!(
            compared(Some("refs/heads/main"), Some("main")),
            Some(Compared::Origin)
        );
        assert_eq!(
            compared(Some("main"), Some("refs/heads/main")),
            Some(Compared::Origin)
        );
        assert_eq!(
            compared(Some("refs/remotes/origin/main"), Some("main")),
            Some(Compared::Base)
        );
        assert_eq!(
            compared(Some("feature"), Some("main")),
            Some(Compared::Base)
        );
        assert_eq!(compared(Some("feature"), None), Some(Compared::Origin));
    }

    fn run(env: &[(String, OsString)], dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .envs(env.iter().map(|(key, value)| (key.as_str(), value)))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn a_feature_branch_counts_lines_and_the_base_without_origin_is_null() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("gitconfig");
        std::fs::write(
            &config,
            "[user]\n\tname = Test\n\temail = test@example.invalid\n\
             [init]\n\tdefaultBranch = main\n\
             [commit]\n\tgpgsign = false\n\
             [core]\n\tautocrlf = false\n",
        )
        .unwrap();
        std::fs::create_dir(temp.path().join("no-hooks")).unwrap();
        let env = vec![
            ("GIT_CONFIG_GLOBAL".to_owned(), config.into_os_string()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), OsString::from("1")),
        ];
        let found = Git::probe(Some(GitSetup {
            program: which::which("git").unwrap(),
            no_hooks: temp.path().join("no-hooks"),
            env: env.clone(),
        }))
        .await;
        let git = found.git.unwrap();
        let dir = temp.path().join("repo");
        std::fs::create_dir(&dir).unwrap();
        run(&env, &dir, &["init", "-q"]);
        std::fs::write(dir.join("README.md"), "hello\n").unwrap();
        run(&env, &dir, &["add", "README.md"]);
        run(&env, &dir, &["commit", "-q", "-m", "first"]);
        let base: RefName = "main".parse().unwrap();
        assert!(super::stat_in(&git, &dir, Some(&base)).await.is_none());
        std::fs::write(dir.join("README.md"), "hello\nworld\n").unwrap();
        assert!(super::stat_in(&git, &dir, Some(&base)).await.is_none());
        run(&env, &dir, &["checkout", "-q", "-b", "feature"]);
        assert_eq!(
            super::stat_in(&git, &dir, Some(&base)).await,
            Some(DiffStat {
                additions: 1,
                deletions: 0
            })
        );
        run(&env, &dir, &["checkout", "-q", "--detach"]);
        assert!(super::stat_in(&git, &dir, Some(&base)).await.is_none());
    }
}
