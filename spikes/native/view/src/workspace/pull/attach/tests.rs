//! Paseo's attachment text, line for line.

use chrono::{Duration, TimeZone as _, Utc};
use vessel_proto::forge::{
    Annotation, AnnotationLevel, CheckOutput, CheckReport, CheckRun, CheckState, FailedJob,
    LogTail, PullBody, ReviewState, ThreadPlace, TimelineItem,
};

use super::*;
use crate::workspace::pull::timeline::{Entry, entries, shown};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 12, 0, 0).unwrap()
}

fn about() -> About<'static> {
    About {
        number: 42,
        title: "Fix login",
        url: "https://github.com/acme/widgets/pull/42",
        now: now(),
    }
}

fn place() -> ThreadPlace {
    ThreadPlace {
        thread: "PRRT_abc".parse().unwrap(),
        path: Some("src/foo.ts".parse().unwrap()),
        line: Some(14),
        start_line: Some(12),
        resolved: false,
        outdated: true,
    }
}

fn comment(id: &str, body: &str, hours: i64, thread: Option<ThreadPlace>) -> TimelineItem {
    TimelineItem::Comment {
        id: id.parse().unwrap(),
        author: Some("octo".parse().unwrap()),
        body: PullBody::lossy(body),
        created_at: now() - Duration::hours(hours),
        url: Some(format!("https://github.com/acme/widgets/pull/42#{id}").parse().unwrap()),
        review: None,
        thread,
    }
}

#[test]
fn a_comment_reads_as_paseo_s() {
    let made = activity(&about(), &comment("c1", " Why? ", 3, Some(place()))).unwrap();
    assert_eq!(made.kind, AttachKind::Comment);
    assert_eq!(made.id, "42:c1");
    assert_eq!(made.title, "octo");
    assert_eq!(
        made.text,
        "GitHub pull request comment\n\
         Pull request: #42 Fix login\n\
         Pull request URL: https://github.com/acme/widgets/pull/42\n\
         URL: https://github.com/acme/widgets/pull/42#c1\n\
         Author: octo\n\
         Created: 3h ago\n\
         Location: src/foo.ts:12-14 · unresolved · outdated · thread PRRT_abc\n\
         \n\
         Why?"
    );
    assert!(activity(&about(), &comment("c2", "  ", 0, None)).is_none());
}

#[test]
fn a_change_request_without_words_still_goes() {
    let review = TimelineItem::Review {
        id: "r1".parse().unwrap(),
        author: None,
        body: PullBody::lossy(""),
        created_at: now(),
        url: None,
        state: ReviewState::ChangesRequested,
    };
    let made = activity(&about(), &review).unwrap();
    assert_eq!(made.kind, AttachKind::Review);
    assert!(made.text.contains("\nState: changes_requested\n"), "{}", made.text);
    assert!(made.text.ends_with("Created: just now"), "no body, no blank line");
    assert_eq!(made.title, "unknown");
}

#[test]
fn a_thread_is_one_conversation() {
    let items = [
        comment("c1", "Why?", 2, Some(place())),
        comment("c2", "Because", 1, Some(place())),
    ];
    let listed = shown(&items);
    let made = entries(&listed);
    let Some(Entry::Thread(found)) = made.first() else {
        panic!("one thread: {made:?}");
    };
    let attached = thread(&about(), found).unwrap();
    assert_eq!(attached.id, "42:thread:PRRT_abc");
    assert_eq!(attached.title, "src/foo.ts:12-14");
    assert_eq!(
        attached.text,
        "GitHub pull request review thread\n\
         Pull request: #42 Fix login\n\
         Pull request URL: https://github.com/acme/widgets/pull/42\n\
         URL: https://github.com/acme/widgets/pull/42#c1\n\
         Location: src/foo.ts:12-14\n\
         Thread state: unresolved\n\
         Note: this thread is outdated (the code it refers to has changed)\n\
         \n\
         octo (2h ago):\nWhy?\n\n---\n\nocto (1h ago):\nBecause"
    );
}

#[test]
fn a_failed_check_carries_its_report_and_logs() {
    let mut run = CheckRun::named("build".parse().unwrap(), CheckState::Failed, None);
    run.run = Some(RunIdFor::id(7));
    let report = CheckReport {
        run: RunIdFor::id(7),
        name: "build".parse().unwrap(),
        state: CheckState::Failed,
        url: None,
        details_url: Some("https://github.com/acme/widgets/actions/runs/1".parse().unwrap()),
        output: Some(CheckOutput {
            title: Some(vessel_proto::forge::PullTitle::lossy("1 error")),
            summary: None,
            text: None,
        }),
        annotations: vec![Annotation {
            path: "src/lib.rs".parse().unwrap(),
            start_line: 3,
            end_line: 4,
            level: AnnotationLevel::Failure,
            message: PullBody::lossy("expected `;`"),
            title: None,
        }],
        failed_jobs: vec![FailedJob {
            job: RunIdFor::id(9),
            name: "test".parse().unwrap(),
            state: CheckState::Failed,
            url: None,
            completed_at: None,
            log: Some(LogTail {
                text: PullBody::lossy("error: one\nerror: two"),
                cut: true,
            }),
        }],
        truncated: false,
    };
    let made = check(&about(), &run, Some(&report));
    assert_eq!(made.id, "42:check-run:7");
    assert_eq!(
        made.url.as_deref(),
        Some("https://github.com/acme/widgets/actions/runs/1")
    );
    assert_eq!(
        made.text,
        "GitHub pull request check\n\
         Pull request: #42 Fix login\n\
         Pull request URL: https://github.com/acme/widgets/pull/42\n\
         Check: build\n\
         Status: failure\n\
         Conclusion: failure\n\
         Check URL: https://github.com/acme/widgets/pull/42\n\
         Details URL: https://github.com/acme/widgets/actions/runs/1\n\
         Output title: 1 error\n\
         \n\
         Annotations:\n\
         - src/lib.rs:3-4 failure: expected `;`\n\
         \n\
         Failed jobs:\n\
         - test: failure\n  ```\n  error: one\n  error: two\n  ```\n\
         \x20 Log tail truncated to the latest capped lines."
    );
    let bare = check(&about(), &run, None);
    assert!(bare.text.ends_with("Check URL: https://github.com/acme/widgets/pull/42"));
}

/// A run id for the tests.
struct RunIdFor;

impl RunIdFor {
    fn id(value: u64) -> vessel_proto::forge::RunId {
        vessel_proto::forge::RunId::new(value).unwrap()
    }
}
