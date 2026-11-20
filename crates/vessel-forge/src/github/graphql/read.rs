//! The GraphQL answer read into the wire's pull request and the
//! discussion feedback needs. Every value a forge gives as text is made
//! with a lossy constructor or dropped, so a forge's text cannot fail a
//! read; only what the facts cannot do without (the head, the branches,
//! the number, the address) fails it.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{
    CheckTally, Checks, ForgeLogin, ForgeUrl, MergeMethod, PullNumber, PullRequest, PullState,
    PullSummary, PullTitle, RepoRef, Reviews,
};

use super::super::facts::{self, Context, MergeFacts};
use super::{Actor, CommitNode, ContextNode, Nodes, Repository, ReviewNode, ThreadNode};
use crate::provider::{
    Author, Comment, Discussion, FailedCheck, ForgeError, ItemId, Read, Review, ReviewVerdict,
    Thread,
};

/// The answer read into the wire's pull request and the discussion, as
/// of `at`.
pub(in crate::github) fn read(
    repo: &RepoRef,
    repository: Repository,
    at: DateTime<Utc>,
) -> Result<Read, ForgeError> {
    let unexpected = |what| ForgeError::Unexpected { what };
    let merge_methods = merge_methods(&repository);
    let pull = repository.pull_request.ok_or(ForgeError::NotFound)?;
    let head_commit = pull
        .head_ref_oid
        .parse()
        .map_err(|_bad| unexpected("a head commit"))?;
    let head_branch = pull
        .head_ref_name
        .parse()
        .map_err(|_bad| unexpected("a head branch"))?;
    let base = pull
        .base_ref_name
        .parse()
        .map_err(|_bad| unexpected("a base branch"))?;
    let number = PullNumber::new(pull.number).map_err(|_bad| unexpected("a number"))?;
    let url = ForgeUrl::lossy(&pull.url).ok_or_else(|| unexpected("an address"))?;
    let checks = checks(pull.commits);
    let review = facts::review(pull.review_decision.as_deref());
    let mergeable = facts::mergeable(MergeFacts {
        mergeable: pull.mergeable.as_deref(),
        merge_state: pull.merge_state_status.as_deref(),
        draft: pull.is_draft,
        checks: checks.state,
        review,
    });
    let reviews = reviews(pull.latest_reviews, &pull.head_ref_oid);
    let threads = threads(pull.review_threads);
    let people = people(&reviews, &threads);
    let failed = checks
        .failing
        .iter()
        .map(|check| FailedCheck {
            name: check.name.clone(),
            url: check.url.clone(),
        })
        .collect();
    Ok(Read {
        pull: PullRequest {
            summary: PullSummary {
                repo: repo.clone(),
                number,
                url,
                title: PullTitle::lossy(&pull.title),
                state: state(&pull.state),
                draft: pull.is_draft,
                head_commit,
                checks: checks.state,
                review,
                mergeable,
                updated_at: pull.updated_at,
                tally: CheckTally::of(&checks),
            },
            author: pull
                .author
                .as_ref()
                .and_then(|actor| ForgeLogin::lossy(&actor.login)),
            head_branch,
            base,
            checks,
            reviews: people,
            created_at: pull.created_at,
            merged_at: pull.merged_at,
            closed_at: pull.closed_at,
            merge_methods,
            read_at: at,
        },
        discussion: Discussion {
            reviews,
            threads,
            failed,
        },
    })
}

fn state(state: &str) -> PullState {
    match state {
        "OPEN" => PullState::Open,
        "MERGED" => PullState::Merged,
        "CLOSED" => PullState::Closed,
        _ => PullState::Unknown,
    }
}

/// The methods the repository allows, in the wire's order.
fn merge_methods(repository: &Repository) -> Vec<MergeMethod> {
    [
        (repository.merge_commit_allowed, MergeMethod::Merge),
        (repository.squash_merge_allowed, MergeMethod::Squash),
        (repository.rebase_merge_allowed, MergeMethod::Rebase),
    ]
    .into_iter()
    .filter_map(|(allowed, method)| allowed.then_some(method))
    .collect()
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

/// The latest review per person on the head `head` (a review of an older
/// head no longer speaks for this one).
fn reviews(nodes: Nodes<ReviewNode>, head: &str) -> Vec<Review> {
    nodes
        .nodes
        .into_iter()
        .filter(|node| node.commit.as_ref().is_none_or(|commit| commit.oid == head))
        .filter_map(|node| {
            Some(Review {
                id: ItemId::new(&node.id)?,
                author: author(node.author.as_ref()),
                verdict: match node.state.as_str() {
                    "APPROVED" => ReviewVerdict::Approved,
                    "CHANGES_REQUESTED" => ReviewVerdict::ChangesRequested,
                    _ => ReviewVerdict::Other,
                },
                body: node.body,
                url: node.url.as_deref().and_then(ForgeUrl::lossy),
            })
        })
        .collect()
}

fn threads(nodes: Nodes<ThreadNode>) -> Vec<Thread> {
    nodes
        .nodes
        .into_iter()
        .filter_map(|node| {
            Some(Thread {
                id: ItemId::new(&node.id)?,
                resolved: node.is_resolved,
                path: node.path,
                line: node.line,
                comments: node
                    .comments
                    .nodes
                    .into_iter()
                    .filter_map(|comment| {
                        Some(Comment {
                            id: ItemId::new(&comment.id)?,
                            author: author(comment.author.as_ref()),
                            body: comment.body,
                            url: comment.url.as_deref().and_then(ForgeUrl::lossy),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

/// People's reviews (agent-orchestrator's `ExternalReviewFacts`): bots
/// left out.
fn people(reviews: &[Review], threads: &[Thread]) -> Reviews {
    let by_person = |verdict| {
        reviews
            .iter()
            .any(|review| !review.author.bot && review.verdict == verdict)
    };
    Reviews {
        approved: by_person(ReviewVerdict::Approved),
        changes_requested: by_person(ReviewVerdict::ChangesRequested),
        commented: threads.iter().any(|thread| {
            !thread.resolved && thread.comments.iter().any(|comment| !comment.author.bot)
        }),
        unresolved: u32::try_from(threads.iter().filter(|thread| !thread.resolved).count())
            .unwrap_or(u32::MAX),
    }
}

fn author(actor: Option<&Actor>) -> Author {
    Author {
        login: actor.and_then(|actor| ForgeLogin::lossy(&actor.login)),
        bot: actor.is_some_and(|actor| actor.typename == "Bot"),
    }
}

fn context(node: ContextNode) -> Option<Context> {
    match node {
        ContextNode::CheckRun {
            name,
            status,
            conclusion,
            details_url,
            started_at,
            completed_at,
            database_id,
            check_suite,
        } => {
            let workflow_run = check_suite.and_then(|suite| suite.workflow_run);
            Some(Context {
                state: facts::check_run(&status, conclusion.as_deref()),
                name,
                at: started_at,
                url: details_url,
                run: database_id,
                workflow_run: workflow_run.as_ref().and_then(|run| run.database_id),
                workflow: workflow_run
                    .and_then(|run| run.workflow)
                    .map(|flow| flow.name),
                completed_at,
            })
        }
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
            run: None,
            workflow_run: None,
            workflow: None,
            completed_at: None,
        }),
        ContextNode::Other => None,
    }
}
