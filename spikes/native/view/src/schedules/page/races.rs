//! No late reply or reconnect may create room for duplicate writes.
use super::tests::{bound, rebound};
use super::tests::{key, loaded, snapshot};
use super::*;
use crate::schedules::{Action, tests::info};
use vessel_proto::automation::{ScheduleState, ScheduleTarget};

#[test]
fn invalidated_calls_keep_their_capacity_until_their_actual_reply() {
    let mut page = Page::default();
    let hosts: Vec<_> = (0..MAX_IN_FLIGHT)
        .map(|n| key(&format!("node{n}")))
        .collect();
    bound(&mut page, &hosts);
    let requests = page.due();
    assert_eq!(requests.len(), MAX_IN_FLIGHT);
    for _ in 0..MAX_IN_FLIGHT {
        page.refresh();
        assert!(page.due().is_empty());
    }
    page.listed(&requests[0], Ok(snapshot(vec![info()])));
    assert!(page.visible_rows().is_empty());
    let next = page.due();
    assert_eq!(next.len(), 1);
    // A duplicate obsolete reply cannot release the new request's capacity.
    page.listed(&requests[0], Ok(snapshot(vec![info()])));
    assert!(page.due().is_empty());
    page.listed(&next[0], Ok(snapshot(vec![info()])));
    assert_eq!(page.visible_rows().len(), 1);
}

#[test]
fn lost_run_reply_requires_a_read_before_manual_retry_and_never_dispatches_itself() {
    let (mut page, row) = loaded();
    let run = page.act(&row, Action::RunNow).unwrap();
    page.written(&run, Err(Failure::Unconfirmed));
    assert_eq!(
        page.row(&row).unwrap().mutation.failure(),
        Some(&Failure::Unconfirmed)
    );
    assert!(page.act(&row, Action::RunNow).is_none());
    let reads = page.due();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].ask, Ask::List);
    page.listed(&reads[0], Err(Failure::Offline));
    assert!(page.act(&row, Action::RunNow).is_none());
    page.retry_failed();
    let refresh = page.due().remove(0);
    assert_eq!(refresh.ask, Ask::List);
    page.listed(&refresh, Ok(snapshot(vec![info()])));
    assert_eq!(
        page.row(&row).unwrap().mutation.failure(),
        Some(&Failure::Unconfirmed)
    );
    assert!(page.due().is_empty());
    assert!(page.act(&row, Action::RunNow).is_some());
}

#[test]
fn a_new_connection_preserves_uncertain_writes_but_refuses_the_old_reply() {
    let (mut page, row) = loaded();
    let run = page.act(&row, Action::RunNow).unwrap();
    page.connected(&row.machine);
    let _changed = page.admitted(rebound(&row.machine));
    assert!(page.act(&row, Action::RunNow).is_none());
    let refresh = page.due().remove(0);
    page.listed(&refresh, Ok(snapshot(vec![info()])));
    page.written(&run, Ok(WriteValue::Deleted));
    assert!(page.row(&row).is_some());
    assert_eq!(
        page.row(&row).unwrap().mutation.failure(),
        Some(&Failure::Unconfirmed)
    );
    assert_eq!(page.pending(), 0);
}

#[test]
fn mismatched_write_kind_or_identity_is_uncertain_and_cannot_change_the_row() {
    for bad in [
        WriteValue::Deleted,
        WriteValue::Schedule(Box::new({
            let mut other = info();
            other.id = "00000000-0000-4000-8000-000000000009".parse().unwrap();
            other
        })),
    ] {
        let (mut page, row) = loaded();
        let pause = page.act(&row, Action::Pause).unwrap();
        page.written(&pause, Ok(bad));
        assert_eq!(page.row(&row).unwrap().schedule.id, row.schedule);
        assert_eq!(
            page.row(&row).unwrap().mutation.failure(),
            Some(&Failure::Unconfirmed)
        );
        assert!(page.act(&row, Action::Pause).is_none());
    }
}

#[test]
fn duplicate_read_identities_preserve_the_previous_authoritative_snapshot() {
    let (mut page, row) = loaded();
    page.refresh();
    let read = page.due().remove(0);
    page.listed(&read, Ok(snapshot(vec![info(), info()])));
    assert_eq!(page.visible_rows().len(), 1);
    assert_eq!(page.row(&row).unwrap().schedule.id, row.schedule);
    assert_eq!(page.problems(), [(&row.machine, &Failure::Unexpected)]);
}

#[test]
fn a_row_that_becomes_a_heartbeat_hides_the_pending_execution_control() {
    let (mut page, row) = loaded();
    let pause = page.act(&row, Action::Pause).unwrap();
    page.refresh();
    let read = page.due().remove(0);
    let mut paused = info();
    paused.revision += 1;
    paused.state = ScheduleState::Paused {
        since: tests::now(),
    };
    let mut changed = paused.clone();
    changed.revision += 1;
    changed.definition.target = ScheduleTarget::Session {
        session: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
    };
    page.listed(&read, Ok(snapshot(vec![changed])));
    let menu = page.row(&row).unwrap().menu_entries();
    assert_eq!(menu.len(), 1);
    assert_eq!(menu[0].action, Action::Delete);

    page.written(&pause, Ok(WriteValue::Schedule(Box::new(paused))));
    assert_eq!(page.row(&row).unwrap().actions(), [Action::Delete]);
}
