//! The one GraphQL read of a pull request: everything the facts and
//! feedback need that REST has no single answer for (the review decision,
//! threads and their resolution, the latest review per person, the
//! rollup's contexts, the repository's merge methods), and its answer
//! read into the wire's `PullRequest` and a `Discussion` (`read`).

mod read;

use chrono::{DateTime, Utc};
use serde::Deserialize;

pub(super) use self::read::read;

/// The query. Bounded: 100 contexts, 50 reviews, 50 threads of 20
/// comments each.
pub(super) const READ: &str = "query($owner:String!,$name:String!,$number:Int!){\
repository(owner:$owner,name:$name){mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed \
pullRequest(number:$number){number url title state isDraft createdAt updatedAt mergedAt closedAt \
author{login __typename} headRefName headRefOid baseRefName mergeable mergeStateStatus reviewDecision \
commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100){totalCount nodes{__typename \
...on CheckRun{name status conclusion detailsUrl startedAt} \
...on StatusContext{context state targetUrl createdAt}}}}}}} \
latestReviews(first:50){nodes{id state body url author{login __typename} commit{oid}}} \
reviewThreads(first:50){nodes{id isResolved path line comments(first:20){nodes{id body url \
author{login __typename}}}}}}}}";

/// The whole answer.
#[derive(Debug, Deserialize)]
pub(super) struct Answer {
    #[serde(default)]
    pub(super) data: Option<Data>,
    #[serde(default)]
    pub(super) errors: Vec<Error>,
}

/// One GraphQL error.
#[derive(Debug, Deserialize)]
pub(super) struct Error {
    #[serde(default, rename = "type")]
    pub(super) kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Data {
    pub(super) repository: Option<Repository>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Repository {
    merge_commit_allowed: bool,
    squash_merge_allowed: bool,
    rebase_merge_allowed: bool,
    pull_request: Option<Pull>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    number: u64,
    url: String,
    title: String,
    state: String,
    is_draft: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    author: Option<Actor>,
    head_ref_name: String,
    head_ref_oid: String,
    base_ref_name: String,
    mergeable: Option<String>,
    merge_state_status: Option<String>,
    review_decision: Option<String>,
    commits: Nodes<CommitNode>,
    latest_reviews: Nodes<ReviewNode>,
    review_threads: Nodes<ThreadNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Actor {
    login: String,
    #[serde(rename = "__typename")]
    typename: String,
}

#[derive(Debug, Deserialize)]
struct Nodes<T> {
    #[serde(default = "Vec::new")]
    nodes: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct CommitNode {
    commit: CommitFacts,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitFacts {
    status_check_rollup: Option<Rollup>,
}

#[derive(Debug, Deserialize)]
struct Rollup {
    contexts: Contexts,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contexts {
    total_count: u64,
    #[serde(default)]
    nodes: Vec<ContextNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
#[serde(rename_all_fields = "camelCase")]
enum ContextNode {
    CheckRun {
        name: String,
        status: String,
        conclusion: Option<String>,
        details_url: Option<String>,
        started_at: Option<String>,
    },
    StatusContext {
        context: String,
        state: String,
        target_url: Option<String>,
        created_at: Option<String>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ReviewNode {
    id: String,
    state: String,
    #[serde(default)]
    body: String,
    url: Option<String>,
    author: Option<Actor>,
    commit: Option<Oid>,
}

#[derive(Debug, Deserialize)]
struct Oid {
    oid: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadNode {
    id: String,
    is_resolved: bool,
    path: Option<String>,
    line: Option<u32>,
    comments: Nodes<CommentNode>,
}

#[derive(Debug, Deserialize)]
struct CommentNode {
    id: String,
    #[serde(default)]
    body: String,
    url: Option<String>,
    author: Option<Actor>,
}

impl Answer {
    /// Whether GitHub answered with its own rate limit (a 200 with an
    /// error of type `RATE_LIMITED`, GOAT's trap).
    pub(super) fn limited(&self) -> bool {
        self.errors
            .iter()
            .any(|error| error.kind.as_deref() == Some("RATE_LIMITED"))
    }
}
