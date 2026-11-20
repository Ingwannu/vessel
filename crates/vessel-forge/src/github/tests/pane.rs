//! The pane's reads (ADR 0088) against recorded GitHub answers on
//! loopback: the timeline in Paseo's order with threads and truncation,
//! and a check run's output, annotations, failed jobs and log tails, a
//! log behind a redirect to another origin over `http` refused.

use axum::http::StatusCode;
use vessel_proto::forge::{
    AnnotationLevel, CheckState, PullNumber, ReviewState, RunId, TimelineItem,
};

use super::stand::{set, stand};
use crate::provider::Forge as _;

/// The timeline query's answer, trimmed to a few items of each kind.
const TIMELINE: &str = r#"{"data":{"repository":{"pullRequest":{
"reviews":{"nodes":[
{"id":"PRR_2","state":"CHANGES_REQUESTED","body":"Please fix","url":"https://github.com/owner/name/pull/7#pullrequestreview-2",
"submittedAt":"2026-01-03T00:00:00Z","author":{"login":"reviewer"}},
{"id":"PRR_9","state":"PENDING","body":"draft","url":null,"submittedAt":null,"author":{"login":"reviewer"}}],
"pageInfo":{"hasNextPage":false}},
"comments":{"nodes":[
{"id":"IC_1","body":"Looks close","url":"https://github.com/owner/name/pull/7#issuecomment-1",
"createdAt":"2026-01-02T00:00:00Z","author":{"login":"someone"}}],
"pageInfo":{"hasNextPage":true}},
"reviewThreads":{"nodes":[
{"id":"PRRT_1","path":"src/a.rs","line":12,"startLine":10,"isResolved":false,"isOutdated":true,
"comments":{"nodes":[{"id":"PRRC_1","body":"Why?","url":null,"createdAt":"2026-01-03T00:00:00Z",
"author":{"login":"reviewer"},"pullRequestReview":{"id":"PRR_2"}}],"pageInfo":{"hasNextPage":false}}}],
"pageInfo":{"hasNextPage":false}}}}}}"#;

#[tokio::test]
async fn the_timeline_is_in_time_then_id_order_with_threads_and_truncation() {
    let (shared, github, repo) = stand().await;
    set(&shared, "POST /graphql", StatusCode::OK, &[], TIMELINE);
    let timeline = github
        .timeline(&repo, PullNumber::new(7).unwrap())
        .await
        .unwrap();
    assert!(timeline.truncated, "a kind had a next page");
    let ids: Vec<_> = timeline
        .items
        .iter()
        .map(|item| item.id().as_str())
        .collect();
    assert_eq!(
        ids,
        ["IC_1", "PRRC_1", "PRR_2"],
        "a pending review is not activity"
    );
    let TimelineItem::Comment {
        review: Some(review),
        thread: Some(place),
        ..
    } = &timeline.items[1]
    else {
        panic!("a thread comment: {:?}", timeline.items[1]);
    };
    assert_eq!(review.as_str(), "PRR_2");
    assert_eq!(
        (place.line, place.start_line, place.resolved, place.outdated),
        (Some(12), Some(10), false, true)
    );
    assert!(matches!(
        timeline.items[2],
        TimelineItem::Review {
            state: ReviewState::ChangesRequested,
            ..
        }
    ));
}

const RUN: &str = r#"{"id":11,"name":"test","status":"completed","conclusion":"failure",
"html_url":"https://github.com/owner/name/runs/11","details_url":"https://ci.example/11",
"output":{"title":"2 failed","summary":"Two tests failed","text":null}}"#;

const NOTES: &str = r#"[{"path":"src/a.rs","start_line":3,"end_line":4,
"annotation_level":"failure","message":"assertion failed","title":"a test","raw_details":null}]"#;

const JOBS: &str = r#"{"total_count":3,"jobs":[
{"id":31,"name":"unit","status":"completed","conclusion":"failure","html_url":"https://github.com/owner/name/actions/runs/22/job/31","completed_at":"2026-01-02T00:05:00Z"},
{"id":32,"name":"lint","status":"completed","conclusion":"success","html_url":null,"completed_at":null},
{"id":33,"name":"e2e","status":"completed","conclusion":"timed_out","html_url":null,"completed_at":null}]}"#;

#[tokio::test]
async fn a_check_is_read_whole_and_a_log_behind_an_http_redirect_is_refused() {
    let (shared, github, repo) = stand().await;
    set(
        &shared,
        "GET /repos/owner/name/check-runs/11",
        StatusCode::OK,
        &[],
        RUN,
    );
    set(
        &shared,
        "GET /repos/owner/name/check-runs/11/annotations?per_page=20",
        StatusCode::OK,
        &[],
        NOTES,
    );
    set(
        &shared,
        "GET /repos/owner/name/actions/runs/22/jobs?per_page=100",
        StatusCode::OK,
        &[],
        JOBS,
    );
    // GitHub Enterprise Server may answer a job's log itself.
    set(
        &shared,
        "GET /repos/owner/name/actions/jobs/31/logs",
        StatusCode::OK,
        &[],
        "setup\nrunning\nFAILED: a test\n",
    );
    // A redirect to another origin over http: never followed.
    set(
        &shared,
        "GET /repos/owner/name/actions/jobs/33/logs",
        StatusCode::FOUND,
        &[("location", "http://localhost:9/signed-blob")],
        "",
    );
    let report = github
        .check(
            &repo,
            RunId::new(11).unwrap(),
            Some(RunId::new(22).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(report.state, CheckState::Failed);
    assert_eq!(
        report
            .output
            .as_ref()
            .and_then(|output| output.title.as_ref())
            .map(|title| title.as_str().to_owned()),
        Some("2 failed".to_owned())
    );
    assert_eq!(report.annotations.len(), 1);
    assert_eq!(report.annotations[0].level, AnnotationLevel::Failure);
    let jobs: Vec<_> = report
        .failed_jobs
        .iter()
        .map(|job| job.name.as_str())
        .collect();
    assert_eq!(jobs, ["unit", "e2e"], "failure and timed_out, not success");
    let unit = report.failed_jobs[0].log.as_ref().unwrap();
    assert_eq!(unit.text.as_str(), "setup\nrunning\nFAILED: a test");
    assert!(!unit.cut);
    assert!(
        report.failed_jobs[1].log.is_none(),
        "an http redirect is refused"
    );
    assert!(!report.truncated);
    let asked = &shared.lock().unwrap().asked;
    assert!(
        asked.iter().all(|(key, ..)| !key.contains("signed-blob")),
        "the redirect was not followed"
    );
}
