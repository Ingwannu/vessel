//! The common summary decoder of a whole pull and a project page.
//! Both paths use the same checks/review/mergeability policy; a list may
//! not invent a second interpretation of a pull request's facts.

use super::super::facts::{self, Context, MergeFacts};
use super::{CommitNode, ContextNode, Nodes};
use crate::provider::ForgeError;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::forge::{
    Checks, ForgeUrl, PullNumber, PullState, PullSummary, PullTitle, RepoRef,
};
use vessel_proto::workspace::CommitId;

/// Shared external summary fields, owned only by this decoding edge.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Fields {
    number: u64,
    url: String,
    title: String,
    state: State,
    is_draft: bool,
    updated_at: DateTime<Utc>,
    head_ref_oid: String,
    mergeable: Option<Mergeability>,
    merge_state_status: Option<MergeState>,
    review_decision: Option<Review>,
    commits: Nodes<CommitNode>,
}

impl Fields {
    /// Preserve the original review-head comparison without cloning or
    /// changing the existing whole read's error precedence.
    pub(super) fn head_text(&self) -> &str {
        &self.head_ref_oid
    }

    /// Parse the essential head before a whole read checks its branches.
    pub(super) fn head(&self) -> Result<CommitId, ForgeError> {
        self.head_ref_oid
            .parse()
            .map_err(|_bad| ForgeError::Unexpected {
                what: "a head commit",
            })
    }
}

/// Both whole and project reads use this single facts conversion.
pub(super) fn read(
    repo: &RepoRef,
    fields: Fields,
    head: CommitId,
) -> Result<(PullSummary, Checks), ForgeError> {
    let number = PullNumber::new(fields.number)
        .map_err(|_bad| ForgeError::Unexpected { what: "a number" })?;
    let url = ForgeUrl::lossy(&fields.url).ok_or(ForgeError::Unexpected { what: "an address" })?;
    let checks = checks(fields.commits);
    let review = facts::review(fields.review_decision.as_ref().map(Review::wire));
    let mergeable = facts::mergeable(MergeFacts {
        mergeable: fields.mergeable.as_ref().map(Mergeability::wire),
        merge_state: fields.merge_state_status.as_ref().map(MergeState::wire),
        draft: fields.is_draft,
        checks: checks.state,
        review,
    });
    Ok((
        PullSummary {
            repo: repo.clone(),
            number,
            url,
            title: PullTitle::lossy(&fields.title),
            state: fields.state.into(),
            draft: fields.is_draft,
            head_commit: head,
            checks: checks.state,
            review,
            mergeable,
            updated_at: fields.updated_at,
        },
        checks,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum State {
    Open,
    Merged,
    Closed,
    #[serde(other)]
    Unknown,
}

impl From<State> for PullState {
    fn from(state: State) -> Self {
        match state {
            State::Open => Self::Open,
            State::Merged => Self::Merged,
            State::Closed => Self::Closed,
            State::Unknown => Self::Unknown,
        }
    }
}

// These adapters preserve the existing facts functions' one policy.
// Unknown external variants remain Unknown rather than becoming None.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Review {
    Approved,
    ChangesRequested,
    ReviewRequired,
    #[serde(other)]
    Unknown,
}
impl Review {
    fn wire(&self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::ChangesRequested => "CHANGES_REQUESTED",
            Self::ReviewRequired => "REVIEW_REQUIRED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Mergeability {
    Mergeable,
    Conflicting,
    #[serde(other)]
    Unknown,
}
impl Mergeability {
    fn wire(&self) -> &'static str {
        match self {
            Self::Mergeable => "MERGEABLE",
            Self::Conflicting => "CONFLICTING",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum MergeState {
    Behind,
    Blocked,
    Clean,
    Dirty,
    Draft,
    HasHooks,
    Unstable,
    #[serde(other)]
    Unknown,
}
impl MergeState {
    fn wire(&self) -> &'static str {
        match self {
            Self::Behind => "BEHIND",
            Self::Blocked => "BLOCKED",
            Self::Clean => "CLEAN",
            Self::Dirty => "DIRTY",
            Self::Draft => "DRAFT",
            Self::HasHooks => "HAS_HOOKS",
            Self::Unstable => "UNSTABLE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// The head commit's checks, from its rollup (the last commit's); more
/// contexts than one read takes is `truncated`.
fn checks(commits: Nodes<CommitNode>) -> Checks {
    let rollup = commits
        .nodes
        .into_iter()
        .next()
        .and_then(|node| node.commit.status_check_rollup);
    let (contexts, total) = rollup.map_or((Vec::new(), 0), |rollup| {
        (rollup.contexts.nodes, rollup.contexts.total_count)
    });
    let contexts: Vec<Context> = contexts.into_iter().filter_map(context).collect();
    let truncated = u64::try_from(contexts.len()).unwrap_or(u64::MAX) < total;
    facts::checks(&contexts, truncated)
}

fn context(node: ContextNode) -> Option<Context> {
    match node {
        ContextNode::CheckRun {
            name,
            status,
            conclusion,
            details_url,
            started_at,
        } => Some(Context {
            state: facts::check_run(&status, conclusion.as_deref()),
            name,
            at: started_at,
            url: details_url,
        }),
        ContextNode::StatusContext {
            context,
            state,
            target_url,
            created_at,
        } => Some(Context {
            state: facts::status_context(&state),
            name: context,
            at: created_at,
            url: target_url,
        }),
        ContextNode::Other => None,
    }
}
