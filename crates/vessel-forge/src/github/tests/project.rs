//! Actual provider requests against the loopback HTTP boundary. Fixtures
//! are external JSON, including malformed pages; no real forge is used.

use axum::http::StatusCode;
use vessel_proto::forge::{InvalidOpenPullPage, OpenPullPage, PullCursor};

use super::{
    READ,
    stand::{set, stand},
};
use crate::github::graphql;
use crate::provider::{Forge, ForgeError, Priority};

const NODE: &str = r#"{"number":7,"url":"https://github.com/owner/name/pull/7",
"title":"Fix\nit","state":"OPEN","isDraft":false,"updatedAt":"2026-01-02T00:00:00Z",
"headRefOid":"0123456789abcdef0123456789abcdef01234567","mergeable":"MERGEABLE",
"mergeStateStatus":"BLOCKED","reviewDecision":"CHANGES_REQUESTED","commits":{"nodes":[
{"commit":{"statusCheckRollup":{"contexts":{"totalCount":2,"nodes":[
{"__typename":"CheckRun","name":"test","status":"COMPLETED","conclusion":"FAILURE",
"detailsUrl":"http://ci.example/1","startedAt":"2026-01-02T00:00:00Z"},
{"__typename":"StatusContext","context":"lint","state":"SUCCESS","targetUrl":null,
"createdAt":"2026-01-02T00:00:00Z"}]}}}}]}}"#;

#[derive(Clone, Copy)]
enum Continuation<'a> {
    More(Option<&'a str>),
    Last(Option<&'a str>),
}

// The fixture owns GitHub's raw format: invalid encodings must reach the
// real decoder rather than being prevented by a domain-value constructor.
fn page(nodes: &[&str], continuation: Continuation<'_>) -> String {
    let (more, cursor) = match continuation {
        Continuation::More(cursor) => (true, cursor),
        Continuation::Last(cursor) => (false, cursor),
    };
    format!(
        r#"{{"data":{{"repository":{{"pullRequests":{{"nodes":[{}],"pageInfo":{{"hasNextPage":{more},"endCursor":{}}}}}}}}}}}"#,
        nodes.join(","),
        serde_json::to_string(&cursor).unwrap()
    )
}

#[tokio::test]
async fn a_project_page_uses_the_whole_reads_exact_facts_and_typed_http_variables() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            set(&shared, "POST /graphql", StatusCode::OK, &[], READ);
            let full = crate::deadline::step(
                "whole read",
                github.read(&repo, super::PullNumber::new(7).unwrap(), Priority::Person),
            )
            .await
            .unwrap();
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE], Continuation::Last(None)),
            );
            let listed = crate::deadline::step(
                "project page",
                github.list_open(&repo, None, Priority::Person),
            )
            .await
            .unwrap();
            assert_eq!(listed.pulls(), [full.pull.summary]);
            assert_eq!(listed.continuation(), None);
            let requests = &shared.lock().unwrap();
            assert!(requests.malformed_graphql.is_none());
            let whole = requests.graphql.first().unwrap();
            assert_eq!(whole.query, graphql::READ);
            assert_eq!(
                whole.variables.number,
                Some(super::PullNumber::new(7).unwrap())
            );
            assert_eq!(whole.variables.first, None);
            let request = requests.graphql.last().unwrap();
            assert_eq!(request.query, graphql::LIST);
            assert_eq!(request.variables.owner, "owner");
            assert_eq!(request.variables.name, "name");
            assert_eq!(request.variables.number, None);
            assert_eq!(request.variables.first, Some(OpenPullPage::MAX_PULLS));
            assert_eq!(request.variables.after, None);
            assert_eq!(request.variables.contexts, graphql::MAX_CONTEXTS.get());
            assert_eq!(request.variables.heads, graphql::HEAD_COMMITS.get());
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn a_continuation_is_a_variable_and_the_reply_keeps_its_repository_association() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            let before: PullCursor = "before+/=".parse().unwrap();
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE], Continuation::More(Some("after+/="))),
            );
            let listed = crate::deadline::step(
                "project page",
                github.list_open(&repo, Some(&before), Priority::Person),
            )
            .await
            .unwrap();
            let next = listed.continuation().unwrap();
            assert_eq!(next.repo, repo);
            assert_eq!(next.after.as_str(), "after+/=");
            assert_eq!(
                shared
                    .lock()
                    .unwrap()
                    .graphql
                    .last()
                    .unwrap()
                    .variables
                    .after
                    .as_ref(),
                Some(&before)
            );
        }),
    )
    .await
    .expect(
        "a_continuation_is_a_variable_and_the_reply_keeps_its_repository_association must finish",
    );
}

#[tokio::test]
async fn partial_graphql_errors_are_refused_and_rate_limits_remain_distinct() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            let partial = page(&[NODE], Continuation::Last(None));
            let body = format!(
                "{},\"errors\":[{{\"type\":\"INTERNAL\"}}]}}",
                partial.strip_suffix('}').unwrap()
            );
            set(&shared, "POST /graphql", StatusCode::OK, &[], &body);
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected {
                    what: "a complete project pull-request page"
                })
            ));
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                r#"{"errors":[{"type":"RATE_LIMITED"}]}"#,
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::RateLimited { .. })
            ));
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn page_count_duplicates_and_non_open_rows_fail_whole() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            let many = vec![NODE; OpenPullPage::MAX_PULLS.checked_add(1).unwrap()];
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&many, Continuation::Last(None)),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Page(InvalidOpenPullPage::TooMany))
            ));
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE, NODE], Continuation::Last(None)),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Page(InvalidOpenPullPage::Duplicate))
            ));
            let closed = NODE.replacen(r#""state":"OPEN""#, r#""state":"CLOSED""#, 1);
            assert_ne!(closed, NODE);
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE, &closed], Continuation::Last(None)),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Page(InvalidOpenPullPage::NotOpen))
            ));
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn empty_more_missing_and_stuck_cursors_are_not_successful_final_pages() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[], Continuation::Last(None)),
            );
            let empty = crate::deadline::step(
                "project page",
                github.list_open(&repo, None, Priority::Person),
            )
            .await
            .unwrap();
            assert!(empty.pulls().is_empty());
            assert_eq!(empty.continuation(), None);
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[], Continuation::More(Some("next"))),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Page(InvalidOpenPullPage::EmptyContinuation))
            ));
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE], Continuation::More(None)),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected {
                    what: "a project page's next cursor"
                })
            ));
            let after: PullCursor = "same".parse().unwrap();
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE], Continuation::More(Some("same"))),
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, Some(&after), Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected {
                    what: "a project page cursor that advances"
                })
            ));
            let unused = "x".repeat(PullCursor::MAX_BYTES.checked_add(1).unwrap());
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                &page(&[NODE], Continuation::Last(Some(&unused))),
            );
            assert_eq!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await
                .unwrap()
                .continuation(),
                None
            );
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn no_access_is_distinct_from_an_empty_success_and_unsigned_hosts_receive_no_request() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, mut github, repo) =
                crate::deadline::step("bind HTTP fixture", stand()).await;
            set(
                &shared,
                "POST /graphql",
                StatusCode::OK,
                &[],
                r#"{"data":{"repository":null}}"#,
            );
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::Refused {
                    reason: super::RefusedReason::NoAccess
                })
            ));
            let before = shared.lock().unwrap().asked.len();
            github.credentials =
                std::sync::Arc::new(crate::Credentials::new(crate::EnvTokens::default(), None));
            assert!(matches!(
                crate::deadline::step(
                    "project page",
                    github.list_open(&repo, None, Priority::Person)
                )
                .await,
                Err(ForgeError::SignedOut { .. })
            ));
            assert_eq!(shared.lock().unwrap().asked.len(), before);
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn factoring_summary_fields_preserves_whole_read_error_precedence() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            let bad_head = READ
                .replacen(
                    "0123456789abcdef0123456789abcdef01234567",
                    "not-a-commit",
                    1,
                )
                .replacen(r#""headRefName":"topic""#, r#""headRefName":"bad ref""#, 1);
            set(&shared, "POST /graphql", StatusCode::OK, &[], &bad_head);
            assert!(matches!(
                crate::deadline::step(
                    "whole read",
                    github.read(&repo, super::PullNumber::new(7).unwrap(), Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected {
                    what: "a head commit"
                })
            ));
            let bad_branch = READ
                .replacen(r#""headRefName":"topic""#, r#""headRefName":"bad ref""#, 1)
                .replacen(r#""number":7"#, r#""number":0"#, 1);
            assert_ne!(bad_branch, READ);
            set(&shared, "POST /graphql", StatusCode::OK, &[], &bad_branch);
            assert!(matches!(
                crate::deadline::step(
                    "whole read",
                    github.read(&repo, super::PullNumber::new(7).unwrap(), Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected {
                    what: "a head branch"
                })
            ));
            let bad_number = READ.replacen(r#""number":7"#, r#""number":0"#, 1);
            set(&shared, "POST /graphql", StatusCode::OK, &[], &bad_number);
            assert!(matches!(
                crate::deadline::step(
                    "whole read",
                    github.read(&repo, super::PullNumber::new(7).unwrap(), Priority::Person)
                )
                .await,
                Err(ForgeError::Unexpected { what: "a number" })
            ));
        }),
    )
    .await
    .expect("the forge fixture must finish");
}

#[tokio::test]
async fn known_repository_refusals_invalidate_page_associations_but_not_token_selection() {
    vessel_testkit::within(
        std::time::Duration::from_secs(10),
        Box::pin(async {
            let (shared, github, repo) = crate::deadline::step("bind HTTP fixture", stand()).await;
            for (status, body) in [
                (StatusCode::UNAUTHORIZED, "{}"),
                (StatusCode::FORBIDDEN, "{}"),
                (StatusCode::NOT_FOUND, "{}"),
                (StatusCode::OK, r#"{"data":{"repository":null}}"#),
            ] {
                let before = crate::deadline::step(
                    "credential association",
                    github.credentials.identity(&repo.host),
                )
                .await
                .unwrap();
                set(&shared, "POST /graphql", status, &[], body);
                assert!(matches!(
                    crate::deadline::step(
                        "project page",
                        github.list_open(&repo, None, Priority::Person)
                    )
                    .await,
                    Err(ForgeError::Refused { .. })
                ));
                assert_ne!(
                    crate::deadline::step(
                        "credential association",
                        github.credentials.identity(&repo.host)
                    )
                    .await
                    .unwrap(),
                    before
                );
                assert!(matches!(
                    crate::deadline::step(
                        "credential selection",
                        github.credentials.token(&repo.host)
                    )
                    .await,
                    crate::TokenFound::Token(_, vessel_proto::forge::TokenSource::Environment)
                ));
            }
            let before = crate::deadline::step(
                "credential association",
                github.credentials.identity(&repo.host),
            )
            .await
            .unwrap();
            set(&shared, "POST /graphql", StatusCode::NOT_FOUND, &[], "{}");
            assert!(matches!(
                crate::deadline::step(
                    "whole read",
                    github.read(&repo, super::PullNumber::new(7).unwrap(), Priority::Person)
                )
                .await,
                Err(ForgeError::Refused {
                    reason: super::RefusedReason::NoAccess
                })
            ));
            assert_ne!(
                crate::deadline::step(
                    "credential association",
                    github.credentials.identity(&repo.host)
                )
                .await
                .unwrap(),
                before
            );
        }),
    )
    .await
    .expect("the forge fixture must finish");
}
