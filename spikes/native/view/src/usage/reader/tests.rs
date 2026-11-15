//! A deterministic in-memory node boundary drives the real lifetime model.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Account {
    First,
    Second,
    Third,
}
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    at: u32,
    used: u32,
}
type UsageReader = Reader<Account, Observation>;
fn value(at: u32) -> Observation {
    Observation { at, used: 42 }
}

#[test]
fn a_to_b_to_a_cannot_accept_the_first_a_response() {
    let mut reader = UsageReader::default();
    let first = reader.request(Account::First).unwrap();
    assert!(reader.request(Account::Second).is_none());
    assert!(reader.request(Account::First).is_none());
    let newest = reader.answer(first.ticket, Ok(value(1))).unwrap();
    assert_eq!(newest.selection, Account::First);
    assert_ne!(first.ticket, newest.ticket);
    assert!(reader.phase().observation().is_none());
    assert!(reader.answer(first.ticket, Ok(value(2))).is_none());
    assert!(reader.answer(newest.ticket, Ok(value(3))).is_none());
    assert_eq!(reader.phase().observation().unwrap().at, 3);
}

#[test]
fn only_the_latest_queued_choice_reaches_the_node() {
    let mut reader = UsageReader::default();
    let first = reader.request(Account::First).unwrap();
    assert!(reader.request(Account::Second).is_none());
    assert!(reader.request(Account::Third).is_none());
    let next = reader.answer(first.ticket, Ok(value(1))).unwrap();
    assert_eq!(next.selection, Account::Third);
    assert!(reader.phase().observation().is_none());
}

#[test]
fn failed_refresh_retains_original_success_and_its_timestamp() {
    let mut reader = UsageReader::default();
    let first = reader.request(Account::First).unwrap();
    assert!(reader.answer(first.ticket, Ok(value(7))).is_none());
    let refresh = reader.request(Account::First).unwrap();
    assert!(matches!(
        reader.phase(),
        Phase::Loading { retained: Some(_) }
    ));
    assert!(
        reader
            .answer(refresh.ticket, Err(RequestFailure::TimedOut))
            .is_none()
    );
    assert!(matches!(
        reader.phase(),
        Phase::Failed {
            reason: RequestFailure::TimedOut,
            ..
        }
    ));
    assert_eq!(reader.phase().observation().unwrap().at, 7);
    let _second = reader.request(Account::Second).unwrap();
    assert!(reader.phase().observation().is_none());
}

#[test]
fn closing_and_reopening_discards_the_old_lifetimes_result() {
    let mut reader = UsageReader::default();
    let old = reader.request(Account::First).unwrap();
    reader.close();
    assert!(matches!(reader.phase(), Phase::Idle));
    assert!(reader.request(Account::First).is_none());
    let new = reader.answer(old.ticket, Ok(value(1))).unwrap();
    assert!(reader.phase().observation().is_none());
    assert!(reader.answer(new.ticket, Ok(value(2))).is_none());
    assert_eq!(reader.phase().observation().unwrap().at, 2);
}

#[test]
fn a_closed_page_admits_no_queued_reads() {
    let mut reader = UsageReader::default();
    let old = reader.request(Account::First).unwrap();
    assert!(reader.request(Account::Second).is_none());
    reader.close();
    assert!(reader.answer(old.ticket, Ok(value(1))).is_none());
    assert!(matches!(reader.phase(), Phase::Idle));
}

#[test]
fn render_edits_follow_the_observation_identity_not_only_its_connection() {
    let mut reader = UsageReader::default();
    let first = reader.request(Account::First).unwrap();
    assert!(reader.observation_ticket().is_none());
    assert!(reader.answer(first.ticket, Ok(value(1))).is_none());
    assert_eq!(reader.observation_ticket(), Some(first.ticket));
    let failed = reader.request(Account::First).unwrap();
    assert_eq!(reader.observation_ticket(), Some(first.ticket));
    assert!(
        reader
            .answer(failed.ticket, Err(RequestFailure::TimedOut))
            .is_none()
    );
    assert_eq!(reader.observation_ticket(), Some(first.ticket));
    let replacement = reader.request(Account::First).unwrap();
    assert!(reader.answer(replacement.ticket, Ok(value(2))).is_none());
    assert_eq!(reader.observation_ticket(), Some(replacement.ticket));
    assert_ne!(reader.observation_ticket(), Some(first.ticket));
    reader.close();
    assert!(reader.observation_ticket().is_none());
    let reopened = reader.request(Account::First).unwrap();
    assert!(reader.observation_ticket().is_none());
    assert!(reader.answer(reopened.ticket, Ok(value(3))).is_none());
    assert_ne!(reader.observation_ticket(), Some(replacement.ticket));
}
