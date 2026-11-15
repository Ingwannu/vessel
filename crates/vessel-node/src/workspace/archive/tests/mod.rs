//! Workflow regressions exercise admission/side-effect ordering, not mock call counts alone.
mod fixture;
mod recovery;
use super::{ArchiveOutcome, Backing, Failure, Member, Stage, archive};
use fixture::{Fake, Fault, Life, Starts, Storage};
use vessel_proto::{SessionId, TerminalId, WorkspaceId, workspace::RemoveWorktreeOutcome};

#[tokio::test]
async fn exact_members_are_drained_once_and_safety_outcomes_are_retained() {
    let session = Member::Session(SessionId::random());
    let terminal = Member::Terminal(TerminalId::random());
    for storage in [
        Storage::Local,
        Storage::Shared,
        Storage::Dirty,
        Storage::Removed,
    ] {
        let fake = Fake::new(vec![session, terminal, session]);
        fake.state().storage = storage;
        let report = archive(&fake, fake.workspace).await;
        assert_eq!(report.workspace, fake.workspace);
        let ArchiveOutcome::Archived { backing } = report.outcome else {
            panic!("not archived")
        };
        match (storage, backing) {
            (Storage::Local, Backing::LocalPreserved)
            | (Storage::Shared, Backing::SharedPreserved) => {}
            (Storage::Dirty, Backing::Managed(RemoveWorktreeOutcome::Kept { reasons })) => {
                assert_eq!(reasons.len(), 1)
            }
            (Storage::Removed, Backing::Managed(RemoveWorktreeOutcome::Removed { snapshot })) => {
                assert!(snapshot.is_none())
            }
            other => panic!("safety outcome changed: {other:?}"),
        }
        assert_eq!(fake.state().life, Life::Archived);
        assert_eq!(fake.state().stopped.len(), 2);
    }
}

#[tokio::test]
async fn failed_member_keeps_claim_and_never_cleans_backing_but_other_members_drain() {
    let failed = Member::Session(SessionId::random());
    let other = Member::Terminal(TerminalId::random());
    let fake = Fake::new(vec![failed, other]);
    fake.state().fail_member = Some(failed);
    let report = archive(&fake, fake.workspace).await;
    let ArchiveOutcome::Incomplete(failures) = report.outcome else {
        panic!("partial was lost")
    };
    assert!(matches!(failures.as_slice(), [Failure::At {
        stage: Stage::Drain, member: Some(member), source: Fault::Stage(Stage::Drain),
    }] if *member == failed));
    let state = fake.state();
    assert_eq!(state.life, Life::Archiving);
    assert!(state.stopped.contains(&other));
    assert!(!state.calls.contains(&Stage::Backing));
    assert!(!state.calls.contains(&Stage::Commit));
}

#[tokio::test]
async fn stage_refusals_retain_cause_and_never_run_later_phases() {
    for stage in [
        Stage::Claim,
        Stage::Setup,
        Stage::Starts,
        Stage::Members,
        Stage::Backing,
        Stage::Commit,
    ] {
        let fake = Fake::new(Vec::new());
        fake.state().refuse = Some(stage);
        let report = archive(&fake, fake.workspace).await;
        let ArchiveOutcome::Incomplete(failures) = report.outcome else {
            panic!("refusal was lost")
        };
        assert!(matches!(failures.as_slice(), [Failure::At {
            stage: actual, member: None, source: Fault::Stage(cause),
        }] if *actual == stage && *cause == stage));
        assert_eq!(fake.state().calls.last(), Some(&stage));
        assert_eq!(
            fake.state().life,
            if stage == Stage::Claim {
                Life::Active
            } else {
                Life::Archiving
            }
        );
    }
}

#[tokio::test(start_paused = true)]
async fn unresolved_start_deadline_preserves_claim_without_backing_cleanup() {
    let fake = Fake::new(Vec::new());
    let _previous = fake.starts.send_replace(Starts::Pending);
    let report = archive(&fake, fake.workspace).await;
    assert!(
        matches!(report.outcome, ArchiveOutcome::Incomplete(failures) if matches!(failures.as_slice(), [Failure::Deadline]))
    );
    assert_eq!(fake.state().life, Life::Archiving);
    assert!(!fake.state().calls.contains(&Stage::Members));
    assert!(!fake.state().calls.contains(&Stage::Backing));
}

#[tokio::test]
async fn actor_registered_after_claim_is_included_after_lease_settlement() {
    let fake = Fake::new(Vec::new());
    let _previous = fake.starts.send_replace(Starts::Pending);
    let late = Member::Session(SessionId::random());
    let mut waiting = fake.waiting.subscribe();
    let arrival = async {
        let _ready = waiting
            .wait_for(|state| *state == Starts::Pending)
            .await
            .unwrap();
        assert_eq!(fake.state().life, Life::Archiving);
        fake.state().members.push(late);
        let _previous = fake.starts.send_replace(Starts::Settled);
    };
    let (report, ()) = tokio::join!(archive(&fake, fake.workspace), arrival);
    assert!(matches!(report.outcome, ArchiveOutcome::Archived { .. }));
    assert!(fake.state().stopped.contains(&late));
}

#[tokio::test]
async fn archived_is_idempotent_and_wrong_identity_has_no_side_effect() {
    let fake = Fake::new(Vec::new());
    let report = archive(&fake, WorkspaceId::random()).await;
    assert!(
        matches!(report.outcome, ArchiveOutcome::Incomplete(failures) if matches!(failures.as_slice(), [Failure::At { source: Fault::Identity, .. }]))
    );
    assert!(fake.state().calls.is_empty());
    fake.state().life = Life::Archived;
    assert!(matches!(
        archive(&fake, fake.workspace).await.outcome,
        ArchiveOutcome::AlreadyArchived
    ));
    assert_eq!(fake.state().calls.as_slice(), [Stage::Claim]);
}
