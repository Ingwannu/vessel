//! Opened-FD admission and recovery namespace fencing share the sole `RootGate`.
use super::{WorkspaceError, Workspaces, exclusion::RecoveryRoots};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
use vessel_proto::{WorktreeId, checkpoint::PhysicalRoot};
use vessel_workspace::physical::Directory;

pub(super) struct OpenedAdmission {
    pub(super) directory: Arc<Directory>,
    pub(super) root_directory: Arc<Directory>,
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
        self.root_metadata(move || {
            let directory = Arc::new(Directory::open(&path)?);
            let ancestry = directory.ancestry()?;
            let mut known: BTreeMap<PhysicalRoot, BTreeSet<WorktreeId>> = BTreeMap::new();
            let mut root_directory: Option<Arc<Directory>> = None;
            for (id, path) in registered {
                match Directory::open(&path) {
                    Ok(opened) => {
                        let identity = opened.identity();
                        let _inserted = known.entry(identity).or_default().insert(id);
                        // Pin only the nearest selected ancestor, never every
                        // registered directory. This prevents inode reuse while
                        // bounding retained FDs independently of registry size.
                        let candidates = root_directory
                            .iter()
                            .map(|root| root.identity())
                            .chain(std::iter::once(identity))
                            .collect();
                        if ancestry.nearest(&candidates) == Some(identity) {
                            root_directory = Some(Arc::new(opened));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            let root_directory = root_directory.unwrap_or_else(|| Arc::clone(&directory));
            let root = root_directory.identity();
            let worktrees = known.remove(&root).unwrap_or_default();
            Ok(OpenedAdmission {
                directory,
                root_directory,
                root,
                worktrees,
            })
        })
        .await
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
