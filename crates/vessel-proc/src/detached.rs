//! Observation after cleanup authority has been consumed; never signals or sweeps.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;

#[cfg(unix)]
use crate::ProcessGroup;
use crate::child::{ChildPid, os};
use crate::{Child, WaitError};

/// Detach was refused without consuming the caller's cleanup authority.
#[derive(Debug, thiserror::Error)]
#[error("child was launched without release capability")]
pub struct DetachRefused {
    child: Box<Child>,
}

impl DetachRefused {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child: Box::new(child),
        }
    }

    /// Recover the still-owned child for its ordinary stop/wait path.
    #[must_use]
    pub fn into_child(self) -> Child {
        *self.child
    }
}

/// Why a released child's direct exit could not be observed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DetachedError {
    /// The OS failed to reap the exact direct child handle.
    #[error("cannot wait for detached child")]
    Wait(#[source] Arc<WaitError>),
    /// The launch runtime ended before its explicit reaper completed.
    #[error("detached child reaper stopped before exit was observed")]
    ReaperStopped,
}

#[derive(Debug, Clone)]
enum State {
    Running,
    Exited(Option<i32>),
    Failed(Arc<WaitError>),
}

/// Private launch facts identify this unreaped child; none grants signal authority.
struct Identity {
    pid: ChildPid,
    started: Instant,
    #[cfg(unix)]
    group: ProcessGroup,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("DetachedLaunch");
        let _fields = debug
            .field("pid", &self.pid)
            .field("started", &self.started);
        #[cfg(unix)]
        let _group = debug.field("group", &self.group);
        debug.finish_non_exhaustive()
    }
}

/// A released child may outlive this handle. Its explicit reaper retains the
/// direct process/job handle and opaque launch facts while the runtime lives.
/// On Unix, exiting the launcher reparents the child; Windows retains its own
/// process/job lifetime and needs separate platform qualification.
#[derive(Debug, Clone)]
pub struct Detached {
    state: watch::Receiver<State>,
    identity: Arc<Identity>,
}

impl Detached {
    /// Whether two observers were created by the same consuming detach.
    #[must_use]
    pub fn same_launch(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }

    /// Observe the direct child's exit. Repeated waits return the retained result;
    /// cancelling a wait does not cancel the reaper or change process ownership.
    /// Callers that need a finite operation wrap this in their named deadline.
    ///
    /// # Errors
    /// The OS wait failed or the original launch runtime stopped prematurely.
    pub async fn wait(&mut self) -> Result<Option<i32>, DetachedError> {
        loop {
            match self.state.borrow_and_update().clone() {
                State::Running => {}
                State::Exited(code) => return Ok(code),
                State::Failed(error) => return Err(DetachedError::Wait(error)),
            }
            self.state
                .changed()
                .await
                .map_err(|_closed| DetachedError::ReaperStopped)?;
        }
    }
}

pub(crate) fn start(
    inner: Option<os::Inner>,
    pid: ChildPid,
    started: Instant,
    #[cfg(unix)] group: ProcessGroup,
    runtime: &tokio::runtime::Handle,
) -> Detached {
    let identity = Arc::new(Identity {
        pid,
        started,
        #[cfg(unix)]
        group,
    });
    let retained = Arc::clone(&identity);
    let (state, receiver) = watch::channel(State::Running);
    drop(runtime.spawn(async move {
        // Retain the exact OS child and its launch facts until it is reaped;
        // dropping every observer never becomes an implicit stop operation.
        if let Some(mut inner) = inner {
            let result = match os::wait_for_leader(&mut inner).await {
                Ok(status) => State::Exited(status.code()),
                Err(error) => State::Failed(Arc::new(WaitError::from(error))),
            };
            let _unobserved = state.send_replace(result);
        }
        drop(retained);
    }));
    Detached {
        state: receiver,
        identity,
    }
}
