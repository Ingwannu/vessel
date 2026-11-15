use super::*;
use crate::schedules::page::tests::{loaded, rebound};
use vessel_proto::automation::{RunOutcome, RunTrigger};
pub(super) fn run(number: u32, outcome: RunOutcome) -> ScheduleRun {
    let at = "2026-11-15T00:00:00Z".parse().unwrap();
    ScheduleRun {
        number: RunNumber::new(number).unwrap(),
        at,
        due_at: at,
        trigger: RunTrigger::Due,
        outcome,
    }
}
pub(super) fn list() -> RunList {
    RunList {
        runs: vec![run(2, RunOutcome::Interrupted), run(1, RunOutcome::Making)],
    }
}
#[test]
fn every_body_and_explicit_retry_keep_original_schedule_and_context() {
    let (page, key) = loaded();
    let mut reader = Reader::default();
    assert_eq!(reader.body(), None);
    let first = reader.open(&page.row(&key).unwrap()).unwrap();
    let visit = reader.visit().unwrap();
    assert_eq!(first.input().schedule, key.schedule);
    assert_eq!(reader.schedule(), Some(key.schedule));
    assert_eq!(reader.body(), Some(Body::Loading));
    assert!(reader.retry(visit).is_none());
    assert!(reader.settle(&first, Err(Failure::Offline)));
    assert_eq!(reader.body(), Some(Body::Failed));
    assert_eq!(reader.failure(), Some(&Failure::Offline));
    let retry = reader.retry(visit).unwrap();
    assert_eq!(retry.input(), first.input());
    assert_eq!(retry.connection(), first.connection());
    assert_ne!(retry.ticket(), first.ticket());
    assert!(!reader.settle(&first, Ok(list())));
    assert!(reader.settle(&retry, Ok(RunList { runs: vec![] })));
    assert_eq!(reader.body(), Some(Body::Empty));
    let refresh = reader.retry(visit).unwrap();
    assert!(reader.settle(&refresh, Ok(list())));
    assert_eq!(reader.body(), Some(Body::Content));
    assert_eq!(reader.runs(), list().runs);
    assert_eq!(reader.visit(), Some(visit));
    assert!(reader.close(visit));
    assert_eq!(reader.body(), None);
    assert!(!reader.settle(&refresh, Ok(list())));
}
#[test]
fn another_visit_cannot_be_closed_or_populated_by_a_retained_callback() {
    let (page, key) = loaded();
    let mut reader = Reader::default();
    let first = reader.open(&page.row(&key).unwrap()).unwrap();
    let old = reader.visit().unwrap();
    let next = reader.open(&page.row(&key).unwrap()).unwrap();
    let current = reader.visit().unwrap();
    assert_ne!(current, old);
    assert!(!reader.close(old));
    assert!(reader.retry(old).is_none());
    assert!(!reader.settle(&first, Ok(list())));
    let wrong = Request {
        connection: rebound(&key.machine),
        ..next.clone()
    };
    assert!(!reader.settle(&wrong, Ok(list())));
    let wrong = Request {
        input: ListRuns {
            schedule: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        },
        ..next.clone()
    };
    assert!(!reader.settle(&wrong, Ok(list())));
    assert!(reader.settle(&next, Ok(list())));
}
#[test]
fn malformed_lists_do_not_become_partial_or_reordered_success() {
    let (page, key) = loaded();
    let mut oversized = Vec::new();
    for number in (1..=RunList::MAX.saturating_add(1)).rev() {
        oversized.push(run(u32::try_from(number).unwrap(), RunOutcome::Unknown));
    }
    for runs in [
        vec![run(1, RunOutcome::Making), run(2, RunOutcome::Making)],
        vec![run(1, RunOutcome::Making), run(1, RunOutcome::Making)],
        oversized,
    ] {
        let mut reader = Reader::default();
        let request = reader.open(&page.row(&key).unwrap()).unwrap();
        assert!(reader.settle(&request, Ok(RunList { runs })));
        assert_eq!(reader.failure(), Some(&Failure::Unexpected));
        assert!(reader.runs().is_empty());
        assert_eq!(reader.body(), Some(Body::Failed));
    }
    let runs = (1..=RunList::MAX)
        .rev()
        .map(|number| run(u32::try_from(number).unwrap(), RunOutcome::Unknown))
        .collect();
    let mut reader = Reader::default();
    let request = reader.open(&page.row(&key).unwrap()).unwrap();
    assert!(reader.settle(&request, Ok(RunList { runs })));
    assert_eq!(reader.runs().len(), RunList::MAX);
}
#[test]
fn same_node_reconnect_removes_rows_links_and_late_reply_authority() {
    let (page, key) = loaded();
    let mut reader = Reader::default();
    let request = reader.open(&page.row(&key).unwrap()).unwrap();
    let visit = reader.visit().unwrap();
    let session = "00000000-0000-4000-8000-000000000088".parse().unwrap();
    let runs = RunList {
        runs: vec![run(1, RunOutcome::Started { session })],
    };
    assert!(reader.settle(&request, Ok(runs.clone())));
    let link = reader.session_link(visit, RunNumber::FIRST).unwrap();
    assert_eq!(link.session(), session);
    assert_eq!(link.connection(), request.connection());
    assert!(!reader.connected(&MachineKey::of("other").unwrap()));
    assert!(reader.connected(&key.machine));
    assert!(!reader.connected(&key.machine));
    assert!(reader.runs().is_empty());
    assert!(reader.session_link(visit, RunNumber::FIRST).is_none());
    assert!(reader.retry(visit).is_none());
    assert!(!reader.settle(&request, Ok(runs)));
    assert!(!reader.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    assert_eq!(reader.failure(), Some(&Failure::ConnectionChanged));
}
#[test]
fn failed_creation_links_only_when_the_node_actually_names_a_session() {
    let (page, key) = loaded();
    let session = "00000000-0000-4000-8000-000000000088".parse().unwrap();
    for named in [None, Some(session)] {
        let mut reader = Reader::default();
        let request = reader.open(&page.row(&key).unwrap()).unwrap();
        let error = vessel_proto::automation::RunError::from_wire(&vessel_proto::WireError::new(
            vessel_proto::ErrorCode::SpawnFailed,
            "typed reason",
        ));
        assert!(reader.settle(
            &request,
            Ok(RunList {
                runs: vec![run(
                    1,
                    RunOutcome::Failed {
                        error,
                        session: named
                    }
                )]
            })
        ));
        assert_eq!(
            reader
                .session_link(reader.visit().unwrap(), RunNumber::FIRST)
                .map(|link| link.session()),
            named
        );
    }
}
#[test]
fn read_only_and_ended_rows_remain_readable_but_offline_rows_cannot_open() {
    let (mut page, key) = loaded();
    page.refresh();
    let request = page.due(crate::schedules::page::tests::now()).remove(0);
    let mut ended = crate::schedules::tests::info();
    ended.state = vessel_proto::automation::ScheduleState::Ended {
        at: ended.updated_at,
        reason: vessel_proto::automation::EndReason::MaxRuns,
    };
    let mut snapshot = crate::schedules::page::tests::snapshot(vec![ended]);
    snapshot.scopes = Ok(vec![vessel_proto::Scope::Read]);
    page.listed(&request, Ok(snapshot));
    let row = page.row(&key).unwrap();
    assert!(row.actions().is_empty());
    let mut reader = Reader::default();
    assert!(reader.open(&row).is_some());
    assert!(reader.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    page.connected(&key.machine);
    assert!(reader.open(&page.row(&key).unwrap()).is_none());
}
