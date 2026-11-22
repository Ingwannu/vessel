//! Archive authority, retained capacity and cleanup ownership at Store ingress.

use super::*;
use rusqlite::params;
use vessel_proto::automation::{AfterRun, RunNumber};
use vessel_proto::registry::WorkspacePin;
use vessel_proto::workspace::{SetupChoice, WorktreeBranch};
use vessel_proto::{ScheduleId, WorkspaceOperationId, WorktreeId};

use crate::{RunBindingStage, ScheduleRunBinding};

pub(super) fn publish(db: &mut Connection, record: RegistryRecord) -> RegistryReservation {
    let workspace = record.descriptor.id;
    let (isolation, worktree) = match record.descriptor.backing {
        WorkspaceBacking::Local => (WorkspaceIsolation::Local, None),
        WorkspaceBacking::Managed { worktree } => (
            WorkspaceIsolation::Worktree {
                branch: WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: SetupChoice::Skip,
            },
            Some(worktree),
        ),
    };
    let reservation = RegistryReservation {
        workspace,
        intent: WorkspaceProvision {
            project: record.descriptor.project,
            isolation,
            automation: record.descriptor.automation.clone(),
            title: record.descriptor.title.clone(),
            visibility: record.descriptor.visibility,
        },
        worktree,
        stage: ReservationStage::Reserved,
    };
    apply(db, &RegistryWrite::Reserve(reservation.clone())).unwrap();
    apply(
        db,
        &RegistryWrite::Stage {
            workspace,
            expected: ReservationStage::Reserved,
            stage: ReservationStage::Creating,
        },
    )
    .unwrap();
    apply(db, &RegistryWrite::Completed(Box::new(record))).unwrap();
    reservation
}

pub(super) fn claim(
    db: &mut Connection,
    workspace_id: WorkspaceId,
    purpose: WorkspaceArchivePurpose,
) -> WorkspaceArchiveClaim {
    let expected = workspace(db, workspace_id)
        .unwrap()
        .unwrap()
        .descriptor
        .revision;
    let claim = WorkspaceArchiveClaim {
        workspace: workspace_id,
        token: WorkspaceOperationId::random(),
        purpose,
    };
    apply(
        db,
        &RegistryWrite::ClaimArchive {
            claim,
            expected,
            at: DateTime::UNIX_EPOCH,
        },
    )
    .unwrap();
    claim
}

fn archive(db: &mut Connection, id: WorkspaceId) {
    let claim = claim(db, id, WorkspaceArchivePurpose::Explicit);
    apply(
        db,
        &RegistryWrite::SettleArchive {
            claim,
            settlement: ArchiveSettlement::Archived {
                at: DateTime::UNIX_EPOCH,
            },
        },
    )
    .unwrap();
}

fn retire(db: &mut Connection, id: WorkspaceId, nanos: i64) -> Result<(), StoreError> {
    apply(
        db,
        &RegistryWrite::Retire {
            workspace: id,
            evidence: ProvisionRetirement::CleanArchive,
            at: DateTime::from_timestamp_nanos(nanos),
        },
    )
}

#[test]
fn three_hundred_archives_release_capacity_and_prune_receipts_with_descriptors() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let first_id = first.descriptor.id;
    let initial = publish(&mut db, first);
    archive(&mut db, first_id);
    retire(&mut db, first_id, 0).unwrap();
    assert!(
        matches!(apply(&mut db, &RegistryWrite::Reserve(initial)), Err(StoreError::RegistryFinished(id)) if id == first_id)
    );
    for step in 1..300 {
        let id = WorkspaceId::random();
        let _reservation = publish(&mut db, record(id));
        archive(&mut db, id);
        retire(&mut db, id, step).unwrap();
    }
    assert!(workspace(&db, first_id).unwrap().is_none());
    assert!(reservation(&db, first_id).unwrap().is_none());
    assert_eq!(load(&db).unwrap().workspaces.len(), MAX_ARCHIVED_WORKSPACES);
    let retired: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM workspace_registry_reservations WHERE stage='retired'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(usize::try_from(retired).unwrap(), MAX_ARCHIVED_WORKSPACES);
}

#[test]
fn a_kept_managed_backing_retains_its_slot_even_after_the_archive() {
    let mut db = db();
    let mut item = record(WorkspaceId::random());
    let id = item.descriptor.id;
    let project = item.descriptor.project;
    let backing = WorktreeId::random();
    item.descriptor.backing = WorkspaceBacking::Managed { worktree: backing };
    let _written = db
        .execute(
            "INSERT INTO projects VALUES (?1,0,'{}')",
            [project.to_string()],
        )
        .unwrap();
    let _written = db
        .execute(
            "INSERT INTO worktrees VALUES (?1,?2,0,'{}')",
            params![backing.to_string(), project.to_string()],
        )
        .unwrap();
    let intent = publish(&mut db, item).intent;
    archive(&mut db, id);
    assert!(matches!(
        retire(&mut db, id, 0),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(worktree_reference(&db, backing, None).unwrap(), Some(id));
    assert_eq!(
        worktree_reference(&db, WorktreeId::random(), None).unwrap(),
        None
    );
    for _ in 1..MAX_WORKSPACES {
        apply(
            &mut db,
            &RegistryWrite::Reserve(RegistryReservation {
                workspace: WorkspaceId::random(),
                intent: intent.clone(),
                worktree: None,
                stage: ReservationStage::Reserved,
            }),
        )
        .unwrap();
    }
    let next = RegistryWrite::Reserve(RegistryReservation {
        workspace: WorkspaceId::random(),
        intent,
        worktree: None,
        stage: ReservationStage::Reserved,
    });
    assert!(matches!(
        apply(&mut db, &next),
        Err(StoreError::RegistryFull)
    ));
    let _written = db
        .execute("DELETE FROM worktrees WHERE id=?1", [backing.to_string()])
        .unwrap();
    retire(&mut db, id, 1).unwrap();
    assert_eq!(worktree_reference(&db, backing, None).unwrap(), None);
    apply(&mut db, &next).unwrap();
}

#[test]
fn metadata_cannot_settle_an_archive_and_a_deferred_token_cannot_reclaim() {
    let mut db = db();
    let id = WorkspaceId::random();
    let _reservation = publish(&mut db, record(id));
    let automatic = claim(&mut db, id, WorkspaceArchivePurpose::Automatic);
    let mut item = workspace(&db, id).unwrap().unwrap();
    let expected = item.descriptor.revision;
    item.descriptor.revision = expected.next().unwrap();
    item.descriptor.title = Some("Still archiving".parse().unwrap());
    item.descriptor.pin = WorkspacePin::Pinned;
    let mut bypass = item.clone();
    bypass.descriptor.lifecycle = WorkspaceLifecycle::Active;
    assert!(matches!(
        put(&mut db, bypass, Some(expected)),
        Err(StoreError::RegistryConflict)
    ));
    put(&mut db, item, Some(expected)).unwrap();
    apply(
        &mut db,
        &RegistryWrite::SettleArchive {
            claim: automatic,
            settlement: ArchiveSettlement::Deferred(ArchiveDefer::OtherMembers),
        },
    )
    .unwrap();
    assert_eq!(
        archive_claim(&db, id).unwrap().unwrap().deferred,
        Some(ArchiveDefer::OtherMembers)
    );
    let explicit = claim(&mut db, id, WorkspaceArchivePurpose::Explicit);
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::SettleArchive {
                claim: automatic,
                settlement: ArchiveSettlement::Archived {
                    at: DateTime::UNIX_EPOCH
                },
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    apply(
        &mut db,
        &RegistryWrite::SettleArchive {
            claim: explicit,
            settlement: ArchiveSettlement::Archived {
                at: DateTime::UNIX_EPOCH,
            },
        },
    )
    .unwrap();
    let record = workspace(&db, id).unwrap().unwrap();
    assert_eq!(record.descriptor.pin, WorkspacePin::Pinned);
    assert_eq!(record.descriptor.title.unwrap().as_str(), "Still archiving");
}

fn making(db: &Connection, workspace: WorkspaceId) -> ScheduleRunBinding {
    let schedule = ScheduleId::random();
    let _written = db
        .execute(
            "INSERT INTO schedules VALUES (?1,0,'{}')",
            [schedule.to_string()],
        )
        .unwrap();
    let _written = db
        .execute(
            "INSERT INTO schedule_runs VALUES (?1,1,'{\"outcome\":{\"outcome\":\"making\"}}')",
            [schedule.to_string()],
        )
        .unwrap();
    ScheduleRunBinding {
        schedule,
        run: RunNumber::FIRST,
        workspace,
        session: SessionId::random(),
        after: AfterRun::Archive,
        stage: RunBindingStage::Bound,
    }
}

#[test]
fn unresolved_run_pins_are_bounded_and_survive_history_deletion() {
    use crate::automation::{AutomationWrite, apply as write_run};
    let mut db = db();
    let id = WorkspaceId::random();
    let first = ScheduleRunBinding {
        after: AfterRun::Keep,
        ..making(&db, id)
    };
    write_run(&db, &AutomationWrite::BindRun(first)).unwrap();
    for _ in 1..MAX_WORKSPACES {
        let next = making(&db, WorkspaceId::random());
        write_run(&db, &AutomationWrite::BindRun(next)).unwrap();
    }
    let over = making(&db, WorkspaceId::random());
    assert!(matches!(
        write_run(&db, &AutomationWrite::BindRun(over)),
        Err(StoreError::RegistryFull)
    ));
    write_run(&db, &AutomationWrite::ForgetSchedule(first.schedule)).unwrap();
    assert_eq!(
        crate::automation::binding::find(&db, first.schedule, first.run).unwrap(),
        Some(first)
    );
    let stage = RunBindingStage::Started;
    write_run(
        &db,
        &AutomationWrite::AdvanceRunBinding {
            previous: first,
            stage,
        },
    )
    .unwrap();
    let started = ScheduleRunBinding { stage, ..first };
    let stage = RunBindingStage::Kept;
    write_run(
        &db,
        &AutomationWrite::AdvanceRunBinding {
            previous: started,
            stage,
        },
    )
    .unwrap();
    let first = ScheduleRunBinding { stage, ..first };
    assert!(
        crate::automation::binding::awaiting_archive(&db)
            .unwrap()
            .is_empty()
    );
    assert!(!crate::automation::binding::request_keep_release(&db, SessionId::random()).unwrap());
    assert!(crate::automation::binding::request_keep_release(&db, first.session).unwrap());
    assert_eq!(
        crate::automation::binding::awaiting_archive(&db).unwrap(),
        [first]
    );

    let _reservation = publish(&mut db, record(id));
    archive(&mut db, id);
    assert!(matches!(
        retire(&mut db, id, 0),
        Err(StoreError::RegistryConflict)
    ));
    let stage = RunBindingStage::RetiringArchive;
    write_run(
        &db,
        &AutomationWrite::AdvanceRunBinding {
            previous: first,
            stage,
        },
    )
    .unwrap();
    retire(&mut db, id, 1).unwrap();
    retire(&mut db, id, 1).unwrap();
    let terminal = ScheduleRunBinding { stage, ..first };
    assert_eq!(
        crate::automation::binding::find(&db, first.schedule, first.run).unwrap(),
        Some(terminal)
    );
    write_run(&db, &AutomationWrite::FinishRunBinding(terminal)).unwrap();
    write_run(&db, &AutomationWrite::BindRun(over)).unwrap();
}

#[test]
fn a_provisioned_local_fences_project_removal_until_its_pending_start_is_settled() {
    let mut db = db();
    let item = record(WorkspaceId::random());
    let id = item.descriptor.id;
    let project = item.descriptor.project;
    let _written = db
        .execute(
            "INSERT INTO projects VALUES (?1,0,'{}')",
            [project.to_string()],
        )
        .unwrap();
    let placement = Placement {
        project,
        path: item.descriptor.path.clone(),
    };
    let _reservation = publish(&mut db, item);
    assert_eq!(project_reference(&db, project).unwrap(), Some(id));
    let legacy_id = WorkspaceId::random();
    let legacy = Register {
        placement,
        fresh: legacy_id,
        at: DateTime::UNIX_EPOCH,
    };
    assert_eq!(register(&db, &legacy).unwrap().unwrap().0, legacy_id);

    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    apply(
        &mut db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let removal = crate::workspace::WorkspaceWrite::ForgetProject {
        project,
        at: DateTime::UNIX_EPOCH,
    };
    assert!(matches!(
        crate::workspace::apply(&db, &removal),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(
        workspace(&db, legacy_id)
            .unwrap()
            .unwrap()
            .descriptor
            .lifecycle,
        WorkspaceLifecycle::Active
    );
    assert_eq!(pending_starts(&db, id).unwrap().len(), 1);

    let lease = pending_starts(&db, id).unwrap().remove(0).lease;
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease,
            settled: RegistryStartSettlement::Failed,
        },
    )
    .unwrap();
    archive(&mut db, id);
    retire(&mut db, id, 1).unwrap();
    assert_eq!(project_reference(&db, project).unwrap(), None);
    let _changed = crate::workspace::apply(&db, &removal).unwrap();
    assert!(matches!(
        workspace(&db, legacy_id)
            .unwrap()
            .unwrap()
            .descriptor
            .lifecycle,
        WorkspaceLifecycle::Archived { .. }
    ));
    assert_eq!(
        reservation(&db, id).unwrap().unwrap().stage,
        ReservationStage::Retired
    );
}
