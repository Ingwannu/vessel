//! Internal restarts obtain physical admission before any replacement can spawn.
//! The node retains the real root guard; these typed receipts convey no file I/O.
//!
//! [Decision Log]
//! - Purpose: every replacement must enter node lifecycle/physical admission.
//! - Constraint: process ownership lives here; the root/registry guards live on
//!   the node. A PID, cwd string or revoked lifetime cannot replace those guards.
//! - Alternatives: a cross-crate node lock would reverse the crate dependency;
//!   releasing on timeout/sender loss could admit a writer during real spawn.
//! - Choice: a bounded request and typestate pre-spawn barrier, with last-state
//!   observation. Deadline bounds admission only; Starting abandonment is unknown.
//! - Cost: the node must retain unknown claims and bind its canonical registry
//!   start lease before granting. Registered is ownership, never cleanup proof.
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::Instant,
};
use vessel_proto::SessionId;
/// Includes queue and root metadata/admission, never claims to cancel spawn I/O.
pub const START_ADMISSION_DEADLINE: Duration = Duration::from_secs(5);
const START_QUEUE: usize = 8;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// A replacement refused before it could become an untracked writer.
pub enum StartupRefusal {
    /// The node has no room for another admission request.
    #[error("agent startup admission is busy")]
    Busy,
    /// The same immutable request exhausted its admission budget.
    #[error("agent startup admission expired")]
    Expired,
    /// The actor's launch folder cannot be admitted.
    #[error("agent startup folder is not supported")]
    Folder,
    /// The node fenced this root for recovery or lifecycle work.
    #[error("agent startup root is unavailable")]
    Unavailable,
    /// An acknowledgement was lost; it is never permission to spawn.
    #[error("agent startup admission was abandoned")]
    Abandoned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Observable progress, bound to one request/channel, never a caller's PID.
pub enum StartupPhase {
    /// The actor has not crossed its pre-spawn barrier.
    Granted,
    /// Spawn may have started; sender loss must retain unknown writer exclusion.
    Starting,
    /// The replacement is owned and published in the existing actor/runtime.
    Registered,
    /// Synchronous spawn failed before any process was returned.
    NotStarted,
}
#[derive(Debug, Clone)]
/// Bounded node admission bridge for wake/move/fork/rewind replacement starts.
pub struct StartupHooks {
    requests: mpsc::Sender<StartupRequest>,
}
#[derive(Debug)]
/// An immutable launch request; node validation precedes granting its permit.
pub struct StartupRequest {
    session: SessionId,
    folder: PathBuf,
    deadline: Instant,
    reply: oneshot::Sender<Result<StartupPermit, StartupRefusal>>,
}
#[derive(Debug)]
/// The node observes this while retaining its actual startup/root leases.
pub struct StartupProgress {
    phase: watch::Receiver<StartupPhase>,
}
#[derive(Debug)]
pub(crate) struct StartupPermit {
    phase: watch::Sender<StartupPhase>,
}
#[derive(Debug)]
pub(crate) struct StartupAttempt {
    phase: watch::Sender<StartupPhase>,
}
impl StartupHooks {
    /// Create a bounded bridge; a full or closed queue refuses without spawning.
    #[must_use]
    pub fn channel() -> (Self, mpsc::Receiver<StartupRequest>) {
        let (requests, received) = mpsc::channel(START_QUEUE);
        (Self { requests }, received)
    }
    pub(crate) async fn admit(
        &self,
        session: SessionId,
        folder: PathBuf,
    ) -> Result<StartupPermit, StartupRefusal> {
        if !folder.is_absolute() {
            return Err(StartupRefusal::Folder);
        }
        let deadline = Instant::now()
            .checked_add(START_ADMISSION_DEADLINE)
            .ok_or(StartupRefusal::Expired)?;
        let (reply, received) = oneshot::channel();
        self.requests
            .try_send(StartupRequest {
                session,
                folder,
                deadline,
                reply,
            })
            .map_err(|_full_or_closed| StartupRefusal::Busy)?;
        let permit = tokio::time::timeout_at(deadline, received)
            .await
            .map_err(|_expired| StartupRefusal::Expired)?
            .map_err(|_gone| StartupRefusal::Abandoned)??;
        if Instant::now() >= deadline {
            return Err(StartupRefusal::Expired);
        }
        Ok(permit)
    }
}
impl StartupRequest {
    /// Runtime identity, independently of workspace and path identities.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
    /// Actual replacement cwd; not a wire-supplied authority or root lock.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }
    /// The original queue-inclusive budget, never renewed by the node.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
    /// Refuse before taking a new root lease for a caller that already left.
    #[must_use]
    pub fn abandoned(&self) -> bool {
        self.reply.is_closed() || Instant::now() >= self.deadline
    }
    /// Refuse a launch; no replacement process may follow this result.
    pub fn refuse(self, reason: StartupRefusal) {
        let _gone = self.reply.send(Err(reason));
    }
    /// Grant only while owning node lifecycle and physical startup admission.
    /// The node retains those leases until safe progress, never timer expiry.
    /// # Errors
    /// Expired/lost actor; a dropped granted permit has not begun spawn.
    pub fn grant(self) -> Result<StartupProgress, StartupRefusal> {
        if self.abandoned() {
            return Err(StartupRefusal::Expired);
        }
        let (phase, received) = watch::channel(StartupPhase::Granted);
        self.reply
            .send(Ok(StartupPermit { phase }))
            .map_err(|_gone| StartupRefusal::Abandoned)?;
        Ok(StartupProgress { phase: received })
    }
}
impl StartupProgress {
    /// Last progress remains evidence after sender loss; Starting stays unknown.
    #[must_use]
    pub fn phase(&self) -> StartupPhase {
        *self.phase.borrow()
    }
    /// Await actual actor progress; a closed sender is explicit abandonment.
    /// # Errors
    /// Sender loss says nothing about process/descendant I/O completion.
    pub async fn changed(&mut self) -> Result<StartupPhase, StartupRefusal> {
        self.phase
            .changed()
            .await
            .map_err(|_gone| StartupRefusal::Abandoned)?;
        Ok(*self.phase.borrow_and_update())
    }
}
impl StartupPermit {
    pub(crate) fn begin(self) -> Result<StartupAttempt, StartupRefusal> {
        self.phase
            .send(StartupPhase::Starting)
            .map_err(|_gone| StartupRefusal::Abandoned)?;
        Ok(StartupAttempt { phase: self.phase })
    }
}
impl StartupAttempt {
    pub(crate) fn registered(self) {
        let _gone = self.phase.send(StartupPhase::Registered);
    }
    pub(crate) fn not_started(self) {
        let _gone = self.phase.send(StartupPhase::NotStarted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn admitted() -> (StartupPermit, StartupProgress) {
        let (hooks, mut requests) = StartupHooks::channel();
        let actor =
            tokio::spawn(
                async move { hooks.admit(SessionId::random(), std::env::temp_dir()).await },
            );
        let request = requests.recv().await.unwrap();
        let progress = request.grant().unwrap();
        (actor.await.unwrap().unwrap(), progress)
    }
    #[tokio::test]
    async fn checkpoint_start_admission_lost_permits_distinguish_never_begun_and_unknown_spawn() {
        let (permit, mut progress) = admitted().await;
        drop(permit);
        assert_eq!(progress.changed().await, Err(StartupRefusal::Abandoned));
        assert_eq!(progress.phase(), StartupPhase::Granted);
        let (permit, mut progress) = admitted().await;
        let attempt = permit.begin().unwrap();
        drop(attempt);
        assert_eq!(progress.changed().await.unwrap(), StartupPhase::Starting);
        assert_eq!(progress.changed().await, Err(StartupRefusal::Abandoned));
        assert_eq!(
            progress.phase(),
            StartupPhase::Starting,
            "sender loss cannot become a cleanup proof"
        );
    }
    #[tokio::test]
    async fn checkpoint_start_admission_requires_a_live_node_before_spawn_and_retains_terminal_progress()
     {
        let (permit, progress) = admitted().await;
        drop(progress);
        assert!(matches!(permit.begin(), Err(StartupRefusal::Abandoned)));
        for terminal in [StartupPhase::Registered, StartupPhase::NotStarted] {
            let (permit, mut progress) = admitted().await;
            let attempt = permit.begin().unwrap();
            match terminal {
                StartupPhase::Registered => attempt.registered(),
                StartupPhase::NotStarted => attempt.not_started(),
                StartupPhase::Granted | StartupPhase::Starting => unreachable!(),
            }
            assert_eq!(progress.changed().await.unwrap(), terminal);
            assert_eq!(progress.changed().await, Err(StartupRefusal::Abandoned));
            assert_eq!(progress.phase(), terminal);
        }
    }
    #[tokio::test(start_paused = true)]
    async fn checkpoint_start_admission_deadline_includes_waiting_for_the_node() {
        let (hooks, mut requests) = StartupHooks::channel();
        let actor =
            tokio::spawn(
                async move { hooks.admit(SessionId::random(), std::env::temp_dir()).await },
            );
        let request = requests.recv().await.unwrap();
        tokio::time::advance(START_ADMISSION_DEADLINE).await;
        assert!(matches!(actor.await.unwrap(), Err(StartupRefusal::Expired)));
        assert!(matches!(request.grant(), Err(StartupRefusal::Expired)));
    }
}
