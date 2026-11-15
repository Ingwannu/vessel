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
    #[error("workspace root has no pending recovery")]
    NotRecovering,
}
#[derive(Debug, Default)]
pub(crate) struct RecoveryRoots(BTreeMap<PhysicalRoot, BTreeSet<WorktreeId>>);
impl TryFrom<BTreeSet<PhysicalRoot>> for RecoveryRoots {
    type Error = RootRefusal;
    fn try_from(roots: BTreeSet<PhysicalRoot>) -> Result<Self, Self::Error> {
        if roots.len() > ROOT_LEASES {
            Err(RootRefusal::Busy)
        } else {
            Ok(Self(
                roots
                    .into_iter()
                    .map(|root| (root, BTreeSet::new()))
                    .collect(),
            ))
        }
    }
}
impl RecoveryRoots {
    pub(super) fn add(
        &mut self,
        root: PhysicalRoot,
        worktree: WorktreeId,
    ) -> Result<(), RootRefusal> {
        if !self.0.contains_key(&root) && self.0.len() >= ROOT_LEASES {
            return Err(RootRefusal::Busy);
        }
        let worktrees = self.0.entry(root).or_default();
        if !worktrees.contains(&worktree) && worktrees.len() >= super::MAX_WORKTREES {
            return Err(RootRefusal::Busy);
        }
        let _inserted = worktrees.insert(worktree);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct RootLease {
    root: PhysicalRoot,
    release: CancellationToken,
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
    Recovered {
        root: PhysicalRoot,
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
        let recovering = recovering.0;
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
    /// Called only after physical rollback and durable journal resolution verify.
    /// This is never an RPC or an unconditional force-unlock operation.
    #[cfg(test)]
    pub(crate) async fn recovered(
        &self,
        root: PhysicalRoot,
        deadline: Instant,
    ) -> Result<(), RootRefusal> {
        self.request(deadline, |reply| Request::Recovered {
            root,
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
    active: BTreeMap<PhysicalRoot, CancellationToken>,
    recovering: BTreeMap<PhysicalRoot, BTreeSet<WorktreeId>>,
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
        #[cfg(test)]
        Request::Recovered {
            root,
            deadline,
            reply,
        } => {
            let result = if Instant::now() >= deadline {
                Err(RootRefusal::TimedOut)
            } else if state.recovering.remove(&root).is_some() {
                Ok(())
            } else {
                Err(RootRefusal::NotRecovering)
            };
            let _gone = reply.send(result);
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
    if state.recovering.contains_key(&root)
        || state
            .recovering
            .values()
            .any(|pending| !pending.is_disjoint(worktrees))
    {
        return Err(RootRefusal::RecoveryPending);
    }
    if state.active.contains_key(&root)
        || state.active.len().saturating_add(state.recovering.len()) >= ROOT_LEASES
    {
        return Err(RootRefusal::Busy);
    }
    let release = CancellationToken::new();
    let _previous = state.active.insert(root, release.clone());
    Ok(RootLease {
        root,
        release,
        notify: Arc::clone(notify),
    })
}

fn prune(active: &mut BTreeMap<PhysicalRoot, CancellationToken>) {
    active.retain(|_root, release| !release.is_cancelled());
}
#[cfg(test)]
mod tests;
