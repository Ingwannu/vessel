//! Restore claims fence concurrent backing creation and retain failed recovery state.
use super::super::restore::{RecoveryStage, Unavailable};
use super::super::{Recovery, RecoveryAction, RecoveryReport, RestoreBackend, restore};
use std::sync::{Mutex, MutexGuard};
use tokio::sync::watch;
use vessel_proto::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Life {
    Archived,
    Restoring,
    Active,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
enum Fault {
    #[error("injected recovery refusal: {0:?}")]
    Stage(RecoveryStage),
    #[error("workspace is already claimed")]
    Claimed,
}
struct State {
    life: Life,
    calls: Vec<RecoveryStage>,
    unavailable: Option<Unavailable>,
    refuse: Option<RecoveryStage>,
    action: RecoveryAction,
}
struct Fake {
    workspace: WorkspaceId,
    state: Mutex<State>,
    backing: watch::Sender<Option<RecoveryAction>>,
    claimed: watch::Sender<Life>,
}
impl Fake {
    fn new(action: RecoveryAction) -> Self {
        Self {
            workspace: WorkspaceId::random(),
            state: Mutex::new(State {
                life: Life::Archived,
                calls: Vec::new(),
                unavailable: None,
                refuse: None,
                action,
            }),
            backing: watch::channel(Some(action)).0,
            claimed: watch::channel(Life::Archived).0,
        }
    }
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
    fn enter(&self, phase: RecoveryStage) -> Result<(), Fault> {
        let mut state = self.state();
        state.calls.push(phase);
        if state.refuse == Some(phase) {
            Err(Fault::Stage(phase))
        } else {
            Ok(())
        }
    }
}
impl RestoreBackend for Fake {
    type Plan = RecoveryAction;
    type Claim = WorkspaceId;
    type Error = Fault;
    async fn inspect(&self, workspace: WorkspaceId) -> Result<Recovery<RecoveryAction>, Fault> {
        assert_eq!(workspace, self.workspace);
        self.enter(RecoveryStage::Inspect)?;
        let state = self.state();
        Ok(match state.unavailable {
            Some(reason) => Recovery::Unavailable(reason),
            None => Recovery::Ready(state.action),
        })
    }
    async fn claim_restore(
        &self,
        workspace: WorkspaceId,
        plan: &RecoveryAction,
    ) -> Result<WorkspaceId, Fault> {
        assert_eq!(workspace, self.workspace);
        self.enter(RecoveryStage::Claim)?;
        let mut state = self.state();
        assert_eq!(*plan, state.action);
        if state.life != Life::Archived {
            return Err(Fault::Claimed);
        }
        state.life = Life::Restoring;
        let _previous = self.claimed.send_replace(Life::Restoring);
        Ok(workspace)
    }
    async fn restore_backing(
        &self,
        claim: &WorkspaceId,
        plan: &RecoveryAction,
    ) -> Result<RecoveryAction, Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(RecoveryStage::Backing)?;
        assert_eq!(self.state().life, Life::Restoring);
        let mut backing = self.backing.subscribe();
        let action = *backing.wait_for(Option::is_some).await.unwrap();
        assert_eq!(action, Some(*plan));
        Ok(*plan)
    }
    async fn activate(&self, claim: &WorkspaceId) -> Result<(), Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(RecoveryStage::Activate)?;
        assert_eq!(self.state().life, Life::Restoring);
        self.state().life = Life::Active;
        Ok(())
    }
}

#[tokio::test]
async fn restore_inspects_and_revalidates_before_activation_without_session_launch() {
    for action in [RecoveryAction::Unarchive, RecoveryAction::RecreateBacking] {
        let fake = Fake::new(action);
        assert!(
            matches!(restore(&fake, fake.workspace).await, RecoveryReport::Restored(actual) if actual == action)
        );
        assert_eq!(fake.state().life, Life::Active);
        assert_eq!(
            fake.state().calls.as_slice(),
            [
                RecoveryStage::Inspect,
                RecoveryStage::Claim,
                RecoveryStage::Backing,
                RecoveryStage::Activate
            ]
        );
    }
}

#[tokio::test]
async fn unavailable_sources_never_claim_or_create() {
    for reason in [
        Unavailable::WorkspaceMissing,
        Unavailable::ProjectMissing,
        Unavailable::DirectoryMissing,
        Unavailable::SourceMissing,
        Unavailable::UnsupportedBacking,
        Unavailable::UncertainStarts,
    ] {
        let fake = Fake::new(RecoveryAction::RecreateBacking);
        fake.state().unavailable = Some(reason);
        assert!(
            matches!(restore(&fake, fake.workspace).await, RecoveryReport::Unavailable(actual) if actual == reason)
        );
        assert_eq!(fake.state().life, Life::Archived);
        assert_eq!(fake.state().calls.as_slice(), [RecoveryStage::Inspect]);
    }
}

#[tokio::test]
async fn recovery_failure_retains_stage_cause_and_nonactive_claim() {
    for stage in [
        RecoveryStage::Inspect,
        RecoveryStage::Claim,
        RecoveryStage::Backing,
        RecoveryStage::Activate,
    ] {
        let fake = Fake::new(RecoveryAction::RecreateBacking);
        fake.state().refuse = Some(stage);
        assert!(matches!(restore(&fake, fake.workspace).await,
            RecoveryReport::Failed { stage: actual, source: Fault::Stage(cause) } if actual == stage && cause == stage));
        assert_eq!(fake.state().calls.last(), Some(&stage));
        assert_eq!(
            fake.state().life,
            match stage {
                RecoveryStage::Inspect | RecoveryStage::Claim => Life::Archived,
                RecoveryStage::Backing | RecoveryStage::Activate => Life::Restoring,
            }
        );
    }
}

#[tokio::test(start_paused = true)]
async fn timeout_is_uncertain_and_leaves_restoring_fence() {
    let fake = Fake::new(RecoveryAction::RecreateBacking);
    let _previous = fake.backing.send_replace(None);
    assert!(matches!(
        restore(&fake, fake.workspace).await,
        RecoveryReport::Uncertain
    ));
    assert_eq!(fake.state().life, Life::Restoring);
    assert!(!fake.state().calls.contains(&RecoveryStage::Activate));
}

#[tokio::test]
async fn concurrent_restore_cannot_duplicate_backing_creation() {
    let fake = Fake::new(RecoveryAction::RecreateBacking);
    let _previous = fake.backing.send_replace(None);
    let mut claimed = fake.claimed.subscribe();
    let second = async {
        let _ready = claimed
            .wait_for(|life| *life == Life::Restoring)
            .await
            .unwrap();
        let report = restore(&fake, fake.workspace).await;
        assert!(matches!(
            report,
            RecoveryReport::Failed {
                stage: RecoveryStage::Claim,
                source: Fault::Claimed
            }
        ));
        let _previous = fake
            .backing
            .send_replace(Some(RecoveryAction::RecreateBacking));
    };
    let (first, ()) = tokio::join!(restore(&fake, fake.workspace), second);
    assert!(matches!(
        first,
        RecoveryReport::Restored(RecoveryAction::RecreateBacking)
    ));
    assert_eq!(
        fake.state()
            .calls
            .iter()
            .filter(|stage| **stage == RecoveryStage::Backing)
            .count(),
        1
    );
}
