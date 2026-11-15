//! The page's commit dialog marks the project's default branch, including
//! its remote-tracking spelling. This is guidance, never a write gate.

use vessel_proto::workspace::{BranchList, GitStatus, RefName};

/// Where the commit would land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDestination {
    /// The project's default branch.
    DefaultBranch,
    /// Another checked-out branch, or an unknown project default.
    Branch,
    /// The checked-out commit without a branch.
    Detached,
}

impl CommitDestination {
    /// The page's git-dialogs default-branch comparison. An unavailable
    /// project default does not invent a warning for an arbitrary branch.
    #[must_use]
    pub fn of(status: &GitStatus, default: Option<&RefName>) -> Self {
        let Some(branch) = &status.branch else {
            return Self::Detached;
        };
        match default {
            Some(default) if names_default(default, branch.as_str()) => Self::DefaultBranch,
            Some(_) | None => Self::Branch,
        }
    }

    /// Only a default-branch commit needs the reference's warning.
    #[must_use]
    pub const fn alert_variant(self) -> Option<crate::alert::Variant> {
        match self {
            Self::DefaultBranch => Some(crate::alert::Variant::Warning),
            Self::Branch | Self::Detached => None,
        }
    }
}

/// The project's default branch as the node marks it in `list`. With
/// `origin/HEAD` set the node marks the remote-tracking branch
/// (`origin/main`), not the local one: every reader of the mark takes
/// either spelling ([`names_default`]).
#[must_use]
pub fn project_default(list: &BranchList) -> Option<&RefName> {
    list.branches
        .iter()
        .find(|branch| branch.default)
        .map(|branch| &branch.name)
}

/// Whether the local branch `local` is the project's `default`, in either
/// of its spellings (`main`, or `origin/main` when the remote's HEAD
/// names it).
#[must_use]
pub fn names_default(default: &RefName, local: &str) -> bool {
    default.as_str() == local
        || default
            .as_str()
            .strip_suffix(local)
            .and_then(|prefix| prefix.strip_suffix('/'))
            .is_some_and(|remote| !remote.is_empty() && !remote.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git::fixtures;

    #[test]
    fn default_branch_guidance_keeps_the_pages_local_remote_and_absent_cases() {
        let mut status = fixtures::status(vec![]);
        for name in ["feature", "origin/feature"] {
            let name = name.parse().unwrap();
            assert_eq!(
                CommitDestination::of(&status, Some(&name)),
                CommitDestination::DefaultBranch
            );
            assert_eq!(
                CommitDestination::of(&status, Some(&name)).alert_variant(),
                Some(crate::alert::Variant::Warning)
            );
        }
        assert_eq!(
            CommitDestination::of(&status, None),
            CommitDestination::Branch
        );
        assert_eq!(
            CommitDestination::of(&status, Some(&"origin/other".parse().unwrap())),
            CommitDestination::Branch
        );
        assert_eq!(CommitDestination::Branch.alert_variant(), None);
        status.branch = None;
        assert_eq!(
            CommitDestination::of(&status, Some(&"origin/feature".parse().unwrap())),
            CommitDestination::Detached
        );
        assert_eq!(CommitDestination::Detached.alert_variant(), None);
    }

    #[test]
    fn a_remote_tracking_default_names_its_local_branch_and_no_other() {
        let default: RefName = "origin/feature/x".parse().unwrap();
        assert!(names_default(&default, "feature/x"));
        assert!(!names_default(&default, "x"));
        assert!(names_default(&"main".parse().unwrap(), "main"));
        assert!(!names_default(&"origin/main".parse().unwrap(), "domain"));
    }

    #[test]
    fn the_node_s_mark_is_found_on_the_remote_tracking_branch_too() {
        use chrono::{TimeZone as _, Utc};
        use vessel_proto::workspace::Branch;

        let branch = |name: &str, remote: bool, default: bool| Branch {
            name: name.parse().unwrap(),
            remote: remote.then(|| "origin".parse().unwrap()),
            current: !remote,
            default,
            head: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
            time: Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap(),
            upstream: None,
            worktree: None,
        };
        // `origin/HEAD` set: the node marks `origin/main` only.
        let list = BranchList {
            branches: vec![
                branch("main", false, false),
                branch("origin/main", true, true),
            ],
            more: false,
        };
        let default = project_default(&list).unwrap();
        assert_eq!(default.as_str(), "origin/main");
        let status = fixtures::status(vec![]);
        let mut on_main = status.clone();
        on_main.branch = Some("main".parse().unwrap());
        assert_eq!(
            CommitDestination::of(&on_main, Some(default)),
            CommitDestination::DefaultBranch
        );
        assert_eq!(
            CommitDestination::of(&status, Some(default)),
            CommitDestination::Branch
        );
    }
}
