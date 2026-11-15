//! Opened-FD admission and recovery namespace fencing share the sole RootGate.
use super::{WorkspaceError, Workspaces, exclusion::RecoveryRoots};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use vessel_proto::{WorktreeId, checkpoint::PhysicalRoot};
use vessel_workspace::physical::Directory;

pub(super) struct OpenedAdmission {
    pub(super) directory: Directory,
    pub(super) root: PhysicalRoot,
    pub(super) worktrees: BTreeSet<WorktreeId>,
}

impl Workspaces {
    pub(super) async fn opened_admission(
        &self,
        path: &Path,
    ) -> Result<OpenedAdmission, WorkspaceError> {
        let path = path.to_path_buf();
        let registered = self
            .state()
            .worktrees
            .iter()
            .map(|(id, entry)| (*id, entry.info.path.clone()))
            .collect::<Vec<_>>();
        Ok(self
            .root_metadata(move || {
                let directory = Directory::open(&path)?;
                let ancestry = directory.ancestry()?;
                let mut known: BTreeMap<PhysicalRoot, BTreeSet<WorktreeId>> = BTreeMap::new();
                for (id, path) in registered {
                    match Directory::open(&path) {
                        Ok(opened) => {
                            let _inserted = known.entry(opened.identity()).or_default().insert(id);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                    }
                }
                let roots = known.keys().copied().collect();
                let root = ancestry
                    .nearest(&roots)
                    .unwrap_or_else(|| directory.identity());
                let worktrees = known.remove(&root).unwrap_or_default();
                Ok(OpenedAdmission {
                    directory,
                    root,
                    worktrees,
                })
            })
            .await?)
    }
}

/// Store startup has already fenced Prepared rows. Hydrate before any writer or
/// process-start admission; a replacement folder cannot evade its worktree pin.
pub(super) async fn recovering(
    store: &vessel_store::Store,
) -> Result<RecoveryRoots, vessel_store::StoreError> {
    let mut roots = RecoveryRoots::default();
    let mut after = None;
    loop {
        let page = store.checkpoint_recovery_page(after).await?;
        for operation in page.operations {
            let (plan, _consumed) = store
                .checkpoint_plan(operation.plan, operation.owner)
                .await?;
            roots
                .add(plan.evidence.physical(), plan.key.worktree, operation.id)
                .map_err(|_bound| {
                    vessel_store::CheckpointStoreError::from(
                        vessel_proto::checkpoint::CheckpointError::Busy,
                    )
                })?;
        }
        match page.next {
            Some(next) => after = Some(next),
            None => return Ok(roots),
        }
    }
}
