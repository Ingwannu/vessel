//! Paseo's branch-switcher choices, independent of the combobox drawing.
//!
//! [Decision Log]
//! - Purpose: a branch picker is a checked name list, not a metadata dialog.
//! - Constraint: a previous answer may stay visible during a refresh but must
//!   not authorize checkout, and choosing the current branch is a no-op.
//! - Alternatives: disable the checked row (unlike Paseo), or leave the
//!   current/refresh decisions in event callbacks.
//! - Choice: typed read presentation and selection; complete node entries
//!   stay with every choice, including remote refs and worktree facts.
//! - Why: `components/branch-switcher.tsx` and `hooks/use-branch-switcher.ts`
//!   show names with a selected check and close without a write for current.
//! - Cost: names are cloned for the bounded list; metadata is retained without
//!   adding lines or badges the reference does not draw.

use vessel_proto::workspace::{Branch, BranchList, Switch, WorkspaceRoot};

use super::{Reading, SwitchProblem, Writing, filter_branches, switch_request};
use crate::workspace::problem::Problem;

/// One checked-name choice, retaining the node's complete branch record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchChoice {
    /// The full branch, used only after a keyed read was accepted.
    pub branch: Branch,
    /// The shown read is current and no write owns the repository.
    pub enabled: bool,
}

/// The list's read state; the renderer chooses no policy of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchChoices<'a> {
    /// No branch facts have arrived yet.
    Waiting,
    /// The read failed; the caller shows its localized alert beside the action.
    Failed(&'a Problem),
    /// Filtered names and the node's own truncation flag.
    Rows {
        /// Every matching entry in the bounded node reply.
        rows: Vec<BranchChoice>,
        /// More branches exist than the node returned.
        more: bool,
    },
}

/// Choose names and availability from the accepted read and current write.
#[must_use]
pub fn branch_choices<'a>(
    read: &'a Reading<BranchList>,
    writing: &Writing,
    filter: &str,
) -> BranchChoices<'a> {
    match read {
        Reading::Empty | Reading::Loading { previous: None, .. } => BranchChoices::Waiting,
        Reading::Failed(problem) => BranchChoices::Failed(problem),
        Reading::Ready(list)
        | Reading::Loading {
            previous: Some(list),
            ..
        } => {
            let enabled = read.ready().is_some() && !writing.busy();
            BranchChoices::Rows {
                rows: filter_branches(&list.branches, filter)
                    .into_iter()
                    .map(|branch| BranchChoice { branch, enabled })
                    .collect(),
                more: list.more,
            }
        }
    }
}

/// Whether the branch picker still owns the place for its read's error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerVisibility {
    /// Its choices are still on screen.
    Open,
    /// The person closed it or moved on to another form.
    Closed,
}

/// Cache a late failure without replacing another form's notice.
#[must_use]
pub const fn branch_failure(
    read: &Reading<BranchList>,
    visibility: PickerVisibility,
) -> Option<&Problem> {
    match (read, visibility) {
        (Reading::Failed(problem), PickerVisibility::Open) => Some(problem),
        (Reading::Empty | Reading::Loading { .. } | Reading::Ready(_), _)
        | (Reading::Failed(_), PickerVisibility::Closed) => None,
    }
}

/// A selected row either closes without a write or carries a prepared checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchSelection {
    /// Paseo's `branchId === currentBranchName` early return.
    Current,
    /// A local checkout or a remote-to-local creation.
    Switch(Switch),
}

/// Prepare the selected branch, without asking the node to switch to itself.
///
/// # Errors
/// A remote ref or local name did not satisfy the wire's branch contract.
pub fn branch_selection(
    root: WorkspaceRoot,
    branch: &Branch,
) -> Result<BranchSelection, SwitchProblem> {
    if branch.current {
        Ok(BranchSelection::Current)
    } else {
        switch_request(root, branch).map(BranchSelection::Switch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_late_failure_from_a_closed_picker_cannot_replace_the_next_forms_notice() {
        let failed = Reading::Failed(Problem::Older);
        assert_eq!(branch_failure(&failed, PickerVisibility::Closed), None);
        assert_eq!(
            branch_failure(&failed, PickerVisibility::Open),
            Some(&Problem::Older)
        );
        assert_eq!(
            branch_failure(&Reading::Empty, PickerVisibility::Open),
            None
        );
    }

    fn branch(name: &str, current: bool) -> Branch {
        Branch {
            name: name.parse().unwrap(),
            current,
            default: false,
            remote: None,
            head: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
            time: "2026-01-01T00:00:00Z".parse().unwrap(),
            upstream: None,
            worktree: None,
        }
    }

    fn root() -> WorkspaceRoot {
        WorkspaceRoot::Project {
            project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
        }
    }

    #[test]
    fn the_current_choice_keeps_its_check_without_becoming_disabled_or_writing() {
        let current = branch("feature", true);
        let read = Reading::Ready(BranchList {
            branches: vec![current.clone()],
            more: false,
        });
        assert_eq!(
            branch_choices(&read, &Writing::default(), ""),
            BranchChoices::Rows {
                rows: vec![BranchChoice {
                    branch: current.clone(),
                    enabled: true
                }],
                more: false,
            }
        );
        assert_eq!(
            branch_selection(root(), &current),
            Ok(BranchSelection::Current)
        );
    }

    #[test]
    fn a_remote_choice_keeps_all_facts_and_prepares_the_local_checkout() {
        let mut remote = branch("origin/feature/한글", false);
        remote.remote = Some("origin".parse().unwrap());
        let list = BranchList {
            branches: vec![remote.clone()],
            more: true,
        };
        assert_eq!(
            branch_choices(&Reading::Ready(list), &Writing::default(), " FEATURE/한글 "),
            BranchChoices::Rows {
                rows: vec![BranchChoice {
                    branch: remote.clone(),
                    enabled: true
                }],
                more: true,
            }
        );
        assert_eq!(
            branch_selection(root(), &remote),
            Ok(BranchSelection::Switch(Switch {
                root: root(),
                branch: "feature/한글".parse().unwrap(),
                create: Some(vessel_proto::workspace::WorktreeBase::Ref {
                    name: "origin/feature/한글".parse().unwrap(),
                }),
            }))
        );
    }

    #[test]
    fn refreshing_and_writing_leave_previous_choices_visible_without_authorizing_them() {
        let entry = branch("feature", false);
        let mut read = Reading::Ready(BranchList {
            branches: vec![entry.clone()],
            more: false,
        });
        let key = super::super::RequestKeys::default().issue().unwrap();
        read.start(key);
        let expected = BranchChoices::Rows {
            rows: vec![BranchChoice {
                branch: entry.clone(),
                enabled: false,
            }],
            more: false,
        };
        assert_eq!(branch_choices(&read, &Writing::default(), ""), expected);
        let mut writing = Writing::default();
        assert!(writing.begin(key));
        assert_eq!(
            branch_choices(
                &Reading::Ready(BranchList {
                    branches: vec![entry],
                    more: false
                }),
                &writing,
                ""
            ),
            expected
        );
    }

    #[test]
    fn waiting_failure_and_an_empty_filter_result_have_different_presentations() {
        assert_eq!(
            branch_choices(&Reading::Empty, &Writing::default(), ""),
            BranchChoices::Waiting
        );
        assert_eq!(
            branch_choices(&Reading::Failed(Problem::Older), &Writing::default(), ""),
            BranchChoices::Failed(&Problem::Older)
        );
        assert_eq!(
            branch_choices(
                &Reading::Ready(BranchList {
                    branches: vec![branch("main", true)],
                    more: true
                }),
                &Writing::default(),
                "feature"
            ),
            BranchChoices::Rows {
                rows: vec![],
                more: true
            }
        );
    }
}
