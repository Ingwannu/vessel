//! Build only the backing; descriptor publication precedes any future bootstrap.
use super::*;
use chrono::Utc;
use vessel_proto::registry::{RegistryRevision, WorkspaceBacking, WorkspacePin};
use vessel_proto::workspace::{CreateWorktree, CreateWorktreeOutcome, SetupChoice};
use vessel_store::RegistryRecord;

pub(super) enum Made {
    Ready(Box<RegistryRecord>),
    Approval(Box<ProjectCommands>),
    Refused(WorktreeRefusal),
}
impl Workspaces {
    pub(super) async fn make_backing(
        self: &Arc<Self>,
        creator: Creator,
        reservation: &RegistryReservation,
        admission: &Arc<ProjectAdmission>,
    ) -> Result<Made, WorkspaceError> {
        let (path, backing, recovery) = match &reservation.intent.isolation {
            WorkspaceIsolation::Local => (
                admission.root().path().to_path_buf(),
                WorkspaceBacking::Local,
                None,
            ),
            WorkspaceIsolation::Worktree { branch, .. } => {
                let fixed = reservation.worktree.ok_or_else(|| {
                    WorkspaceError::Conflict("workspace backing identity is absent".to_owned())
                })?;
                match self
                    .create_worktree_admitted(
                        Arc::clone(admission),
                        creator,
                        CreateWorktree {
                            project: reservation.intent.project,
                            branch: branch.clone(),
                            setup: SetupChoice::Skip,
                        },
                        None,
                        fixed,
                    )
                    .await?
                {
                    CreateWorktreeOutcome::Created { worktree } => {
                        if worktree.id != fixed || worktree.project != reservation.intent.project {
                            return Err(WorkspaceError::Conflict(
                                "workspace backing identity changed".to_owned(),
                            ));
                        }
                        (
                            worktree.path.clone(),
                            WorkspaceBacking::Managed { worktree: fixed },
                            Some(worktree),
                        )
                    }
                    CreateWorktreeOutcome::NeedsApproval { commands } => {
                        return Ok(Made::Approval(Box::new(commands)));
                    }
                    CreateWorktreeOutcome::Refused { reason } => return Ok(Made::Refused(reason)),
                }
            }
        };
        Ok(Made::Ready(Box::new(RegistryRecord {
            descriptor: WorkspaceDescriptor {
                id: reservation.workspace,
                project: reservation.intent.project,
                path,
                title: reservation.intent.title.clone(),
                automation: reservation.intent.automation.clone(),
                pin: WorkspacePin::Unpinned,
                backing,
                lifecycle: WorkspaceLifecycle::Active,
                visibility: reservation.intent.visibility,
                created_at: Utc::now(),
                revision: RegistryRevision::ZERO,
            },
            recovery,
        })))
    }
}
