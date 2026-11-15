//! One physical-root admission owner, independent of workspace identity (ADR 0077).
//!
//! Lifecycle first takes its durable workspace claim, then this lease. No caller
//! waits for a workspace claim while holding a root. Restores and destructive
//! lifecycle work retain the lease through physical I/O and verified completion;
//! a deadline cannot cancel kernel I/O. Read-only captures may release their
//! common lease after invalidating publication, while their separate bounded
//! capture worker/root permit remains occupied until actual cleanup.
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::{
    sync::{Notify, mpsc, oneshot},
    time::{Instant, timeout_at},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    WorktreeId,
    checkpoint::{CHECKPOINT_QUEUE, PhysicalRoot},
};

/// Bound a wedged admission owner without waiting on filesystem work.
#[cfg(target_os = "linux")]
pub(super) const ADMISSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Covers registered project and worktree roots, with aliases sharing one slot.
const ROOT_LEASES: usize = super::MAX_PROJECTS.saturating_add(super::MAX_WORKTREES);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RootRefusal {
    #[error("workspace root is busy")]
    Busy,
    #[error("workspace root needs recovery")]
    RecoveryPending,
    #[error("workspace admission deadline expired")]
    TimedOut,
    #[error("workspace admission owner is closed")]
    Closed,
    #[cfg(test)]
    #[error("workspace recovery lease is stale")]
    StaleLease,
    #[cfg(test)]
    #[error("workspace root has no pending recovery")]
    NotRecovering,
}
mod recovery;
#[cfg(test)]
mod resolving;
pub(crate) use recovery::RecoveryRoots;

#[derive(Debug)]
pub(crate) struct RootLease {
    root: PhysicalRoot,
    release: Arc<CancellationToken>,
    notify: Arc<Notify>,
}
impl RootLease {
    pub(crate) const fn root(&self) -> PhysicalRoot {
        self.root
    }
}
impl Drop for RootLease {
    fn drop(&mut self) {
        self.release.cancel();
        self.notify.notify_one();
    }
}
#[derive(Debug)]
enum Request {
    Claim {
        root: PhysicalRoot,
        worktrees: BTreeSet<WorktreeId>,
        deadline: Instant,
        reply: oneshot::Sender<Result<RootLease, RootRefusal>>,
    },
    #[cfg(test)]
    ClaimRecovery {
        root: PhysicalRoot,
        restore: vessel_proto::checkpoint::RestoreId,
        deadline: Instant,
        reply: oneshot::Sender<Result<RootLease, RootRefusal>>,
    },
    #[cfg(test)]
    PinRecovery {
        root: PhysicalRoot,
        worktree: WorktreeId,
        restore: vessel_proto::checkpoint::RestoreId,
        held: Arc<CancellationToken>,
        deadline: Instant,
        reply: oneshot::Sender<Result<(), RootRefusal>>,
    },
    #[cfg(test)]
    ResolveRecovery {
        root: PhysicalRoot,
        restore: vessel_proto::checkpoint::RestoreId,
        held: Arc<CancellationToken>,
        deadline: Instant,
        reply: oneshot::Sender<Result<(), RootRefusal>>,
    },
}
#[derive(Debug, Clone)]
pub(crate) struct RootGate {
    requests: mpsc::Sender<Request>,
    stop: CancellationToken,
}
impl RootGate {
    /// Startup supplies every unresolved durable root before admitting any actor.
    pub(crate) fn start(
        recovering: RecoveryRoots,
        stop: CancellationToken,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let shutdown = stop.clone();
        let (requests, mut received) = mpsc::channel::<Request>(CHECKPOINT_QUEUE);
        let task = tokio::spawn(async move {
            let mut state = State {
                active: BTreeMap::new(),
                recovering,
            };
            let released = Arc::new(Notify::new());
            let mut accepting = true;
            loop {
                prune(&mut state.active);
                if !accepting && state.active.is_empty() {
                    break;
                }
                tokio::select! {biased;
                    ()=released.notified()=>{},
                    ()=shutdown.cancelled(), if accepting=> {received.close();accepting=false;},
                    request=received.recv(), if accepting=>match request {
                        Some(request)=>admit(request,&mut state,&released),
                        None=>accepting=false,
                    },
                    else=>break,
                }
            }
        });
        (Self { requests, stop }, task)
    }
    #[cfg(test)]
    pub(crate) async fn claim(
        &self,
        root: PhysicalRoot,
        deadline: Instant,
    ) -> Result<RootLease, RootRefusal> {
        self.claim_registered(root, BTreeSet::new(), deadline).await
    }
    pub(super) async fn claim_registered(
        &self,
        root: PhysicalRoot,
        worktrees: BTreeSet<WorktreeId>,
        deadline: Instant,
    ) -> Result<RootLease, RootRefusal> {
        if worktrees.len() > super::MAX_WORKTREES {
            return Err(RootRefusal::Busy);
        }
        self.request(deadline, |reply| Request::Claim {
            root,
            worktrees,
            deadline,
            reply,
        })
        .await
    }
    async fn request<T>(
        &self,
        deadline: Instant,
        make: impl FnOnce(oneshot::Sender<Result<T, RootRefusal>>) -> Request,
    ) -> Result<T, RootRefusal> {
        if self.stop.is_cancelled() {
            return Err(RootRefusal::Closed);
        }
        if Instant::now() >= deadline {
            return Err(RootRefusal::TimedOut);
        }
        let (reply, received) = oneshot::channel();
        self.requests
            .try_send(make(reply))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => RootRefusal::Busy,
                mpsc::error::TrySendError::Closed(_) => RootRefusal::Closed,
            })?;
        timeout_at(deadline, received)
            .await
            .map_err(|_elapsed| RootRefusal::TimedOut)?
            .map_err(|_gone| RootRefusal::Closed)?
    }
}
#[derive(Debug)]
struct State {
    active: BTreeMap<PhysicalRoot, Active>,
    recovering: RecoveryRoots,
}

#[derive(Debug)]
struct Active {
    release: Arc<CancellationToken>,
    #[cfg(test)]
    purpose: resolving::Purpose,
}

fn admit(request: Request, state: &mut State, notify: &Arc<Notify>) {
    prune(&mut state.active);
    #[cfg(not(test))]
    {
        let Request::Claim {
            root,
            worktrees,
            deadline,
            reply,
        } = request;
        let _gone = reply.send(claim(root, &worktrees, deadline, state, notify));
    }
    #[cfg(test)]
    match request {
        Request::Claim {
            root,
            worktrees,
            deadline,
            reply,
        } => {
            let _gone = reply.send(claim(root, &worktrees, deadline, state, notify));
        }
        Request::ClaimRecovery {
            root,
            restore,
            deadline,
            reply,
        } => {
            let _gone = reply.send(resolving::claim(root, restore, deadline, state, notify));
        }
        Request::PinRecovery {
            root,
            worktree,
            restore,
            held,
            deadline,
            reply,
        } => {
            let _gone = reply.send(resolving::pin(
                root, worktree, restore, &held, deadline, state,
            ));
        }
        Request::ResolveRecovery {
            root,
            restore,
            held,
            deadline,
            reply,
        } => {
            let _gone = reply.send(resolving::resolve(root, restore, &held, deadline, state));
        }
    }
}
fn claim(
    root: PhysicalRoot,
    worktrees: &BTreeSet<WorktreeId>,
    deadline: Instant,
    state: &mut State,
    notify: &Arc<Notify>,
) -> Result<RootLease, RootRefusal> {
    if Instant::now() >= deadline {
        return Err(RootRefusal::TimedOut);
    }
    if state.recovering.contains(&root) || state.recovering.affects(worktrees) {
        return Err(RootRefusal::RecoveryPending);
    }
    issue(root, deadline, state, notify)
}
fn issue(
    root: PhysicalRoot,
    deadline: Instant,
    state: &mut State,
    notify: &Arc<Notify>,
) -> Result<RootLease, RootRefusal> {
    if Instant::now() >= deadline {
        return Err(RootRefusal::TimedOut);
    }
    if state.active.contains_key(&root)
        || state.active.len() >= ROOT_LEASES
        || (!state.recovering.contains(&root)
            && state.active.len().saturating_add(state.recovering.len()) >= ROOT_LEASES)
    {
        return Err(RootRefusal::Busy);
    }
    let release = Arc::new(CancellationToken::new());
    let _previous = state.active.insert(
        root,
        Active {
            release: Arc::clone(&release),
            #[cfg(test)]
            purpose: resolving::Purpose::Writer,
        },
    );
    Ok(RootLease {
        root,
        release,
        notify: Arc::clone(notify),
    })
}

fn prune(active: &mut BTreeMap<PhysicalRoot, Active>) {
    active.retain(|_root, active| !active.release.is_cancelled());
}
#[cfg(test)]
mod tests;
