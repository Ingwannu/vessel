//! Paseo's activity steps: empty items dropped, threads gathered where
//! they start and moved under their review, folded resolved and outdated
//! threads, the header's counts and the ages.

use chrono::{Duration, TimeZone as _, Utc};
use vessel_proto::forge::{PullBody, ReviewState, ThreadPlace, TimelineItem};

use super::*;

fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0).unwrap() + Duration::minutes(minute)
}

fn review(id: &str, state: ReviewState, body: &str, minute: i64) -> TimelineItem {
    TimelineItem::Review {
        id: id.parse().unwrap(),
        author: Some("reviewer".parse().unwrap()),
        body: PullBody::lossy(body),
        created_at: at(minute),
        url: None,
        state,
    }
}

fn place(thread: &str, resolved: bool, outdated: bool) -> ThreadPlace {
    ThreadPlace {
        thread: thread.parse().unwrap(),
        path: Some("src/lib.rs".parse().unwrap()),
        line: Some(14),
        start_line: Some(12),
        resolved,
        outdated,
    }
}

fn comment(
    id: &str,
    body: &str,
    minute: i64,
    review: Option<&str>,
    thread: Option<ThreadPlace>,
) -> TimelineItem {
    TimelineItem::Comment {
        id: id.parse().unwrap(),
        author: Some("author".parse().unwrap()),
        body: PullBody::lossy(body),
        created_at: at(minute),
        url: None,
        review: review.map(|id| id.parse().unwrap()),
        thread,
    }
}

#[test]
fn empty_items_say_nothing() {
    let items = [
        comment("c1", "  ", 0, None, None),
        review("r1", ReviewState::Commented, "", 1),
        review("r2", ReviewState::Approved, "", 2),
        review("r3", ReviewState::Commented, "Looks fine", 3),
    ];
    let kept: Vec<&str> = shown(&items).iter().map(|item| item.id().as_str()).collect();
    assert_eq!(kept, ["r2", "r3"], "an approval without words still counts");
}

#[test]
fn threads_gather_where_they_start_and_move_under_their_review() {
    let open = place("t1", false, false);
    let loose = place("t2", true, false);
    let items = [
        comment("c0", "First", 0, None, None),
        review("r1", ReviewState::ChangesRequested, "Please fix", 1),
        comment("c1", "Here", 2, Some("r1"), Some(open.clone())),
        comment("c2", "Elsewhere", 3, Some("gone"), Some(loose.clone())),
        comment("c3", "Done", 4, None, Some(open)),
        comment("c4", "Reply", 5, None, Some(loose)),
    ];
    let listed = shown(&items);
    let made = entries(&listed);
    let keys: Vec<String> = made.iter().map(Entry::key).collect();
    assert_eq!(keys, ["c0", "r1", "thread:t2"]);
    let Some(Entry::Review { threads, .. }) = made.get(1) else {
        panic!("the review holds its thread: {made:?}");
    };
    let replies: Vec<&str> = threads
        .first()
        .map(|thread| thread.comments.iter().map(|item| item.id().as_str()).collect())
        .unwrap_or_default();
    assert_eq!(replies, ["c1", "c3"], "a reply joins its thread");
    assert!(made.get(2).is_some_and(Entry::folded_at_first), "resolved starts folded");
    assert!(!made.first().is_some_and(Entry::folded_at_first));
    assert!(!made.get(1).is_some_and(Entry::folded_at_first), "a review never");
}

#[test]
fn a_review_without_threads_stays_a_single_card() {
    let items = [review("r1", ReviewState::Approved, "", 0)];
    let listed = shown(&items);
    assert!(matches!(entries(&listed).first(), Some(Entry::Single(_))));
}

#[test]
fn counts_are_of_the_flat_list() {
    let thread = place("t1", false, true);
    let items = [
        review("r1", ReviewState::Approved, "", 0),
        review("r2", ReviewState::ChangesRequested, "Fix", 1),
        review("r3", ReviewState::Commented, "Hm", 2),
        comment("c1", "Why?", 3, Some("r2"), Some(thread.clone())),
        comment("c2", "Because", 4, None, Some(thread)),
        comment("c3", "Thanks", 5, None, None),
    ];
    assert_eq!(
        counts(&shown(&items)),
        Counts {
            approvals: 1,
            changes: 1,
            comments: 4,
        }
    );
}

#[test]
fn ages_step_as_paseo_s() {
    let now = at(0);
    let ago = |minutes: i64| Age::of(now - Duration::minutes(minutes), now);
    assert_eq!(Age::of(now - Duration::seconds(59), now), Age::JustNow);
    assert_eq!(Age::of(now + Duration::minutes(5), now), Age::JustNow);
    assert_eq!(ago(59), Age::Minutes(59));
    assert_eq!(ago(60), Age::Hours(1));
    assert_eq!(ago(24 * 60), Age::Days(1));
    assert_eq!(ago(29 * 24 * 60), Age::Days(29));
    assert_eq!(ago(65 * 24 * 60), Age::Months(2));
    assert_eq!(ago(400 * 24 * 60), Age::Years(1));
}

#[test]
fn a_thread_names_its_lines() {
    let mut where_ = place("t1", false, false);
    assert_eq!(thread_path(&where_).as_deref(), Some("src/lib.rs:12-14"));
    where_.start_line = None;
    assert_eq!(thread_path(&where_).as_deref(), Some("src/lib.rs:14"));
    where_.line = None;
    assert_eq!(thread_path(&where_).as_deref(), Some("src/lib.rs"));
    where_.path = None;
    assert_eq!(thread_path(&where_), None, "a discussion thread");
}
