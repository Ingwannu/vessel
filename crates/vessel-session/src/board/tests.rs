//! Every case of agent-orchestrator's `backend/pkg/contract/kanban_test.go`,
//! ported one for one and named as there, then the cases of Vessel's own
//! differences (the module docs of `board`).
//!
//! Porting notes, so a case can be traced back:
//! - The reference's `ActivityBlocked` is `Activity::WaitingInput` here
//!   (difference 2); the cases that use it are kept, under their names.
//! - The reference's zero `KanbanSessionFacts{}` is [`session`]: an idle
//!   worker, last seen at the epoch, expecting no signal, no policy.
//! - A `nil` pull-request slice is `PullRequests::Seen(&[])`: the reference
//!   always sees pull requests. `PullRequests::Unseen` has cases of its
//!   own at the end.
//! - Times are seconds since the epoch, as `time.Unix` gives them there; a
//!   zero `UpdatedAt` is the epoch, which only ties with another zero.

use std::time::Duration;

use chrono::{DateTime, Utc};
use vessel_proto::{Activity, DisplayStatus, KanbanColumn};

use super::*;

/// The reference's `testGrace`.
const GRACE: Duration = Duration::from_secs(90);

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

/// The reference's zero `KanbanSessionFacts{}`.
fn session() -> SessionFacts {
    SessionFacts {
        worker: WorkerFacts {
            activity: Activity::Idle,
            last_activity_at: at(0),
            has_signal: false,
            signal_expected: false,
            is_terminated: false,
        },
        interrupted: false,
        has_artifact_output: false,
        policy: Policy::default(),
    }
}

/// The reference's `sessionAt`.
fn session_at(activity: Activity) -> SessionFacts {
    let mut session = session();
    session.worker.activity = activity;
    session
}

/// The reference's `interruptedAt`.
fn interrupted_at(activity: Activity) -> SessionFacts {
    let mut session = session_at(activity);
    session.interrupted = true;
    session
}

/// The reference's `silent` worker facts: expected to report, never did,
/// last seen at the epoch (an hour before the cases' `now`).
fn silent() -> SessionFacts {
    let mut session = session();
    session.worker.signal_expected = true;
    session.worker.has_signal = false;
    session.worker.last_activity_at = at(0);
    session
}

fn with(policy: Policy, mut session: SessionFacts) -> SessionFacts {
    session.policy = policy;
    session
}

const AUTO_REVIEW: Policy = Policy {
    auto_review: true,
    auto_inject_review: false,
    auto_inject_ci: false,
};

const INJECT_REVIEW: Policy = Policy {
    auto_review: false,
    auto_inject_review: true,
    auto_inject_ci: false,
};

const INJECT_CI: Policy = Policy {
    auto_review: false,
    auto_inject_review: false,
    auto_inject_ci: true,
};

fn pr(url: &str) -> PullRequestFacts {
    PullRequestFacts::new(PullRequestUrl::new(url), at(0))
}

const PRESENT: ReviewRunFacts = ReviewRunFacts {
    present: true,
    running: false,
    changes_requested: false,
    outcome: false,
    failed: false,
    cancelled: false,
};

/// Only the lane: the reference's `deriveColumn`, which needs neither the
/// clock nor the worker's activity.
fn column(session: &SessionFacts, prs: &[PullRequestFacts]) -> KanbanColumn {
    derive(session, PullRequests::Seen(prs), at(0), Duration::ZERO).0
}

/// The lane and the phrase as the reference's presentation cases derive
/// them: an hour after the epoch, with its grace.
fn presentation(session: &SessionFacts, prs: &[PullRequestFacts]) -> (KanbanColumn, DisplayStatus) {
    derive(session, PullRequests::Seen(prs), at(3600), GRACE)
}

struct ColumnCase {
    name: &'static str,
    session: SessionFacts,
    pr: PullRequestFacts,
    want: KanbanColumn,
}

struct PhraseCase {
    name: &'static str,
    session: SessionFacts,
    pr: PullRequestFacts,
    column: KanbanColumn,
    want: DisplayStatus,
}

#[test]
fn derive_kanban_column_session_level_rules() {
    let mut terminated = session();
    terminated.worker.is_terminated = true;
    let mut artifact = session();
    artifact.has_artifact_output = true;
    let cases: [(&str, SessionFacts, Vec<PullRequestFacts>, KanbanColumn); 3] = [
        (
            "terminated archives even with a live pr",
            terminated,
            vec![pr("pr/1")],
            KanbanColumn::Archive,
        ),
        (
            "no pr is still building",
            session(),
            Vec::new(),
            KanbanColumn::Building,
        ),
        (
            "artifact output without a pr is validating",
            artifact,
            Vec::new(),
            KanbanColumn::Validating,
        ),
    ];
    for (name, session, prs, want) in cases {
        assert_eq!(column(&session, &prs), want, "{name}");
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table, one row per reference case, kept whole so it reads \
              side by side with the reference's"
)]
fn derive_kanban_column_single_pr() {
    let cases = [
        ColumnCase {
            name: "draft is ao validation work",
            session: session(),
            pr: PullRequestFacts {
                draft: true,
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "merged is ready",
            session: session(),
            pr: PullRequestFacts {
                merged: true,
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "closed without merge is ready",
            session: session(),
            pr: PullRequestFacts {
                closed: true,
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "closed draft is still ready",
            session: session(),
            pr: PullRequestFacts {
                closed: true,
                draft: true,
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "mergeable is ready",
            session: session(),
            pr: PullRequestFacts {
                mergeability: Mergeability::Mergeable,
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "human approval is ready",
            session: session(),
            pr: PullRequestFacts {
                review: ReviewDecision::Approved,
                mergeability: Mergeability::Blocked,
                external_review: ExternalReviewFacts {
                    approved: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "ao's own approval alone is not ready",
            session: session(),
            pr: PullRequestFacts {
                review: ReviewDecision::Approved,
                mergeability: Mergeability::Blocked,
                review_run: PRESENT,
                ..pr("pr/1")
            },
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "review pass on the current head is validating",
            session: session(),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    running: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "ao addressing its own changes request is validating",
            session: with(INJECT_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "human changes request stays person-owned even with auto-inject on",
            session: with(INJECT_REVIEW, session()),
            pr: PullRequestFacts {
                review: ReviewDecision::ChangesRequested,
                external_review: ExternalReviewFacts {
                    changes_requested: true,
                    ..ExternalReviewFacts::default()
                },
                review_run: PRESENT,
                ..pr("pr/1")
            },
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "ao fixing ci is validating",
            session: with(INJECT_CI, session()),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                review_run: PRESENT,
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "failing ci with injection off hands the loop to a person",
            session: session(),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                review_run: PRESENT,
                ..pr("pr/1")
            },
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "auto review owns an unreviewed head",
            session: with(AUTO_REVIEW, session()),
            pr: pr("pr/1"),
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "auto review off hands an unreviewed head to a person",
            session: session(),
            pr: pr("pr/1"),
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "auto review hands the loop over once its pass approved this head",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    outcome: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "auto review keeps a changes-requested head, even without auto-inject",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    outcome: true,
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "auto-inject still keeps the loop moving on a changes-requested head",
            session: with(
                Policy {
                    auto_review: true,
                    auto_inject_review: true,
                    auto_inject_ci: false,
                },
                session(),
            ),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    outcome: true,
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "without auto review, a changes-requested pass hands off to a person",
            session: session(),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    outcome: true,
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::NeedsReview,
        },
        ColumnCase {
            name: "auto review keeps a mergeable pr validating until its pass approves",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                mergeability: Mergeability::Mergeable,
                review_run: ReviewRunFacts {
                    outcome: true,
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "auto review keeps a mergeable pr validating after its pass fails",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                mergeability: Mergeability::Mergeable,
                review_run: ReviewRunFacts {
                    failed: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "human approval releases a failed auto review",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review: ReviewDecision::Approved,
                external_review: ExternalReviewFacts {
                    approved: true,
                    ..ExternalReviewFacts::default()
                },
                review_run: ReviewRunFacts {
                    failed: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Ready,
        },
        ColumnCase {
            name: "auto review still owns a head whose pass failed",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    failed: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
        ColumnCase {
            name: "auto review still owns a head whose pass was cancelled",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    cancelled: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            want: KanbanColumn::Validating,
        },
    ];
    for case in cases {
        assert_eq!(
            column(&case.session, &[case.pr]),
            case.want,
            "{}",
            case.name
        );
    }
}

/// A run recorded for an earlier head is dropped before the reducer sees
/// it, so the pull request reads as an unreviewed head and the review loop
/// restarts: the fleet takes the next turn with auto review on, a person
/// with it off.
#[test]
fn derive_kanban_column_stale_review_run_starts_a_new_cycle() {
    let prs = [pr("pr/1")];
    assert_eq!(
        column(&with(AUTO_REVIEW, session()), &prs),
        KanbanColumn::Validating,
        "auto review on"
    );
    assert_eq!(
        column(&session(), &prs),
        KanbanColumn::NeedsReview,
        "auto review off"
    );
}

#[test]
fn derive_kanban_column_multiple_prs() {
    let older = at(1_767_225_600);
    let newer = at(1_767_229_200);

    // A merged pr never hides a live one.
    let prs = [
        PullRequestFacts {
            merged: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/merged"), newer)
        },
        PullRequestFacts::new(PullRequestUrl::new("pr/live"), older),
    ];
    assert_eq!(
        column(&with(AUTO_REVIEW, session()), &prs),
        KanbanColumn::Validating,
        "a merged pr never hides a live one"
    );

    let prs = [
        PullRequestFacts {
            merged: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/merged"), newer)
        },
        PullRequestFacts {
            closed: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/closed"), older)
        },
    ];
    assert_eq!(
        column(&session(), &prs),
        KanbanColumn::Ready,
        "terminal prs decide once nothing is live"
    );

    let prs = [
        PullRequestFacts {
            draft: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/validating"), newer)
        },
        PullRequestFacts::new(PullRequestUrl::new("pr/needs-review"), older),
    ];
    assert_eq!(
        column(&session(), &prs),
        KanbanColumn::NeedsReview,
        "the most actionable live pr wins"
    );

    // Ties break on the newest pr, then on the url, whatever the order.
    let b = PullRequestFacts {
        draft: true,
        ..PullRequestFacts::new(PullRequestUrl::new("pr/b"), older)
    };
    let a = PullRequestFacts {
        draft: true,
        ..PullRequestFacts::new(PullRequestUrl::new("pr/a"), older)
    };
    let first = [b.clone(), a.clone()];
    let second = [a, b];
    assert_eq!(column(&session(), &first), KanbanColumn::Validating);
    assert_eq!(column(&session(), &second), KanbanColumn::Validating);
    // Not in the reference, which can only see the column: the same pull
    // request wins both ways.
    let winner = |prs| choose(&session(), prs).map(|(_, pr)| pr.url.as_str().to_owned());
    assert_eq!(winner(&first), Some("pr/a".to_owned()));
    assert_eq!(winner(&second), Some("pr/a".to_owned()));
}

#[test]
fn derive_kanban_presentation_interrupted_artifact_session() {
    let mut session = interrupted_at(Activity::Idle);
    session.has_artifact_output = true;
    assert_eq!(
        presentation(&session, &[]),
        (KanbanColumn::Validating, DisplayStatus::Interrupted)
    );
}

#[test]
fn derive_kanban_presentation_building() {
    let cases = [
        (
            "active worker is working",
            session_at(Activity::Active),
            DisplayStatus::Working,
        ),
        (
            "blocked worker is blocked",
            session_at(Activity::WaitingInput),
            DisplayStatus::Blocked,
        ),
        (
            "a worker waiting on input is blocked",
            session_at(Activity::WaitingInput),
            DisplayStatus::Blocked,
        ),
        (
            "exited worker has exited",
            session_at(Activity::Exited),
            DisplayStatus::Exited,
        ),
        (
            "a silent worker past the grace period has no signal",
            silent(),
            DisplayStatus::NoSignal,
        ),
        (
            "an idle worker is awaiting its pr",
            session_at(Activity::Idle),
            DisplayStatus::AwaitingPr,
        ),
        (
            "an idle worker the person stopped is interrupted",
            interrupted_at(Activity::Idle),
            DisplayStatus::Interrupted,
        ),
        (
            "a new turn after an interrupt is working",
            interrupted_at(Activity::Active),
            DisplayStatus::Working,
        ),
        (
            "an interrupted worker that exited has exited",
            interrupted_at(Activity::Exited),
            DisplayStatus::Exited,
        ),
    ];
    for (name, session, want) in cases {
        assert_eq!(
            presentation(&session, &[]),
            (KanbanColumn::Building, want),
            "{name}"
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table, one row per reference case, kept whole so it reads \
              side by side with the reference's"
)]
fn derive_kanban_presentation_single_pr() {
    let cases = [
        // Validating: the fleet-driven loop.
        PhraseCase {
            name: "a draft with nothing else to say is a draft",
            session: session(),
            pr: PullRequestFacts {
                draft: true,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::Draft,
        },
        PhraseCase {
            name: "a blocked worker outranks the loop it was running",
            session: with(INJECT_CI, session_at(Activity::WaitingInput)),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::Blocked,
        },
        PhraseCase {
            name: "an exited worker outranks the loop it was running",
            session: with(INJECT_CI, session_at(Activity::Exited)),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::Exited,
        },
        PhraseCase {
            name: "failing ci with auto-fix on and worker active is being fixed",
            session: with(INJECT_CI, session_at(Activity::Active)),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::FixingCi,
        },
        PhraseCase {
            name: "failing ci with auto-fix on but worker idle says ci failing",
            session: with(INJECT_CI, session()),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::CiFailing,
        },
        PhraseCase {
            name: "failing ci on a draft with auto-fix off says ci failing",
            session: session(),
            pr: PullRequestFacts {
                draft: true,
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::CiFailing,
        },
        PhraseCase {
            name: "an ao changes request with auto-inject on and worker active is being addressed",
            session: with(INJECT_REVIEW, session_at(Activity::Active)),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::AddressingComments,
        },
        PhraseCase {
            name: "an ao changes request with auto-inject on but worker idle needs review",
            session: with(INJECT_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::NeedsReview,
        },
        PhraseCase {
            name: "a silent worker past grace has no signal",
            session: with(INJECT_CI, silent()),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::NoSignal,
        },
        PhraseCase {
            name: "an ao changes request with auto-inject off needs review",
            session: session(),
            pr: PullRequestFacts {
                draft: true,
                review_run: ReviewRunFacts {
                    changes_requested: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::NeedsReview,
        },
        PhraseCase {
            name: "auto review with no pass on this head has one scheduled",
            session: with(AUTO_REVIEW, session()),
            pr: pr("pr/1"),
            column: KanbanColumn::Validating,
            want: DisplayStatus::ReviewScheduled,
        },
        PhraseCase {
            name: "a pass in flight is reviewing",
            session: session(),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    running: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::Reviewing,
        },
        PhraseCase {
            name: "a failed pass remains validating and exposes the failure",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    failed: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::ReviewFailed,
        },
        PhraseCase {
            name: "a cancelled pass leaves the review pending",
            session: with(AUTO_REVIEW, session()),
            pr: PullRequestFacts {
                review_run: ReviewRunFacts {
                    cancelled: true,
                    ..PRESENT
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Validating,
            want: DisplayStatus::ReviewPending,
        },
        // In review: the loop seen from the person's side.
        PhraseCase {
            name: "an open pr nobody is handling needs a human review",
            session: session(),
            pr: pr("pr/1"),
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::NeedsHumanReview,
        },
        PhraseCase {
            name: "a blocked worker outranks an open review",
            session: session_at(Activity::WaitingInput),
            pr: pr("pr/1"),
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::Blocked,
        },
        PhraseCase {
            name: "an exited worker outranks an open review",
            session: session_at(Activity::Exited),
            pr: pr("pr/1"),
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::Exited,
        },
        PhraseCase {
            name: "a silent worker past grace has no signal while in review",
            session: silent(),
            pr: pr("pr/1"),
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::NoSignal,
        },
        PhraseCase {
            name: "failing ci nobody is fixing says ci failing",
            session: session(),
            pr: PullRequestFacts {
                ci: CiState::Failing,
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::CiFailing,
        },
        PhraseCase {
            name: "external comments with auto-inject off are commented",
            session: session(),
            pr: PullRequestFacts {
                external_review: ExternalReviewFacts {
                    comments: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::Commented,
        },
        PhraseCase {
            name: "external comments with auto-inject on and worker active are being addressed",
            session: with(INJECT_REVIEW, session_at(Activity::Active)),
            pr: PullRequestFacts {
                external_review: ExternalReviewFacts {
                    comments: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::AddressingComments,
        },
        PhraseCase {
            name: "external comments with auto-inject on but worker idle are commented",
            session: with(INJECT_REVIEW, session()),
            pr: PullRequestFacts {
                external_review: ExternalReviewFacts {
                    comments: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::Commented,
        },
        PhraseCase {
            name: "an external changes request nobody is addressing requests changes",
            session: session(),
            pr: PullRequestFacts {
                review: ReviewDecision::ChangesRequested,
                external_review: ExternalReviewFacts {
                    changes_requested: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::ChangesRequested,
        },
        PhraseCase {
            name: "an external changes request with auto-inject on and worker active stays in review while being addressed",
            session: with(INJECT_REVIEW, session_at(Activity::Active)),
            pr: PullRequestFacts {
                review: ReviewDecision::ChangesRequested,
                external_review: ExternalReviewFacts {
                    changes_requested: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::AddressingComments,
        },
        PhraseCase {
            name: "an external changes request with auto-inject on but worker idle requests changes",
            session: with(INJECT_REVIEW, session()),
            pr: PullRequestFacts {
                review: ReviewDecision::ChangesRequested,
                external_review: ExternalReviewFacts {
                    changes_requested: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::NeedsReview,
            want: DisplayStatus::ChangesRequested,
        },
        // Ready: how the pr landed, or what stands between it and the
        // button.
        PhraseCase {
            name: "a mergeable pr is mergeable",
            session: session(),
            pr: PullRequestFacts {
                mergeability: Mergeability::Mergeable,
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::Mergeable,
        },
        PhraseCase {
            name: "an approved pr that is not mergeable is approved",
            session: session(),
            pr: PullRequestFacts {
                review: ReviewDecision::Approved,
                mergeability: Mergeability::Blocked,
                external_review: ExternalReviewFacts {
                    approved: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::Approved,
        },
        PhraseCase {
            name: "an approved pr blocked by checks says ci failing",
            session: session(),
            pr: PullRequestFacts {
                review: ReviewDecision::Approved,
                mergeability: Mergeability::Blocked,
                ci: CiState::Failing,
                external_review: ExternalReviewFacts {
                    approved: true,
                    ..ExternalReviewFacts::default()
                },
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::CiFailing,
        },
        PhraseCase {
            name: "a merged pr is merged",
            session: session(),
            pr: PullRequestFacts {
                merged: true,
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::Merged,
        },
        PhraseCase {
            name: "a closed pr closed without merging",
            session: session(),
            pr: PullRequestFacts {
                closed: true,
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::Closed,
        },
        PhraseCase {
            name: "a merged pr reports the merge, not its stale merge readiness",
            session: session(),
            pr: PullRequestFacts {
                merged: true,
                mergeability: Mergeability::Mergeable,
                ..pr("pr/1")
            },
            column: KanbanColumn::Ready,
            want: DisplayStatus::Merged,
        },
    ];
    for case in cases {
        assert_eq!(
            presentation(&case.session, &[case.pr]),
            (case.column, case.want),
            "{}",
            case.name
        );
    }
}

#[test]
fn derive_kanban_presentation_terminated_archives() {
    let mut session = session_at(Activity::Active);
    session.worker.is_terminated = true;
    let prs = [PullRequestFacts {
        merged: true,
        ..pr("pr/1")
    }];
    assert_eq!(
        presentation(&session, &prs),
        (KanbanColumn::Archive, DisplayStatus::Terminated)
    );
}

/// A pass recorded for an earlier commit is filtered out before the
/// reducer sees it, so the head it left behind reads as unreviewed rather
/// than as a finished review.
#[test]
fn derive_kanban_presentation_ignores_a_pass_for_an_earlier_head() {
    let (_, phrase) = presentation(&with(AUTO_REVIEW, session()), &[pr("pr/1")]);
    assert_eq!(phrase, DisplayStatus::ReviewScheduled);
}

/// The phrase must describe the pull request the lane was chosen from, so
/// a landed one cannot speak for a session whose other one still needs
/// work.
#[test]
fn derive_kanban_presentation_speaks_for_the_chosen_pr() {
    let older = at(100);
    let newer = at(200);

    let prs = [
        PullRequestFacts {
            merged: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/1"), newer)
        },
        PullRequestFacts {
            ci: CiState::Failing,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/2"), older)
        },
    ];
    assert_eq!(
        presentation(&with(INJECT_CI, session_at(Activity::Active)), &prs),
        (KanbanColumn::Validating, DisplayStatus::FixingCi),
        "a merged pr does not hide live work"
    );

    let prs = [
        PullRequestFacts {
            closed: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/1"), older)
        },
        PullRequestFacts {
            merged: true,
            ..PullRequestFacts::new(PullRequestUrl::new("pr/2"), newer)
        },
    ];
    assert_eq!(
        presentation(&session(), &prs),
        (KanbanColumn::Ready, DisplayStatus::Merged),
        "only terminal prs report the best landing"
    );

    let (column, phrase) = presentation(&session_at(Activity::Active), &[pr("pr/1")]);
    assert!(
        column != KanbanColumn::Building && phrase != DisplayStatus::Working,
        "a pr keeps the card out of building whatever the worker is doing: {column:?} {phrase:?}"
    );
}

// Vessel's own cases.

/// Difference 1: a node that cannot see pull requests never says a session
/// awaits one; everything else reads as with none seen.
#[test]
fn a_node_that_cannot_see_pull_requests_says_idle() {
    let unseen = |session: &SessionFacts| derive(session, PullRequests::Unseen, at(3600), GRACE);
    let mut artifact = session();
    artifact.has_artifact_output = true;
    let mut ended = session_at(Activity::Exited);
    ended.worker.is_terminated = true;
    let cases = [
        (
            "idle",
            session(),
            KanbanColumn::Building,
            DisplayStatus::Idle,
        ),
        (
            "working",
            session_at(Activity::Active),
            KanbanColumn::Building,
            DisplayStatus::Working,
        ),
        (
            "waiting on its owner",
            session_at(Activity::WaitingInput),
            KanbanColumn::Building,
            DisplayStatus::Blocked,
        ),
        (
            "stopped mid-turn",
            interrupted_at(Activity::Idle),
            KanbanColumn::Building,
            DisplayStatus::Interrupted,
        ),
        (
            "silent past the grace",
            silent(),
            KanbanColumn::Building,
            DisplayStatus::NoSignal,
        ),
        (
            "with an artifact",
            artifact,
            KanbanColumn::Validating,
            DisplayStatus::NeedsReview,
        ),
        (
            "ended",
            ended,
            KanbanColumn::Archive,
            DisplayStatus::Terminated,
        ),
    ];
    for (name, session, column, phrase) in cases {
        assert_eq!(unseen(&session), (column, phrase), "{name}");
    }
}

/// The silence rule as the reference has it: strictly past the grace, and
/// never for a clock that went backwards.
#[test]
fn silence_starts_strictly_after_the_grace() {
    let phrase = |now: i64| derive(&silent(), PullRequests::Seen(&[]), at(now), GRACE).1;
    assert_eq!(phrase(90), DisplayStatus::AwaitingPr, "at the grace");
    assert_eq!(phrase(91), DisplayStatus::NoSignal, "past it");
    assert_eq!(phrase(-10), DisplayStatus::AwaitingPr, "clock went back");
    let mut reported = silent();
    reported.worker.has_signal = true;
    assert_eq!(
        derive(&reported, PullRequests::Seen(&[]), at(3600), GRACE).1,
        DisplayStatus::AwaitingPr,
        "a worker that reported once is not silent"
    );
}

/// The whole ranking, ready then in review then validating, whatever the
/// order and however recent. The reference's cases rank in review against
/// validating only, so a ready pull request that lost to a live one in
/// another lane would pass them all.
#[test]
fn ready_outranks_in_review_outranks_validating() {
    let ready = PullRequestFacts {
        mergeability: Mergeability::Mergeable,
        ..PullRequestFacts::new(PullRequestUrl::new("pr/ready"), at(100))
    };
    let in_review = PullRequestFacts::new(PullRequestUrl::new("pr/review"), at(200));
    let validating = PullRequestFacts {
        draft: true,
        ..PullRequestFacts::new(PullRequestUrl::new("pr/draft"), at(300))
    };
    let orders = [
        [ready.clone(), in_review.clone(), validating.clone()],
        [validating.clone(), in_review.clone(), ready.clone()],
        [in_review.clone(), validating.clone(), ready],
    ];
    for prs in &orders {
        assert_eq!(
            presentation(&session(), prs),
            (KanbanColumn::Ready, DisplayStatus::Mergeable),
            "{prs:?}"
        );
    }
    for prs in [
        [in_review.clone(), validating.clone()],
        [validating, in_review],
    ] {
        assert_eq!(
            presentation(&session(), &prs),
            (KanbanColumn::NeedsReview, DisplayStatus::NeedsHumanReview)
        );
    }
}
