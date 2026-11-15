//! The bridge's bounds (`keep/drafts_doc.rs`) on the app's own document.

use chrono::{TimeDelta, TimeZone as _};

use super::*;

fn at(day: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 1, 9, 0, 0)
        .single()
        .unwrap()
        .checked_add_signed(TimeDelta::days(day))
        .unwrap()
}

fn session() -> DraftKey {
    DraftKey::Session {
        session: SessionId::random(),
    }
}

#[test]
fn a_draft_comes_back_for_its_key_and_emptying_it_removes_it() {
    let mut drafts = Drafts::default();
    let one = session();
    assert_eq!(drafts.keep(one, "half a thought", at(0)), Kept::Kept);
    assert_eq!(drafts.keep(DraftKey::New, "새 채팅", at(0)), Kept::Kept);
    assert_eq!(drafts.text(one), "half a thought");
    assert_eq!(drafts.text(DraftKey::New), "새 채팅");
    assert_eq!(drafts.text(session()), "", "another session has none");
    assert_eq!(drafts.keep(one, "  \n", at(1)), Kept::Removed);
    assert_eq!(drafts.text(one), "");
    assert_eq!(drafts.len(), 1);
}

#[test]
fn a_draft_too_long_is_not_kept_and_its_older_text_goes() {
    let mut drafts = Drafts::default();
    let one = session();
    let _kept = drafts.keep(one, "short", at(0));
    let long = "a".repeat(Drafts::MAX_TEXT + 1);
    assert_eq!(drafts.keep(one, &long, at(0)), Kept::TooLong);
    assert_eq!(drafts.text(one), "", "not the stale shorter one");
    let exact = "a".repeat(Drafts::MAX_TEXT);
    assert_eq!(drafts.keep(one, &exact, at(0)), Kept::Kept);
}

#[test]
fn drafts_older_than_thirty_days_are_dropped() {
    let mut drafts = Drafts::default();
    let (old, recent) = (session(), session());
    let _old = drafts.keep(old, "old", at(0));
    let _recent = drafts.keep(recent, "recent", at(20));
    drafts.prune(at(Drafts::MAX_AGE_DAYS));
    assert_eq!(drafts.text(old), "old", "exactly thirty days is kept");
    drafts.prune(at(Drafts::MAX_AGE_DAYS + 1));
    assert_eq!(drafts.text(old), "");
    assert_eq!(drafts.text(recent), "recent");
}

#[test]
fn past_the_count_or_the_total_the_oldest_goes_first() {
    let mut drafts = Drafts::default();
    let first = session();
    let _first = drafts.keep(first, "first", at(0));
    for _ in 0..Drafts::MAX_DRAFTS {
        let _kept = drafts.keep(session(), "later", at(1));
    }
    assert_eq!(drafts.len(), Drafts::MAX_DRAFTS);
    assert_eq!(drafts.text(first), "", "the oldest went");

    let mut drafts = Drafts::default();
    let big = "b".repeat(Drafts::MAX_TEXT);
    let keys: Vec<DraftKey> = (0..9).map(|_| session()).collect();
    for (day, key) in (0_i64..).zip(&keys) {
        let _kept = drafts.keep(*key, &big, at(day));
    }
    assert_eq!(drafts.len(), 8, "eight of 128 KiB fill 1 MiB");
    assert_eq!(keys.first().map(|key| drafts.text(*key)), Some(""));
}

#[test]
fn the_document_round_trips_through_json() {
    let mut drafts = Drafts::default();
    let one = session();
    let _kept = drafts.keep(one, "kept", at(0));
    let _new = drafts.keep(DraftKey::New, "new", at(0));
    let text = serde_json::to_string(&drafts).unwrap();
    let back: Drafts = serde_json::from_str(&text).unwrap();
    assert_eq!(back, drafts);
}
