//! Deterministic registry/resource port: no provider, process or filesystem access.
use super::super::{ArchiveBackend, Backing, Claim, Member, Stage};
use std::{
    collections::BTreeSet,
    sync::{Mutex, MutexGuard},
};
use tokio::sync::watch;
use vessel_proto::{
    WorkspaceId,
    workspace::{KeptReason, RemoveWorktreeOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Life {
    Active,
    Archiving,
    Archived,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Starts {
    Pending,
    Settled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Storage {
    Local,
    Shared,
    Dirty,
    Removed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Fault {
    #[error("injected stage refusal: {0:?}")]
    Stage(Stage),
    #[error("wrong workspace identity")]
    Identity,
    #[error("workspace archive is already claimed")]
    Claimed,
}
pub(super) struct State {
    pub life: Life,
    pub members: Vec<Member>,
    pub stopped: BTreeSet<Member>,
    pub calls: Vec<Stage>,
    pub refuse: Option<Stage>,
    pub fail_member: Option<Member>,
    pub storage: Storage,
}
pub(super) struct Fake {
    pub workspace: WorkspaceId,
    state: Mutex<State>,
    pub starts: watch::Sender<Starts>,
    pub waiting: watch::Sender<Starts>,
}
impl Fake {
    pub(super) fn new(members: Vec<Member>) -> Self {
        Self {
            workspace: WorkspaceId::random(),
            state: Mutex::new(State {
                life: Life::Active,
                members,
                stopped: BTreeSet::new(),
                calls: Vec::new(),
                refuse: None,
                fail_member: None,
                storage: Storage::Local,
            }),
            starts: watch::channel(Starts::Settled).0,
            waiting: watch::channel(Starts::Settled).0,
        }
    }
    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
    fn enter(&self, phase: Stage) -> Result<(), Fault> {
        let mut state = self.state();
        state.calls.push(phase);
        if phase != Stage::Claim && state.life != Life::Archiving {
            return Err(Fault::Claimed);
        }
        if state.refuse == Some(phase) {
            Err(Fault::Stage(phase))
        } else {
            Ok(())
        }
    }
}
impl ArchiveBackend for Fake {
    type Claim = WorkspaceId;
    type Error = Fault;
    async fn claim(&self, workspace: WorkspaceId) -> Result<Claim<WorkspaceId>, Fault> {
        if workspace != self.workspace {
            return Err(Fault::Identity);
        }
        self.enter(Stage::Claim)?;
        let mut state = self.state();
        match state.life {
            Life::Archived => Ok(Claim::AlreadyArchived),
            Life::Archiving => Err(Fault::Claimed),
            Life::Active => {
                state.life = Life::Archiving;
                Ok(Claim::Acquired(workspace))
            }
        }
    }
    async fn stop_setup(&self, claim: &WorkspaceId) -> Result<(), Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Setup)
    }
    async fn settle_starts(&self, claim: &WorkspaceId) -> Result<(), Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Starts)?;
        let mut starts = self.starts.subscribe();
        let _previous = self.waiting.send_replace(Starts::Pending);
        while *starts.borrow_and_update() == Starts::Pending {
            starts.changed().await.unwrap();
        }
        Ok(())
    }
    async fn members(&self, claim: &WorkspaceId) -> Result<Vec<Member>, Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Members)?;
        assert_eq!(*self.starts.borrow(), Starts::Settled);
        Ok(self.state().members.clone())
    }
    async fn stop_member(&self, claim: &WorkspaceId, member: Member) -> Result<(), Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Drain)?;
        let mut state = self.state();
        if state.fail_member == Some(member) {
            return Err(Fault::Stage(Stage::Drain));
        }
        assert!(
            state.stopped.insert(member),
            "a duplicate row issued a second mutation"
        );
        Ok(())
    }
    async fn cleanup_backing(&self, claim: &WorkspaceId) -> Result<Backing, Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Backing)?;
        let state = self.state();
        assert!(
            state
                .members
                .iter()
                .all(|member| state.stopped.contains(member))
        );
        Ok(match state.storage {
            Storage::Local => Backing::LocalPreserved,
            Storage::Shared => Backing::SharedPreserved,
            Storage::Dirty => Backing::Managed(RemoveWorktreeOutcome::Kept {
                reasons: vec![KeptReason::Uncommitted { files: 1 }],
            }),
            Storage::Removed => Backing::Managed(RemoveWorktreeOutcome::Removed { snapshot: None }),
        })
    }
    async fn finish(&self, claim: &WorkspaceId) -> Result<(), Fault> {
        assert_eq!(*claim, self.workspace);
        self.enter(Stage::Commit)?;
        assert_eq!(*self.starts.borrow(), Starts::Settled);
        self.state().life = Life::Archived;
        Ok(())
    }
}
