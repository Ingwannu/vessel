//! The GraphQL answer read into the wire's pull request and the
//! discussion feedback needs. Every value a forge gives as text is made
//! with a lossy constructor or dropped, so a forge's text cannot fail a
//! read; only what the facts cannot do without (the head, the branches,
//! the number, the address) fails it.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{ForgeLogin, ForgeUrl, MergeMethod, PullRequest, RepoRef, Reviews};

use super::{Actor, Nodes, Repository, ReviewNode, ThreadNode};
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
    let head_commit = pull.summary.head()?;
    let head_branch = pull
        .head_ref_name
        .parse()
        .map_err(|_bad| unexpected("a head branch"))?;
    let base = pull
        .base_ref_name
        .parse()
        .map_err(|_bad| unexpected("a base branch"))?;
    // Read discussion against the original head text before the common
    // summary decoder consumes its fields; no normalized comparison or
    // duplicate parse changes the existing review policy.
    let reviews = reviews(pull.latest_reviews, pull.summary.head_text());
    let (summary, checks) = super::summary::read(repo, pull.summary, head_commit)?;
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
            summary,
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
