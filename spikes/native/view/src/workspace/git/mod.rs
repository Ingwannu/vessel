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
mod discard;
mod form;
mod stash;
mod state;
mod written;

pub use branch::{BranchMode, BranchRequest, SwitchProblem, branch_request, switch_request};
pub use discard::{DiscardDecision, DiscardEffect};
pub use form::{ChangedFiles, CommitForm, IndexAction, commit_request, file_actions};
pub use stash::{StashChoice, stash_request};
pub use state::{KeysExhausted, Reading, RequestKey, RequestKeys};
pub use written::{Success, UpstreamChange, Writing, Written};

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod written_tests;
