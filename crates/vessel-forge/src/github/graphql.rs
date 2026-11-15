//! The one GraphQL read of a pull request: everything the facts and
//! feedback need that REST has no single answer for (the review decision,
//! threads and their resolution, the latest review per person, the
//! rollup's contexts, the repository's merge methods), and its answer
//! read into the wire's `PullRequest` and a `Discussion` (`read`).

mod project;
mod read;
mod summary;

pub(super) use project::{LIST, PageData, page};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub(super) use self::read::read;

/// Fixed GitHub connection work, not a caller-selected unbounded count.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
pub(super) struct QueryLimit(usize);

#[cfg(test)]
impl QueryLimit {
    /// Compare typed captured wire variables with the fixed query budget.
    pub(super) const fn get(self) -> usize {
        self.0
    }
}

/// The existing whole-read context budget, also used by project summaries.
pub(super) const MAX_CONTEXTS: QueryLimit = QueryLimit(100);
/// Only the current head commit contributes checks.
pub(super) const HEAD_COMMITS: QueryLimit = QueryLimit(1);
/// The public page bound also limits the query's requested rows.
pub(super) const PROJECT_PAGE_LIMIT: QueryLimit =
    QueryLimit(vessel_proto::forge::OpenPullPage::MAX_PULLS);

/// A typed GraphQL envelope, shared by whole and project reads.
#[derive(Serialize)]
pub(super) struct Query<'a, V> {
    pub(super) query: &'a str,
    pub(super) variables: V,
}

/// The whole query retains its existing nested review/thread budgets.
pub(super) const READ: &str = "query($owner:String!,$name:String!,$number:Int!,$contexts:Int!,$heads:Int!){\
repository(owner:$owner,name:$name){mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed \
pullRequest(number:$number){number url title state isDraft createdAt updatedAt mergedAt closedAt \
author{login __typename} headRefName headRefOid baseRefName mergeable mergeStateStatus reviewDecision \
commits(last:$heads){nodes{commit{statusCheckRollup{contexts(first:$contexts){totalCount nodes{__typename \
...on CheckRun{name status conclusion detailsUrl startedAt} \
...on StatusContext{context state targetUrl createdAt}}}}}}} \
latestReviews(first:50){nodes{id state body url author{login __typename} commit{oid}}} \
reviewThreads(first:50){nodes{id isResolved path line comments(first:20){nodes{id body url \
author{login __typename}}}}}}}}";

/// The whole answer.
#[derive(Debug, Deserialize)]
pub(super) struct Answer<T = Data> {
    pub(super) data: Option<T>,
    #[serde(default)]
    pub(super) errors: Vec<Error>,
}

/// One GraphQL error.
#[derive(Debug, Deserialize)]
pub(super) struct Error {
    #[serde(default, rename = "type")]
    pub(super) kind: Option<ErrorKind>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum ErrorKind {
    RateLimited,
    #[serde(other)]
    Unknown,
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
    #[serde(flatten)]
    summary: summary::Fields,
    created_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    author: Option<Actor>,
    head_ref_name: String,
    base_ref_name: String,
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

impl<T> Answer<T> {
    /// Whether GitHub answered with its own rate limit (a 200 with an
    /// error of type `RATE_LIMITED`, GOAT's trap).
    pub(super) fn limited(&self) -> bool {
        self.errors
            .iter()
            .any(|error| matches!(error.kind, Some(ErrorKind::RateLimited)))
    }
}
