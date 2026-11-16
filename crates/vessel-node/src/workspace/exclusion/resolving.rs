//! Owned controller seam: resolving one operation never force-unlocks its aliases.
use super::{
    Active, Arc, CancellationToken, Instant, Notify, PhysicalRoot, Request, RootGate, RootLease,
    RootRefusal, State, WorktreeId, issue,
};
use vessel_proto::checkpoint::RestoreId;

#[derive(Debug, Clone, Copy)]
pub(super) enum Purpose {
    Writer,
    Recovering(RestoreId),
    Resolved(RestoreId),
}

impl RootGate {
    pub(crate) async fn claim_recovery(
        &self,
        root: PhysicalRoot,
        restore: RestoreId,
        deadline: Instant,
    ) -> Result<RootLease, RootRefusal> {
        self.request(deadline, |reply| Request::ClaimRecovery {
            root,
            restore,
            deadline,
            reply,
        })
        .await
    }
    pub(crate) async fn pin_recovery(
        &self,
        lease: &RootLease,
        worktree: WorktreeId,
        restore: RestoreId,
        deadline: Instant,
    ) -> Result<(), RootRefusal> {
        self.request(deadline, |reply| Request::PinRecovery {
            root: lease.root,
            worktree,
            restore,
            held: Arc::clone(&lease.release),
            deadline,
            reply,
        })
        .await
    }
    /// Caller has verified the whole before image and its FULL-sync journal result.
    pub(crate) async fn resolve_recovery(
        &self,
        lease: &RootLease,
        restore: RestoreId,
        deadline: Instant,
    ) -> Result<(), RootRefusal> {
        self.request(deadline, |reply| Request::ResolveRecovery {
            root: lease.root,
            restore,
            held: Arc::clone(&lease.release),
            deadline,
            reply,
        })
        .await
    }
}

pub(super) fn claim(
    root: PhysicalRoot,
    restore: RestoreId,
    deadline: Instant,
    state: &mut State,
    notify: &Arc<Notify>,
) -> Result<RootLease, RootRefusal> {
    if Instant::now() >= deadline {
        return Err(RootRefusal::TimedOut);
    }
    if !state.recovering.has(&root, &restore) {
        return Err(RootRefusal::NotRecovering);
    }
    let lease = issue(root, deadline, state, notify)?;
    if let Some(active) = state.active.get_mut(&root) {
        active.purpose = Purpose::Recovering(restore);
    }
    Ok(lease)
}
fn held<'a>(
    root: &PhysicalRoot,
    held: &Arc<CancellationToken>,
    deadline: Instant,
    state: &'a mut State,
) -> Result<&'a mut Active, RootRefusal> {
    if Instant::now() >= deadline {
        return Err(RootRefusal::TimedOut);
    }
    state
        .active
        .get_mut(root)
        .filter(|active| Arc::ptr_eq(&active.release, held) && !active.release.is_cancelled())
        .ok_or(RootRefusal::StaleLease)
}
pub(super) fn pin(
    root: PhysicalRoot,
    worktree: WorktreeId,
    restore: RestoreId,
    token: &Arc<CancellationToken>,
    deadline: Instant,
    state: &mut State,
) -> Result<(), RootRefusal> {
    let _lease = held(&root, token, deadline, state)?;
    state.recovering.add(root, worktree, restore)
}
pub(super) fn resolve(
    root: PhysicalRoot,
    restore: RestoreId,
    token: &Arc<CancellationToken>,
    deadline: Instant,
    state: &mut State,
) -> Result<(), RootRefusal> {
    let purpose = held(&root, token, deadline, state)?.purpose;
    match purpose {
        Purpose::Recovering(id) if id == restore => {}
        Purpose::Resolved(id) if id == restore => return Ok(()),
        Purpose::Writer if state.recovering.has(&root, &restore) => {}
        Purpose::Writer | Purpose::Recovering(_) | Purpose::Resolved(_) => {
            return Err(RootRefusal::StaleLease);
        }
    }
    state.recovering.settled(root, restore)?;
    if let Some(active) = state.active.get_mut(&root) {
        active.purpose = Purpose::Resolved(restore);
    }
    Ok(())
}
