//! Whether the git menu offers "Create PR" or "View PR", and why "Create
//! PR" would be refused: Paseo's `buildPrAction` and
//! `getCreatePrUnavailableMessage` (`git/policy.ts`), read from what the
//! node says of the session (`SessionPullRequest`).
//!
//! [Decision Log]
//! - Purpose: Paseo offers the pull request action only off the base
//!   branch; with one open it is "View PR", else "Create PR", which stays
//!   in the menu when it cannot run and answers with why (its Info toast).
//! - Constraint: the node knows the forge (`ForgeAccess`); the app does
//!   not ask GitHub itself. A summary kept while access later failed is
//!   still shown (the node keeps the last one read).
//! - Choice: one rule from the session's pull request and the git status;
//!   every reason Paseo's two sentences cover, and the node's finer ones
//!   (signed out, refused, rate-limited, unreachable) as their own words.
//! - Cost: none beyond the enum; a newer node's reason reads as unknown.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{
    ForgeAccess, ForgeHost, ForgeUrl, PullNumber, PullSummary, SessionPullRequest,
};
use vessel_proto::workspace::GitStatus;

use super::facts::PullPhase;

/// The pull request entry of the git menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullEntry {
    /// Not offered: on the base branch, or the session has no repository
    /// the node watches (Paseo shows no pull request action there).
    Hidden,
    /// "View PR": the one the node read.
    View {
        /// Its number.
        number: PullNumber,
        /// Its page.
        url: ForgeUrl,
        /// Where it is in its life.
        phase: PullPhase,
    },
    /// "Create PR": opens the form.
    Create,
    /// "Create PR", which would be refused: says why instead.
    Unavailable(Unavailable),
}

/// Why "Create PR" cannot run now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// The branch has nothing its base does not (Paseo's
    /// `createPrNoCommits`).
    NoCommits,
    /// On no branch.
    NoBranch,
    /// The branch's remote does not exist.
    NoRemote,
    /// No forge this node reads for `host` (Paseo's `createPrNoForge`).
    NotSupported(ForgeHost),
    /// The node is not signed in to `host`.
    SignedOut(ForgeHost),
    /// `host` refused the node's sign-in.
    Refused(ForgeHost),
    /// `host`'s rate limit holds until then.
    RateLimited(ForgeHost, DateTime<Utc>),
    /// `host` could not be reached; the node tries again then.
    Unreachable(ForgeHost, DateTime<Utc>),
    /// The node watches as many as it allows.
    Limit,
    /// The session ended.
    Ended,
    /// A reason this app does not know.
    Unknown,
}

/// The entry for a session whose pull request the node says `pull` (absent
/// when the session has none watched) and whose folder's git is `status`.
#[must_use]
pub fn pull_entry(pull: Option<&SessionPullRequest>, status: &GitStatus) -> PullEntry {
    let Some(pull) = pull else {
        return PullEntry::Hidden;
    };
    if let Some(summary) = &pull.summary {
        return view(summary);
    }
    if status.branch.is_none() {
        return PullEntry::Unavailable(Unavailable::NoBranch);
    }
    match &pull.access {
        ForgeAccess::Ready { .. } => {
            let nothing_new = status.base.as_ref().is_some_and(|base| base.ahead == 0);
            if nothing_new {
                PullEntry::Unavailable(Unavailable::NoCommits)
            } else {
                PullEntry::Create
            }
        }
        ForgeAccess::OnBase | ForgeAccess::NotPlaced => PullEntry::Hidden,
        refused @ (ForgeAccess::NoBranch
        | ForgeAccess::NoRemote
        | ForgeAccess::NotSupported { .. }
        | ForgeAccess::SignedOut { .. }
        | ForgeAccess::Refused { .. }
        | ForgeAccess::RateLimited { .. }
        | ForgeAccess::Unreachable { .. }
        | ForgeAccess::Limit
        | ForgeAccess::Ended
        | ForgeAccess::Unknown) => PullEntry::Unavailable(Unavailable::of(refused)),
    }
}

fn view(summary: &PullSummary) -> PullEntry {
    PullEntry::View {
        number: summary.number,
        url: summary.url.clone(),
        phase: PullPhase::of(summary),
    }
}

impl Unavailable {
    /// Why `access` keeps a pull request from being opened (also what the
    /// facts say when the node could not read one).
    #[must_use]
    pub fn of(access: &ForgeAccess) -> Self {
        unavailable_of(access)
    }
}

/// Why `access` keeps a pull request from being opened.
fn unavailable_of(access: &ForgeAccess) -> Unavailable {
    match access {
        ForgeAccess::NoBranch => Unavailable::NoBranch,
        ForgeAccess::NoRemote => Unavailable::NoRemote,
        ForgeAccess::NotSupported { host, .. } => Unavailable::NotSupported(host.clone()),
        ForgeAccess::SignedOut { host, .. } => Unavailable::SignedOut(host.clone()),
        ForgeAccess::Refused { host, .. } => Unavailable::Refused(host.clone()),
        ForgeAccess::RateLimited { host, until, .. } => {
            Unavailable::RateLimited(host.clone(), *until)
        }
        ForgeAccess::Unreachable { host, retry_at, .. } => {
            Unavailable::Unreachable(host.clone(), *retry_at)
        }
        ForgeAccess::Limit => Unavailable::Limit,
        ForgeAccess::Ended => Unavailable::Ended,
        ForgeAccess::Ready { .. }
        | ForgeAccess::OnBase
        | ForgeAccess::NotPlaced
        | ForgeAccess::Unknown => Unavailable::Unknown,
    }
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
