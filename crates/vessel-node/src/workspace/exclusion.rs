//! One bounded physical admission owner. No restore or terminal cleanup proof.
//! A lease releases only its own slot. Stop drains outstanding actual owners;
//! cancelling a caller never forcibly cancels its filesystem I/O.
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{
    sync::{Notify, mpsc, oneshot},
    time::{Instant, timeout_at},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::PhysicalRoot;
/// Bounds the admission actor acknowledgement, independently of filesystem I/O.
pub(super) const ADMISSION_TIMEOUT: Duration = Duration::from_secs(5);
const ADMISSION_QUEUE: usize = 8;
const ROOT_LEASES: usize = super::MAX_PROJECTS.saturating_add(super::MAX_WORKTREES);
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RootRefusal {
    #[error("workspace root is busy")]
    Busy,
    #[error("workspace admission deadline expired")]
    TimedOut,
    #[error("workspace admission owner is closed")]
    Closed,
}
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
struct Request {
    root: PhysicalRoot,
    deadline: Instant,
    reply: oneshot::Sender<Result<RootLease, RootRefusal>>,
}
#[derive(Debug, Clone)]
pub(super) struct RootGate {
    requests: mpsc::Sender<Request>,
    stop: CancellationToken,
}
impl RootGate {
    pub(super) fn start(stop: CancellationToken) -> (Self, tokio::task::JoinHandle<()>) {
        let (requests, mut received) = mpsc::channel::<Request>(ADMISSION_QUEUE);
        let shutdown = stop.clone();
        let owner = tokio::spawn(async move {
            let mut active = BTreeMap::new();
            let notify = Arc::new(Notify::new());
            let mut accepting = true;
            loop {
                prune(&mut active);
                if !accepting && active.is_empty() {
                    break;
                }
                tokio::select! { biased;
                    () = notify.notified() => {},
                    () = shutdown.cancelled(), if accepting => { received.close(); accepting = false; },
                    request = received.recv(), if accepting => match request {
                        Some(request) => {
                            let result = issue(request.root, request.deadline, &mut active, &notify);
                            let _gone = request.reply.send(result);
                        },
                        None => accepting = false,
                    },
                    else => break,
                }
            }
        });
        (Self { requests, stop }, owner)
    }
    pub(super) async fn claim(
        &self,
        root: PhysicalRoot,
        deadline: Instant,
    ) -> Result<RootLease, RootRefusal> {
        if self.stop.is_cancelled() {
            return Err(RootRefusal::Closed);
        }
        if Instant::now() >= deadline {
            return Err(RootRefusal::TimedOut);
        }
        let (reply, received) = oneshot::channel();
        self.requests
            .try_send(Request {
                root,
                deadline,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => RootRefusal::Busy,
                mpsc::error::TrySendError::Closed(_) => RootRefusal::Closed,
            })?;
        timeout_at(deadline, received)
            .await
            .map_err(|_expired| RootRefusal::TimedOut)?
            .map_err(|_gone| RootRefusal::Closed)?
    }
}
fn issue(
    root: PhysicalRoot,
    deadline: Instant,
    active: &mut BTreeMap<PhysicalRoot, Arc<CancellationToken>>,
    notify: &Arc<Notify>,
) -> Result<RootLease, RootRefusal> {
    prune(active);
    if Instant::now() >= deadline {
        return Err(RootRefusal::TimedOut);
    }
    if active.contains_key(&root) || active.len() >= ROOT_LEASES {
        return Err(RootRefusal::Busy);
    }
    let release = Arc::new(CancellationToken::new());
    let _old = active.insert(root, Arc::clone(&release));
    Ok(RootLease {
        root,
        release,
        notify: Arc::clone(notify),
    })
}
fn prune(active: &mut BTreeMap<PhysicalRoot, Arc<CancellationToken>>) {
    active.retain(|_root, release| !release.is_cancelled());
}
#[cfg(test)]
mod tests;
