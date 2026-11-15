//! A session's pull request, as the git row and its dialogs read it
//! (protocol 11's `pr.*` calls, ADR 0029): whether "Create PR" or "View
//! PR" is offered and why not (Paseo's `git/policy.ts`), the open form's
//! request and its defaults, and the facts a person reads (Paseo's
//! `git/pull-request-panel`).
//!
//! [Decision Log]
//! - Purpose: the rules are tested once, outside the screens, and the
//!   words are chosen from closed enums (no node message pasted into a
//!   sentence).
//! - Constraint: the summary arrives on the session (`SessionInfo`'s
//!   `pull_request`, kept fresh by the node's watcher); the whole pull
//!   request is `pr.get`. Only a live session with a project or a worktree
//!   has one; `pr.list` covers watched sessions only, so a project-wide
//!   list is not drawn from it.
//! - Choice: [`entry`] reads the session's summary and the git status;
//!   [`open`] makes `pr.open`'s request; [`facts`] reads what is shown.
//! - Cost: a newer node's unknown values read as "not known" in each enum.

pub mod bases;
pub mod card;
pub mod entry;
pub mod facts;
pub mod menu;
pub mod merge;
pub mod merge_words;
pub mod open;
pub mod words;

#[cfg(test)]
pub(crate) mod fixtures;

pub use bases::{BaseChoice, base_choices, start_base};
pub use card::{CardChecks, CardPull};
pub use entry::{PullEntry, Unavailable, pull_entry};
pub use facts::{CheckCount, ChecksOutcome, Merge, PullPhase, ReviewPills, Verdict, check_counts};
pub use menu::{MenuEntry, with_pull};
pub use merge::{Merged, merge_methods, merge_outcome, merge_request};
pub use merge_words::{merge_label, merge_refusal_words};
pub use open::{OpenDraft, OpenProblem, Opened, open_outcome, open_request, prefill};
pub use words::refusal_words;
