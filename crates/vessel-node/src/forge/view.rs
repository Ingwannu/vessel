//! What a held session is shown: its `SessionInfo.pull_request`, the view
//! its actor places it by, and `pr.get`'s answer, all from the session and
//! its watch (ADR 0029).
//!
//! - The access is the watch's last failure, else `ready`; a session on
//!   no watch says why (`no_branch`, `limit`, ...).
//! - The summary is the last one read, kept while a later look fails, so
//!   a card says what was last known.
//! - The facts are `Seen` once the branch was looked at, with its one pull
//!   request or none; `Unseen` before, and off a watch.
//! - The policy follows the feedback switch (agent-orchestrator's one
//!   flag): with feedback on, failing checks and requested changes are
//!   the fleet's to send, so the reducer places the session as fixing
//!   them.

use std::sync::Arc;
use std::time::Duration;

use vessel_proto::forge::{ForgeAccess, PullRequestView, SessionPullRequest};
use vessel_session::ForgeView;
use vessel_session::board::{Policy, PullRequestFacts};

use super::watcher::{Session, Watch};

/// A refresh is a look at most this often per session; more often is
/// answered from memory.
pub(super) const REFRESH_EVERY: Duration = Duration::from_secs(10);

fn access(held: &Session, watch: Option<&Watch>) -> ForgeAccess {
    match watch {
        Some(watch) => watch.error.clone().unwrap_or_else(|| ForgeAccess::Ready {
            login: watch.login.clone(),
        }),
        None => held.access.clone(),
    }
}

/// What `SessionInfo.pull_request` holds.
pub(super) fn info(held: &Session, watch: Option<&Watch>) -> SessionPullRequest {
    SessionPullRequest {
        access: access(held, watch),
        summary: watch
            .and_then(|watch| watch.read.as_ref())
            .map(|read| read.pull.summary.clone()),
        feedback: held.feedback == Some(true),
    }
}

/// What the session's actor is given.
pub(super) fn forge_view(held: &Session, watch: Option<&Watch>) -> ForgeView {
    let seen = watch.filter(|watch| watch.seen).map(|watch| {
        let facts: Vec<PullRequestFacts> = watch
            .read
            .iter()
            .map(|read| PullRequestFacts::from_pull(&read.pull))
            .collect();
        Arc::from(facts)
    });
    ForgeView {
        info: info(held, watch),
        seen,
        policy: Policy {
            auto_review: false,
            auto_inject_review: held.feedback == Some(true) && held.structured,
            auto_inject_ci: held.feedback == Some(true) && held.structured,
        },
    }
}

/// What `pr.get` answers.
pub(super) fn pull_request(held: &Session, watch: Option<&Watch>) -> PullRequestView {
    PullRequestView {
        access: access(held, watch),
        pull_request: watch
            .and_then(|watch| watch.read.as_ref())
            .map(|read| Box::new(read.pull.clone())),
        feedback: held.feedback == Some(true),
    }
}
