//! A session's pull request (`pr.get`, `pr.open`, `pr.merge`,
//! `pr.feedback`, protocol 11): asked and answered on the workspace's
//! pair, as `session.get` is.
//!
//! [Decision Log]
//! - Purpose: the git row's pull request actions and the pull request's
//!   facts call the node without a new variant in the lead's `Command`
//!   and `Event`.
//! - Constraint: the workspace pair already carries a session's calls
//!   (`Ask::Session`); a pull request is a session's, read beside its git.
//! - Choice: one [`PullAsk`] and one [`PullAnswer`] here, one variant each
//!   in [`super::Ask`] and [`super::Answer`]; every answer carries what
//!   was asked, so the screen that waits matches it.
//! - Cost: none beyond the two variants.

use vessel_app_view::workspace::problem::Problem;
use vessel_client::NodeClient;
use vessel_proto::SessionId;
use vessel_proto::forge::{
    GetPullRequest, MergeOutcome, MergePullRequest, OpenOutcome, OpenPullRequest, PullRequestView,
    SessionPullRequest, SetFeedback,
};

use super::calls::call;

/// What a screen asks of a session's pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullAsk {
    /// `pr.get`: it whole (from the forge now with `refresh`).
    Get(GetPullRequest),
    /// `pr.open`: open one from the session's branch.
    Open(OpenPullRequest),
    /// `pr.merge`: merge it as it was seen.
    Merge(MergePullRequest),
    /// `pr.feedback`: send its failing checks and reviews to the session,
    /// or stop.
    Feedback(SetFeedback),
}

/// What the node answered, with what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullAnswer {
    /// It whole, or why not.
    Read {
        /// Which session.
        session: SessionId,
        /// It.
        result: Result<Box<PullRequestView>, Problem>,
    },
    /// What opening it did.
    Opened {
        /// What was asked (the form matches its answer by it).
        ask: Box<OpenPullRequest>,
        /// The outcome.
        result: Result<OpenOutcome, Problem>,
    },
    /// What merging it did.
    Merged {
        /// What was asked.
        ask: Box<MergePullRequest>,
        /// The outcome.
        result: Result<MergeOutcome, Problem>,
    },
    /// The feedback switch as the node now has it.
    Feedback {
        /// What was asked.
        ask: SetFeedback,
        /// The session's pull request after it.
        result: Result<Box<SessionPullRequest>, Problem>,
    },
}

/// Run `ask` to its answer.
pub(super) async fn run(client: &NodeClient, ask: PullAsk) -> PullAnswer {
    match ask {
        PullAsk::Get(ask) => PullAnswer::Read {
            session: ask.session,
            result: call(client, ask).await.map(Box::new),
        },
        PullAsk::Open(ask) => PullAnswer::Opened {
            result: call(client, ask.clone()).await,
            ask: Box::new(ask),
        },
        PullAsk::Merge(ask) => PullAnswer::Merged {
            result: call(client, ask.clone()).await,
            ask: Box::new(ask),
        },
        PullAsk::Feedback(ask) => PullAnswer::Feedback {
            result: call(client, ask.clone()).await.map(Box::new),
            ask,
        },
    }
}

/// The answer `ask` gets when it did not reach the machine.
pub(super) fn refused(ask: PullAsk, problem: &Problem) -> PullAnswer {
    match ask {
        PullAsk::Get(ask) => PullAnswer::Read {
            session: ask.session,
            result: Err(problem.clone()),
        },
        PullAsk::Open(ask) => PullAnswer::Opened {
            ask: Box::new(ask),
            result: Err(problem.clone()),
        },
        PullAsk::Merge(ask) => PullAnswer::Merged {
            ask: Box::new(ask),
            result: Err(problem.clone()),
        },
        PullAsk::Feedback(ask) => PullAnswer::Feedback {
            ask,
            result: Err(problem.clone()),
        },
    }
}
