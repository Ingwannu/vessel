//! One durable-operation index prevents resolving one alias from clearing another.
use super::{ROOT_LEASES, RootRefusal};
use std::collections::{BTreeMap, BTreeSet};
use vessel_proto::{
    WorktreeId,
    checkpoint::{PhysicalRoot, RestoreId},
};

#[derive(Debug, Default)]
pub(crate) struct RecoveryRoots(BTreeMap<PhysicalRoot, BTreeMap<RestoreId, WorktreeId>>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryRoot {
    pub(crate) physical: PhysicalRoot,
    pub(crate) worktree: WorktreeId,
}

impl RecoveryRoots {
    pub(in crate::workspace) fn add(
        &mut self,
        root: PhysicalRoot,
        worktree: WorktreeId,
        restore: RestoreId,
    ) -> Result<(), RootRefusal> {
        if !self.0.contains_key(&root) && self.0.len() >= ROOT_LEASES {
            return Err(RootRefusal::Busy);
        }
        let pending = self.0.entry(root).or_default();
        if let Some(recorded) = pending.get(&restore) {
            return if *recorded == worktree {
                Ok(())
            } else {
                Err(RootRefusal::RecoveryPending)
            };
        }
        if pending.len() >= super::super::MAX_WORKTREES {
            return Err(RootRefusal::Busy);
        }
        let _inserted = pending.insert(restore, worktree);
        Ok(())
    }
    pub(super) fn operation(
        &self,
        restore: RestoreId,
    ) -> Result<Option<RecoveryRoot>, RootRefusal> {
        let mut found = None;
        for (physical, pending) in &self.0 {
            if let Some(worktree) = pending.get(&restore) {
                if found.is_some() {
                    return Err(RootRefusal::RecoveryPending);
                }
                found = Some(RecoveryRoot {
                    physical: *physical,
                    worktree: *worktree,
                });
            }
        }
        Ok(found)
    }
    pub(super) fn contains(&self, root: &PhysicalRoot) -> bool {
        self.0.contains_key(root)
    }
    pub(super) fn affects(&self, worktrees: &BTreeSet<WorktreeId>) -> bool {
        self.0.values().any(|pending| {
            pending
                .values()
                .any(|worktree| worktrees.contains(worktree))
        })
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn has(&self, root: &PhysicalRoot, restore: &RestoreId) -> bool {
        self.0
            .get(root)
            .is_some_and(|pending| pending.contains_key(restore))
    }
    pub(super) fn settled(
        &mut self,
        root: PhysicalRoot,
        restore: RestoreId,
    ) -> Result<(), RootRefusal> {
        let pending = self.0.get_mut(&root).ok_or(RootRefusal::NotRecovering)?;
        if pending.remove(&restore).is_none() {
            return Err(RootRefusal::NotRecovering);
        }
        if pending.is_empty() {
            let _removed = self.0.remove(&root);
        }
        Ok(())
    }
}
