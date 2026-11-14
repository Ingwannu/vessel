//! Git action preparation for the workspace's menu and file rows. The
//! link sends these wire requests; this module never runs git or draws.
//!
//! [Decision Log]
//! - Purpose: keep the menu's decisions out of its GPUI callbacks.
//! - Constraints: the node owns validation and refusal; the page's
//!   commit-dialog, branch-dialog and discard-dialog define the flow.
//! - Alternatives: duplicate the node's git rules, or decide in drawing.
//! - Choice: reuse wire constructors and the existing `git_quick` and
//!   workspace words. Destructive requests require a prepared decision.
//! - Why: the node remains authoritative and both surfaces use the same
//!   inputs; a stash is pinned by commit as well as its moving index.
//! - Cost: remote branch names can be valid refs but invalid local names;
//!   preparation returns that typed error before a request can be sent.

mod branch;
mod branch_picker;
mod commit_row;
mod destination;
mod dialog;
mod discard;
mod drop_stash;
mod file_menu;
mod form;
mod header;
mod intent;
mod issue;
mod menu;
mod poll;
mod repository;
mod scopes;
mod stash;
mod stash_subject;
mod state;
mod step;
mod submit;
mod switch_stash;
mod written;

pub use branch::{BranchMode, BranchRequest, SwitchProblem, branch_request, switch_request};
pub use branch_picker::{
    BranchChoice, BranchChoices, BranchSelection, PickerVisibility, branch_choices, branch_failure,
    branch_selection,
};
pub use commit_row::committed_state;
pub use destination::{CommitDestination, names_default, project_default};
pub use dialog::Dialog;
pub use discard::{DiscardDecision, DiscardEffect};
pub use drop_stash::DropStash;
pub use file_menu::{FileAction, FileMenu, file_menu};
pub use form::{
    ChangedFiles, CommitForm, IndexAction, StatusCoverage, commit_files, commit_request,
    file_actions,
};
pub use header::{
    DistanceName, GitHead, HeadActivity, HeadName, HeadState, git_head, head_activity,
};
pub use intent::WriteIntent;
pub use issue::Issue;
pub use menu::{
    Action, ActionRow, ControlDensity, PanelWidth, PrimaryControl, StashScope, Unavailable,
    availability, filter_branches, primary, primary_control, rows,
};
pub use poll::{PanelVisibility, REFRESH_EVERY, should_refresh};
pub use repository::{Repository, matching_repository};
pub use scopes::{CommitScope, commit_scopes};
pub use stash::{StashChoice, stash_request};
pub use stash_subject::{StashSubject, stash_subject};
pub use state::{KeysExhausted, Reading, RequestKey, RequestKeys};
pub use step::{DirectWrite, Step, step};
pub use submit::{can_branch, can_commit};
pub use switch_stash::{offers_stash, saved, stash_for};
pub use written::{SUCCESS_DISPLAY, Success, SuccessReceipt, UpstreamChange, Writing, Written};

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod written_tests;

#[cfg(test)]
mod menu_tests;

#[cfg(test)]
mod header_tests;

#[cfg(test)]
mod eligibility_tests;
