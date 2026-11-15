//! Fixtures ported from Paseo's agent-attention-policy.test.ts; wire
//! confidentiality is checked by decryption, not by our own mock of crypto.
mod crypto_tests;
mod node_delivery;
use super::policy::{Plan, Presence, plan};
use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::SessionId;
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_776_600_000, 0).unwrap()
}
fn state(focus: PushFocus, millis_ago: Option<i64>) -> Presence {
    Presence {
        focus,
        active_at: millis_ago.map(|age| {
            now()
                .checked_sub_signed(TimeDelta::milliseconds(age))
                .unwrap()
        }),
    }
}
#[test]
fn stale_focused_clients_do_not_suppress_but_present_visible_targets_do() {
    let session = SessionId::random();
    let stale = state(PushFocus::Session { session }, Some(180_001));
    let recent = state(PushFocus::Session { session }, Some(179_999));
    assert_eq!(plan(&[stale], session, now()), Plan::Push);
    assert_eq!(plan(&[stale, recent], session, now()), Plan::Suppressed);
}
#[test]
fn background_or_another_session_selects_a_local_recipient_instead_of_suppressing() {
    let session = SessionId::random();
    for focus in [
        PushFocus::Background,
        PushFocus::Elsewhere,
        PushFocus::Session {
            session: SessionId::random(),
        },
    ] {
        assert_eq!(
            plan(&[state(focus, Some(1000))], session, now()),
            Plan::Local(0)
        );
    }
}
#[test]
fn most_recent_clamped_activity_wins_and_equal_times_keep_the_lower_index() {
    let session = SessionId::random();
    let stale = state(PushFocus::Elsewhere, Some(180_001));
    assert_eq!(
        plan(
            &[
                state(PushFocus::Elsewhere, Some(10_000)),
                state(PushFocus::Elsewhere, Some(1000)),
                stale
            ],
            session,
            now()
        ),
        Plan::Local(1)
    );
    assert_eq!(
        plan(
            &[
                state(PushFocus::Elsewhere, Some(1000)),
                state(PushFocus::Elsewhere, Some(1000))
            ],
            session,
            now()
        ),
        Plan::Local(0)
    );
    assert_eq!(
        plan(
            &[
                state(PushFocus::Elsewhere, Some(1)),
                state(PushFocus::Elsewhere, Some(-600_000))
            ],
            session,
            now()
        ),
        Plan::Local(1)
    );
    assert_eq!(
        plan(
            &[
                state(PushFocus::Elsewhere, Some(-600_000)),
                state(PushFocus::Elsewhere, Some(-700_000))
            ],
            session,
            now()
        ),
        Plan::Local(0)
    );
}
#[test]
fn sockets_without_activity_never_count_as_present_and_180_seconds_is_inclusive() {
    let session = SessionId::random();
    assert_eq!(
        plan(
            &[state(PushFocus::Session { session }, None)],
            session,
            now()
        ),
        Plan::Push
    );
    assert_eq!(
        plan(
            &[
                state(PushFocus::Elsewhere, Some(180_001)),
                state(PushFocus::Elsewhere, Some(180_001))
            ],
            session,
            now()
        ),
        Plan::Push
    );
    assert_eq!(
        plan(
            &[state(PushFocus::Elsewhere, Some(180_000))],
            session,
            now()
        ),
        Plan::Local(0)
    );
}
#[test]
fn focus_reports_do_not_renew_activity_and_client_times_cannot_be_supplied() {
    let mut presence = state(PushFocus::Background, Some(180_001));
    presence.report(
        ReportPushPresence {
            focus: PushFocus::Elsewhere,
            activity: PushActivity::Unchanged,
        },
        now(),
    );
    assert!(!presence.present(now()));
    presence.report(
        ReportPushPresence {
            focus: PushFocus::Elsewhere,
            activity: PushActivity::Active,
        },
        now(),
    );
    assert_eq!(presence.active_at, Some(now()));
    assert!(presence.present(now()));
}
mod candidates;
