//! A pull request's activity for the pane (ADR 0088, `pr.timeline`): its
//! reviews, conversation comments and review threads in one GraphQL query,
//! Paseo's `PULL_REQUEST_TIMELINE_QUERY` bounds (100 of each, 100 comments
//! a thread) without its rendered HTML and avatars (the pane draws the
//! markdown and an initial).
//!
//! [Decision Log]
//! - Purpose: Paseo's Activity section lists reviews, comments and
//!   threads with authors and ages; the feedback read (`read`) keeps only
//!   the latest review per person and no times, so it cannot serve.
//! - Choice: a separate query, asked only when the pane asks; a review
//!   still pending (no `submittedAt`) is not activity; any `hasNextPage`
//!   is `truncated`, as Paseo's.

use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::Deserialize;
use vessel_proto::forge::{
    ForgeItemId, ForgeLogin, ForgePath, ForgeUrl, PullBody, PullNumber, RefusedReason, RepoRef,
    ReviewState, ThreadPlace, TimelineItem,
};

use super::{Ask, GitHub, decode, graphql, no_access, owner_name};
use crate::http::{Reply, Resource};
use crate::provider::{ForgeError, Priority, Timeline};

/// The query (100 reviews, 100 comments, 100 threads of 100 comments).
const TIMELINE: &str = "query($owner:String!,$name:String!,$number:Int!){\
repository(owner:$owner,name:$name){pullRequest(number:$number){\
reviews(first:100){nodes{id state body url submittedAt author{login}} pageInfo{hasNextPage}} \
comments(first:100){nodes{id body url createdAt author{login}} pageInfo{hasNextPage}} \
reviewThreads(first:100){nodes{id path line startLine isResolved isOutdated \
comments(first:100){nodes{id body url createdAt author{login} pullRequestReview{id}} \
pageInfo{hasNextPage}}} pageInfo{hasNextPage}}}}}";

#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    data: Option<Data>,
    #[serde(default)]
    errors: Vec<graphql::Error>,
}

#[derive(Debug, Deserialize)]
struct Data {
    repository: Option<Repository>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    pull_request: Option<Pull>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    reviews: Page<ReviewNode>,
    comments: Page<CommentNode>,
    review_threads: Page<ThreadNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    #[serde(default = "Vec::new")]
    nodes: Vec<T>,
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
}

#[derive(Debug, Deserialize)]
struct Login {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewNode {
    id: String,
    state: String,
    #[serde(default)]
    body: String,
    url: Option<String>,
    submitted_at: Option<DateTime<Utc>>,
    author: Option<Login>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentNode {
    id: String,
    #[serde(default)]
    body: String,
    url: Option<String>,
    created_at: DateTime<Utc>,
    author: Option<Login>,
    #[serde(default)]
    pull_request_review: Option<Id>,
}

#[derive(Debug, Deserialize)]
struct Id {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadNode {
    id: String,
    path: Option<String>,
    line: Option<u32>,
    start_line: Option<u32>,
    is_resolved: bool,
    is_outdated: bool,
    comments: Page<CommentNode>,
}

impl GitHub {
    /// `number`'s reviews, comments and threads, oldest first (a person's
    /// read: the pane asks it).
    pub(super) async fn activity(
        &self,
        repo: &RepoRef,
        number: PullNumber,
    ) -> Result<Timeline, ForgeError> {
        let (owner, name) = owner_name(repo)?;
        let query = serde_json::json!({
            "query": TIMELINE,
            "variables": { "owner": owner, "name": name, "number": number.get() },
        });
        let url = self.graphql(&repo.host)?;
        let ask = Ask::json(
            Method::POST,
            url,
            Resource::Graphql,
            Priority::Person,
            &query,
        )?;
        let Reply::Body { status, body } = self.send(&repo.host, ask).await? else {
            return Err(ForgeError::Unexpected {
                what: "a GraphQL answer",
            });
        };
        if !status.is_success() {
            return Err(no_access(status).unwrap_or(ForgeError::Unexpected {
                what: "a GraphQL answer",
            }));
        }
        let answer: Answer = decode(&body, "a pull request's timeline")?;
        if answer
            .errors
            .iter()
            .any(|error| error.kind.as_deref() == Some("RATE_LIMITED"))
        {
            let until = self.http.limited(&repo.host);
            return Err(ForgeError::RateLimited { until });
        }
        let Some(pull) = answer.data.and_then(|data| data.repository) else {
            return Err(ForgeError::Refused {
                reason: RefusedReason::NoAccess,
            });
        };
        let pull = pull.pull_request.ok_or(ForgeError::NotFound)?;
        Ok(timeline(pull))
    }
}

/// The answer's items in Paseo's order (time, then id).
fn timeline(pull: Pull) -> Timeline {
    let mut truncated = pull.reviews.page_info.has_next_page
        || pull.comments.page_info.has_next_page
        || pull.review_threads.page_info.has_next_page;
    let mut items: Vec<TimelineItem> = pull.reviews.nodes.into_iter().filter_map(review).collect();
    items.extend(
        pull.comments
            .nodes
            .into_iter()
            .filter_map(|node| comment(node, None)),
    );
    for thread in pull.review_threads.nodes {
        truncated |= thread.comments.page_info.has_next_page;
        let Some(id) = ForgeItemId::lossy(&thread.id) else {
            continue;
        };
        let place = ThreadPlace {
            thread: id,
            path: thread.path.as_deref().and_then(ForgePath::lossy),
            line: thread.line,
            start_line: thread.start_line,
            resolved: thread.is_resolved,
            outdated: thread.is_outdated,
        };
        items.extend(
            thread
                .comments
                .nodes
                .into_iter()
                .filter_map(|node| comment(node, Some(place.clone()))),
        );
    }
    items.sort_by(|a, b| {
        a.created_at()
            .cmp(&b.created_at())
            .then_with(|| a.id().cmp(b.id()))
    });
    Timeline { items, truncated }
}

/// A submitted review; `None` for a pending one or an id out of shape.
fn review(node: ReviewNode) -> Option<TimelineItem> {
    Some(TimelineItem::Review {
        id: ForgeItemId::lossy(&node.id)?,
        author: node
            .author
            .and_then(|author| ForgeLogin::lossy(&author.login)),
        body: PullBody::lossy(&node.body),
        created_at: node.submitted_at?,
        url: node.url.as_deref().and_then(ForgeUrl::lossy),
        state: match node.state.as_str() {
            "APPROVED" => ReviewState::Approved,
            "CHANGES_REQUESTED" => ReviewState::ChangesRequested,
            _ => ReviewState::Commented,
        },
    })
}

/// A comment, in `thread` when it is a review thread's.
fn comment(node: CommentNode, thread: Option<ThreadPlace>) -> Option<TimelineItem> {
    Some(TimelineItem::Comment {
        id: ForgeItemId::lossy(&node.id)?,
        author: node
            .author
            .and_then(|author| ForgeLogin::lossy(&author.login)),
        body: PullBody::lossy(&node.body),
        created_at: node.created_at,
        url: node.url.as_deref().and_then(ForgeUrl::lossy),
        review: node
            .pull_request_review
            .and_then(|review| ForgeItemId::lossy(&review.id)),
        thread,
    })
}
