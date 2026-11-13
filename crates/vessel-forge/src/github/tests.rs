//! GitHub's provider against a stand-in on loopback: conditional reads,
//! the GraphQL read, opening and merging, rate limits, refusals, and the
//! token only in its header. No request leaves the machine.

mod stand;

use axum::http::StatusCode;
use vessel_proto::forge::{
    ChecksState, MergeMethod, Mergeable, PullNumber, PullState, RefusedReason, ReviewDecision,
};

use self::stand::{HEAD, TOKEN, set, stand};
use crate::provider::{Forge, ForgeError, Fresh, Merged, Opened, Opening, Priority};
use crate::token::{Credentials, EnvTokens, Found, Token};

const READ: &str = r#"{"data":{"repository":{"mergeCommitAllowed":false,"squashMergeAllowed":true,
"rebaseMergeAllowed":true,"pullRequest":{"number":7,"url":"https://github.com/owner/name/pull/7",
"title":"Fix\nit","state":"OPEN","isDraft":false,"createdAt":"2026-01-01T00:00:00Z",
"updatedAt":"2026-01-02T00:00:00Z","mergedAt":null,"closedAt":null,
"author":{"login":"someone","__typename":"User"},"headRefName":"topic",
"headRefOid":"0123456789abcdef0123456789abcdef01234567","baseRefName":"main",
"mergeable":"MERGEABLE","mergeStateStatus":"BLOCKED","reviewDecision":"CHANGES_REQUESTED",
"commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"totalCount":2,"nodes":[
{"__typename":"CheckRun","name":"test","status":"COMPLETED","conclusion":"FAILURE",
"detailsUrl":"http://ci.example/1","startedAt":"2026-01-02T00:00:00Z"},
{"__typename":"StatusContext","context":"lint","state":"SUCCESS","targetUrl":null,
"createdAt":"2026-01-02T00:00:00Z"}]}}}}]},
"latestReviews":{"nodes":[{"id":"R1","state":"CHANGES_REQUESTED","body":"Please fix",
"url":"https://github.com/owner/name/pull/7#r1","author":{"login":"reviewer","__typename":"User"},
"commit":{"oid":"0123456789abcdef0123456789abcdef01234567"}}]},
"reviewThreads":{"nodes":[{"id":"T1","isResolved":false,"path":"src/a.rs","line":3,
"comments":{"nodes":[{"id":"C1","body":"Why?","url":null,
"author":{"login":"reviewer","__typename":"User"}}]}}]}}}}}"#;

#[tokio::test]
async fn a_pull_request_is_read_whole_into_the_wire_s_facts() {
    let (shared, github, repo) = stand().await;
    set(&shared, "POST /graphql", StatusCode::OK, &[], READ);
    let read = github
        .read(&repo, PullNumber::new(7).unwrap(), Priority::Background)
        .await
        .unwrap();
    let summary = &read.pull.summary;
    assert_eq!(
        summary.title.as_str(),
        "Fix it",
        "a forge's title made one line"
    );
    assert_eq!(summary.state, PullState::Open);
    assert_eq!(summary.checks, ChecksState::Failing);
    assert_eq!(summary.review, ReviewDecision::ChangesRequested);
    assert_eq!(summary.mergeable, Mergeable::Blocked);
    assert_eq!(read.pull.checks.failing.len(), 1);
    assert!(
        read.pull.checks.failing[0].url.is_none(),
        "an http details page is dropped, not refused"
    );
    assert!(read.pull.reviews.changes_requested && read.pull.reviews.commented);
    assert_eq!(read.pull.reviews.unresolved, 1);
    assert_eq!(
        read.pull.merge_methods,
        [MergeMethod::Squash, MergeMethod::Rebase]
    );
    assert_eq!(read.discussion.reviews.len(), 1);
    assert_eq!(read.discussion.threads[0].comments[0].body, "Why?");
    let asked = &shared.lock().unwrap().asked;
    assert_eq!(
        asked[0].1.as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
}

#[tokio::test]
async fn a_change_is_asked_conditionally_and_nothing_changed_costs_a_304() {
    let (shared, github, repo) = stand().await;
    let etag = [("etag", "\"v1\"")];
    set(
        &shared,
        "GET /repos/owner/name/pulls/7",
        StatusCode::OK,
        &etag,
        "{}",
    );
    set(
        &shared,
        &format!("GET /repos/owner/name/commits/{HEAD}/check-runs?per_page=100"),
        StatusCode::OK,
        &[("etag", "\"c1\"")],
        "{}",
    );
    set(
        &shared,
        &format!("GET /repos/owner/name/commits/{HEAD}/status"),
        StatusCode::OK,
        &[("etag", "\"s1\"")],
        "{}",
    );
    let number = PullNumber::new(7).unwrap();
    let head = HEAD.parse().unwrap();
    assert!(
        github
            .changed(&repo, number, &head, Priority::Background)
            .await
            .unwrap()
    );
    assert!(
        !github
            .changed(&repo, number, &head, Priority::Background)
            .await
            .unwrap(),
        "the second look sends the ETags and is answered 304"
    );
    let asked = &shared.lock().unwrap().asked;
    assert_eq!(asked.len(), 6);
    assert_eq!(asked[3].2.as_deref(), Some("\"v1\""));
}

#[tokio::test]
async fn a_branch_s_pull_requests_are_found_by_its_head() {
    let (shared, github, repo) = stand().await;
    set(
        &shared,
        "GET /repos/owner/name/pulls?head=owner%3Atopic&state=all&sort=updated&direction=desc&per_page=10",
        StatusCode::OK,
        &[("etag", "\"l1\"")],
        &format!(
            r#"[{{"number":7,"state":"closed","merged_at":"2026-01-03T00:00:00Z",
"updated_at":"2026-01-03T00:00:00Z","head":{{"sha":"{HEAD}"}}}}]"#
        ),
    );
    let branch = "topic".parse().unwrap();
    let Fresh::Changed(found) = github
        .find(&repo, &branch, Priority::Background)
        .await
        .unwrap()
    else {
        panic!("found");
    };
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].state, PullState::Merged);
    assert_eq!(
        github
            .find(&repo, &branch, Priority::Background)
            .await
            .unwrap(),
        Fresh::Same
    );
}

#[tokio::test]
async fn a_rate_limit_cools_the_host_down_before_the_next_request() {
    let (shared, github, repo) = stand().await;
    set(
        &shared,
        "GET /repos/owner/name/pulls/7",
        StatusCode::FORBIDDEN,
        &[("x-ratelimit-remaining", "0"), ("retry-after", "120")],
        r#"{"message":"API rate limit exceeded"}"#,
    );
    let number = PullNumber::new(7).unwrap();
    let head = HEAD.parse().unwrap();
    let first = github
        .changed(&repo, number, &head, Priority::Background)
        .await;
    assert!(
        matches!(first, Err(ForgeError::RateLimited { .. })),
        "{first:?}"
    );
    let asked = shared.lock().unwrap().asked.len();
    let second = github.changed(&repo, number, &head, Priority::Person).await;
    assert!(
        matches!(second, Err(ForgeError::RateLimited { .. })),
        "{second:?}"
    );
    assert_eq!(
        shared.lock().unwrap().asked.len(),
        asked,
        "nothing more was sent"
    );
}

#[tokio::test]
async fn refusals_are_told_apart() {
    let (shared, github, repo) = stand().await;
    set(
        &shared,
        "GET /repos/owner/name/pulls/7",
        StatusCode::FORBIDDEN,
        &[(
            "x-github-sso",
            "required; url=https://github.com/orgs/o/sso",
        )],
        r#"{"message":"Resource protected by organization SAML enforcement"}"#,
    );
    let number = PullNumber::new(7).unwrap();
    let head = HEAD.parse().unwrap();
    let refused = github.changed(&repo, number, &head, Priority::Person).await;
    assert!(
        matches!(
            refused,
            Err(ForgeError::Refused {
                reason: RefusedReason::Sso { url: Some(_) }
            })
        ),
        "{refused:?}"
    );
    set(
        &shared,
        "GET /repos/owner/name/pulls/7",
        StatusCode::NOT_FOUND,
        &[],
        "{}",
    );
    let hidden = github.changed(&repo, number, &head, Priority::Person).await;
    assert!(
        matches!(
            hidden,
            Err(ForgeError::Refused {
                reason: RefusedReason::NoAccess
            })
        ),
        "{hidden:?}"
    );
}

#[tokio::test]
async fn opening_tells_a_new_one_from_one_that_exists() {
    let (shared, github, repo) = stand().await;
    let opening = Opening {
        head: "topic".parse().unwrap(),
        base: "main".parse().unwrap(),
        title: "A change".parse().unwrap(),
        body: vessel_proto::forge::PullBody::empty(),
        draft: true,
    };
    set(
        &shared,
        "POST /repos/owner/name/pulls",
        StatusCode::CREATED,
        &[],
        r#"{"number":9}"#,
    );
    assert_eq!(
        github.open(&repo, &opening).await.unwrap(),
        Opened::Created(PullNumber::new(9).unwrap())
    );
    set(
        &shared,
        "POST /repos/owner/name/pulls",
        StatusCode::UNPROCESSABLE_ENTITY,
        &[],
        r#"{"message":"Validation Failed","errors":[{"message":"A pull request already exists for owner:topic."}]}"#,
    );
    assert_eq!(github.open(&repo, &opening).await.unwrap(), Opened::Exists);
}

#[tokio::test]
async fn a_merge_pins_its_head_and_says_when_it_moved() {
    let (shared, github, repo) = stand().await;
    let number = PullNumber::new(7).unwrap();
    let head = HEAD.parse().unwrap();
    set(
        &shared,
        "PUT /repos/owner/name/pulls/7/merge",
        StatusCode::CONFLICT,
        &[],
        r#"{"message":"Head branch was modified. Review and try the merge again."}"#,
    );
    assert_eq!(
        github
            .merge(&repo, number, MergeMethod::Squash, &head)
            .await
            .unwrap(),
        Merged::HeadMoved
    );
    set(
        &shared,
        "PUT /repos/owner/name/pulls/7/merge",
        StatusCode::METHOD_NOT_ALLOWED,
        &[],
        r#"{"message":"Merge commits are not allowed on this repository."}"#,
    );
    assert_eq!(
        github
            .merge(&repo, number, MergeMethod::Merge, &head)
            .await
            .unwrap(),
        Merged::MethodNotAllowed
    );
    set(
        &shared,
        "PUT /repos/owner/name/pulls/7/merge",
        StatusCode::OK,
        &[],
        &format!(r#"{{"merged":true,"sha":"{HEAD}"}}"#),
    );
    assert_eq!(
        github
            .merge(&repo, number, MergeMethod::Squash, &head)
            .await
            .unwrap(),
        Merged::Done(head.clone())
    );
}

#[tokio::test]
async fn a_token_never_goes_to_another_host() {
    let credentials = Credentials::new(
        EnvTokens {
            github: Token::new(TOKEN),
            enterprise: None,
        },
        None,
    );
    let other = "git.example.org".parse().unwrap();
    assert!(matches!(credentials.token(&other).await, Found::None(_)));
}
