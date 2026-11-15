//! In-memory action outcomes exercise partial success, navigation and rechecked eligibility.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use crate::subagents::{Host, ManagedChild, Visibility};
use vessel_proto::{
    Creator, HarnessKind, NodeId, SessionId, SessionInfo, SessionMode, SessionSpec,
};
fn track() -> Track {
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    Track::new(Target::new(context, SessionId::random()))
}
fn row(track: &Track, finished: Option<bool>) -> ManagedChild {
    let target = Target::new(track.target().context.clone(), SessionId::random());
    let parent = track.target().address;
    let info = SessionInfo::new(
        target.address.session,
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
    .with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    ManagedChild {
        target,
        info: Box::new(info),
        finished,
        visibility: Visibility::Active,
    }
}
fn rows(track: &mut Track, rows: Vec<ManagedChild>) {
    track
        .managed_host(&Host::new(track.target().context.clone()), rows)
        .unwrap();
}
#[test]
fn batches_are_sequential_and_explicit_retry_never_repeats_an_acknowledged_success() {
    let mut track = track();
    let good = row(&track, Some(true));
    let bad = row(&track, Some(true));
    rows(&mut track, vec![good.clone(), bad.clone()]);
    let mut batch = ArchiveFinished::default();
    let mut tickets = Tickets::default();
    batch.start(&mut track).unwrap();
    let first = batch.next(&track, &mut tickets).unwrap().unwrap();
    assert_eq!(batch.next(&track, &mut tickets), Err(TrackProblem::Busy));
    assert!(batch.received(&first, Ok(()), &track));
    let second = batch.next(&track, &mut tickets).unwrap().unwrap();
    assert!(batch.received(&second, Err(TrackProblem::Timeout), &track));
    assert_eq!(batch.status(), FinishStatus::Failed { failed: 1 });
    batch.retry(&track).unwrap();
    let retry = batch.next(&track, &mut tickets).unwrap().unwrap();
    assert_eq!(retry.child, second.child);
    assert_ne!(retry.child, first.child);
    assert_ne!(retry.ticket, second.ticket);
}
#[test]
fn active_or_unknown_eligibility_is_not_archived_and_metadata_is_rechecked_before_next() {
    let mut track = track();
    let finished = row(&track, Some(true));
    let mut active = finished.clone();
    active.finished = Some(false);
    let unknown = row(&track, None);
    rows(&mut track, vec![finished, unknown]);
    let mut batch = ArchiveFinished::default();
    batch.start(&mut track).unwrap();
    rows(&mut track, vec![active]);
    assert_eq!(batch.next(&track, &mut Tickets::default()).unwrap(), None);
    assert_eq!(batch.status(), FinishStatus::Idle);
}
#[test]
fn late_answers_and_same_parent_on_another_admission_cannot_settle_the_batch() {
    let mut track = track();
    let finished = row(&track, Some(true));
    rows(&mut track, vec![finished]);
    let mut batch = ArchiveFinished::default();
    let mut tickets = Tickets::default();
    batch.start(&mut track).unwrap();
    let request = batch.next(&track, &mut tickets).unwrap().unwrap();
    let mut stale = request.clone();
    stale.ticket = tickets.next_ticket().unwrap();
    assert!(!batch.received(&stale, Ok(()), &track));
    let mut epochs = ConnectionEpochs::default();
    let _prior = epochs
        .admit(
            MachineKey::from_book("fixture"),
            track.target().address.node,
        )
        .unwrap();
    let changed = epochs
        .admit(
            MachineKey::from_book("fixture"),
            track.target().address.node,
        )
        .unwrap();
    let replacement = Track::new(Target::new(changed, track.target().address.session));
    assert!(!batch.received(&request, Ok(()), &replacement));
    assert!(batch.active().is_some());
}
#[test]
fn a_failure_stops_being_retryable_after_current_parent_or_finished_eligibility_changes() {
    let mut track = track();
    let finished = row(&track, Some(true));
    rows(&mut track, vec![finished]);
    let mut batch = ArchiveFinished::default();
    let mut tickets = Tickets::default();
    batch.start(&mut track).unwrap();
    let request = batch.next(&track, &mut tickets).unwrap().unwrap();
    assert!(batch.received(&request, Err(TrackProblem::Uncertain), &track));
    rows(&mut track, vec![]);
    batch.reconcile(&track);
    assert_eq!(batch.status(), FinishStatus::Idle);
    assert!(batch.failures().is_empty());
}
