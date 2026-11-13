//! The page's commit dialog marks the project's default branch, including
//! its remote-tracking spelling. This is guidance, never a write gate.

use vessel_proto::workspace::{GitStatus, RefName};

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
            Some(default)
                if default.as_str() == branch.as_str()
                    || default.as_str().ends_with(&format!("/{branch}")) =>
            {
                Self::DefaultBranch
            }
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
}
