//! One physical-root admission owner, independent of workspace identity (ADR 0077).
//!
//! Lifecycle first takes its durable workspace claim, then this lease. No caller
//! waits for a workspace claim while holding a root. Restores and destructive
//! lifecycle work retain the lease through physical I/O and verified completion;
//! a deadline cannot cancel kernel I/O. Read-only captures may release their
//! common lease after invalidating publication, while their separate bounded
//! capture worker/root permit remains occupied until actual cleanup.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::{
    sync::{Notify, mpsc, oneshot},
    time::{Instant, timeout_at},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{CHECKPOINT_QUEUE, PhysicalRoot};

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
    #[error("workspace root has no pending recovery")]
    NotRecovering,
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
        deadline: Instant,
        reply: oneshot::Sender<Result<RootLease, RootRefusal>>,
    },
    Recovered {
        root: PhysicalRoot,
        deadline: Instant,
        reply: oneshot::Sender<Result<(), RootRefusal>>,
    },
}
#[derive(Debug, Clone)]
pub(crate) struct RootGate {
    requests: mpsc::Sender<Request>,
}
impl RootGate {
    /// Startup supplies every unresolved durable root before admitting any actor.
    pub(crate) fn start(
        mut recovering: BTreeSet<PhysicalRoot>,
    ) -> Result<(Self, tokio::task::JoinHandle<()>), RootRefusal> {
        if recovering.len() > ROOT_LEASES {
            return Err(RootRefusal::Busy);
        }
        let (requests, mut received) = mpsc::channel::<Request>(CHECKPOINT_QUEUE);
        let task = tokio::spawn(async move {
            let mut active = BTreeMap::new();
            let released = Arc::new(Notify::new());
            let mut accepting = true;
            loop {
                prune(&mut active);
                if !accepting && active.is_empty() {
                    break;
                }
                tokio::select! {biased;
                    ()=released.notified()=>{},
                    request=received.recv(), if accepting=>match request {
                        Some(request)=>admit(request,&mut active,&mut recovering,&released),
                        None=>accepting=false,
                    },
                    else=>break,
                }
            }
        });
        Ok((Self { requests }, task))
    }
    pub(crate) async fn claim(
        &self,
        root: PhysicalRoot,
        deadline: Instant,
    ) -> Result<RootLease, RootRefusal> {
        self.request(deadline, |reply| Request::Claim {
            root,
            deadline,
            reply,
        })
        .await
    }
    /// Called only after physical rollback and durable journal resolution verify.
    /// This is never an RPC or an unconditional force-unlock operation.
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
fn admit(
    request: Request,
    active: &mut BTreeMap<PhysicalRoot, CancellationToken>,
    recovering: &mut BTreeSet<PhysicalRoot>,
    notify: &Arc<Notify>,
) {
    prune(active);
    match request {
        Request::Claim {
            root,
            deadline,
            reply,
        } => {
            let result = if Instant::now() >= deadline {
                Err(RootRefusal::TimedOut)
            } else if recovering.contains(&root) {
                Err(RootRefusal::RecoveryPending)
            } else if active.contains_key(&root)
                || active.len().saturating_add(recovering.len()) >= ROOT_LEASES
            {
                Err(RootRefusal::Busy)
            } else {
                let release = CancellationToken::new();
                active.insert(root, release.clone());
                Ok(RootLease {
                    root,
                    release,
                    notify: Arc::clone(notify),
                })
            };
            // An undelivered lease drops here, notifying the owner without a fallible release queue.
            let _gone = reply.send(result);
        }
        Request::Recovered {
            root,
            deadline,
            reply,
        } => {
            let result = if Instant::now() >= deadline {
                Err(RootRefusal::TimedOut)
            } else if recovering.remove(&root) {
                Ok(())
            } else {
                Err(RootRefusal::NotRecovering)
            };
            let _gone = reply.send(result);
        }
    }
}
fn prune(active: &mut BTreeMap<PhysicalRoot, CancellationToken>) {
    active.retain(|_root, release| !release.is_cancelled());
}
#[cfg(test)]
mod tests;
