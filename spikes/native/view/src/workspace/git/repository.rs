//! Guards for actions placed beside another area's displayed changes.
//!
//! [Decision Log]
//! - Purpose: a click is tied to the machine and repository whose files were
//!   drawn, rather than the last repository the Git area happened to read.
//! - Constraint: the code panel owns its shown target; a live session or a
//!   matching path alone does not identify that target.
//! - Alternatives: infer the root from the selected session or a cached Git
//!   target, which can lag the panel during a switch.
//! - Choice: the panel supplies its actual shown repository; a typed expected
//!   repository is accepted only when both machine and root match.
//! - Why: no caller may turn a mismatch into a read or a write elsewhere.
//! - Cost: callers carry both identities and keep the check before mutation.

use vessel_proto::workspace::WorkspaceRoot;

/// A machine's repository identity, borrowed from an accepted panel target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repository<'a> {
    /// The node-book machine key, not a path or display branch name.
    pub machine: &'a str,
    /// A typed project or worktree root.
    pub root: WorkspaceRoot,
}

/// Accept an action only for the repository actually shown by the code panel.
/// A missing shown target never authorizes an operation.
#[must_use]
pub fn matching_repository(
    shown: Option<Repository<'_>>,
    expected: Repository<'_>,
) -> Option<WorkspaceRoot> {
    (shown == Some(expected)).then_some(expected.root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(id: &str) -> WorkspaceRoot {
        WorkspaceRoot::Project {
            project: id.parse().unwrap(),
        }
    }

    #[test]
    fn a_different_shown_machine_or_root_and_an_absent_target_never_authorize_an_action() {
        let first = root("6f1c2a3e-0000-4000-8000-000000000001");
        let second = root("6f1c2a3e-0000-4000-8000-000000000002");
        let expected = Repository {
            machine: "first",
            root: first,
        };
        for shown in [
            None,
            Some(Repository {
                machine: "second",
                root: first,
            }),
            Some(Repository {
                machine: "first",
                root: second,
            }),
            Some(Repository {
                machine: "second",
                root: second,
            }),
        ] {
            assert_eq!(matching_repository(shown, expected), None);
        }
        assert_eq!(matching_repository(Some(expected), expected), Some(first));
    }
}
