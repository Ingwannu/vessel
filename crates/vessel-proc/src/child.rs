//! [`spawn`] and [`Child`]: a piped child that is stopped as a group.

use std::num::NonZeroU32;
use std::process::Stdio;

use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

use crate::checked::{CheckedWaitError, FirstSweep, OwnedGroupExit};
use crate::error::{SignalError, SpawnError, Stream, WaitError};
#[cfg(unix)]
use crate::group::ProcessGroup;
use crate::signal::Signal;
use crate::spec::CommandSpec;
use crate::{ProcessId, ProcessLifetime, ProcessLifetimeGuard};

/// How often a refused signal looks again whether the leader has exited
/// (`Child::reaped_within`).
#[cfg(unix)]
const REAP_POLL: std::time::Duration = std::time::Duration::from_millis(5);

/// A started child and its three pipes.
#[derive(Debug)]
pub struct Spawned {
    /// The process.
    pub child: Child,
    /// Its standard input.
    pub stdin: ChildStdin,
    /// Its standard output.
    pub stdout: ChildStdout,
    /// Its standard error.
    pub stderr: ChildStderr,
}

/// Start `spec` with piped stdio, as the leader of a new process group (a
/// new job object on Windows).
///
/// # Errors
/// Fails if the program cannot be started (on Windows also if its job
/// object cannot be set up).
pub fn spawn(spec: &CommandSpec) -> Result<Spawned, SpawnError> {
    spawn_with(spec, Lifetime::Bound)
}

/// Start an owned child whose cleanup authority can later be explicitly released.
/// Until detach, dropping the returned Child still kills its whole group/job.
/// On Windows its job deliberately lacks kill-on-close, so the child may survive
/// abrupt launcher exit. Ordinary spawn retains the previous kill-on-close policy.
///
/// # Errors
/// The same launch and pipe failures as spawn.
pub fn spawn_releasable(spec: &CommandSpec) -> Result<Spawned, SpawnError> {
    spawn_with(spec, Lifetime::Releasable)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifetime {
    Bound,
    Releasable,
}

fn spawn_with(spec: &CommandSpec, lifetime: Lifetime) -> Result<Spawned, SpawnError> {
    spawn_command(spec, lifetime).map_err(|error| error.with_diagnostics(spec))
}

fn spawn_command(spec: &CommandSpec, lifetime: Lifetime) -> Result<Spawned, SpawnError> {
    let reaper =
        tokio::runtime::Handle::try_current().map_err(|source| SpawnError::Runtime { source })?;
    let mut command = tokio::process::Command::from(spec.to_command());
    let _ = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Cleanup is owned by Child, not the platform wrapper: consuming
        // detach must be able to release the same job without killing it.
        .kill_on_drop(false);

    let program = || spec.program.clone();
    let inner = os::spawn(command, lifetime).map_err(|source| SpawnError::Start {
        program: program(),
        source,
    })?;
    let Some(pid) = inner.id().and_then(NonZeroU32::new) else {
        discard_start(inner, &reaper);
        return Err(SpawnError::NoPid { program: program() });
    };
    let observation = ProcessLifetimeGuard::begin(ProcessId::from(pid));
    let pid = pid.get();
    #[cfg(unix)]
    let group = match ProcessGroup::of_leader(pid) {
        Ok(group) => group,
        Err(source) => {
            discard_start(inner, &reaper);
            return Err(SpawnError::NotLeader {
                program: program(),
                source,
            });
        }
    };
    let mut child = Child {
        inner: Some(inner),
        pid,
        observation,
        #[cfg(unix)]
        group,
        state: State::Running,
        first_sweep: FirstSweep::default(),
        reaper,
        lifetime,
        started: std::time::Instant::now(),
    };
    let (stdin, stdout, stderr) = match child.inner.as_mut() {
        Some(inner) => os::pipes(inner),
        None => (None, None, None),
    };
    let missing = |stream| SpawnError::MissingPipe {
        program: program(),
        stream,
    };
    let stdin = stdin.ok_or_else(|| missing(Stream::Stdin))?;
    let stdout = stdout.ok_or_else(|| missing(Stream::Stdout))?;
    let stderr = stderr.ok_or_else(|| missing(Stream::Stderr))?;
    tracing::debug!(pid, program = %spec.diagnostic_program().display(), "spawned");

    Ok(Spawned {
        child,
        stdin,
        stdout,
        stderr,
    })
}

/// The direct OS handle is retained even if group identity could not be built.
fn discard_start(mut inner: os::Inner, reaper: &tokio::runtime::Handle) {
    if let Err(error) = inner.start_kill() {
        tracing::warn!(%error, "cannot stop a failed process launch");
    }
    drop(reaper.spawn(async move {
        if let Err(error) = os::wait_for_leader(&mut inner).await {
            tracing::warn!(%error, "cannot reap a failed process launch");
        }
    }));
}

/// A running child that leads its own process group (on Windows: the first
/// process of its own job object).
///
/// Dropping a `Child` that has not been waited for kills its whole group, so
/// an early return or a cancelled task cannot leak processes. On Windows
/// dropping it closes the job, which kills every process in it.
#[derive(Debug)]
pub struct Child {
    inner: Option<os::Inner>,
    pid: u32,
    observation: ProcessLifetimeGuard,
    #[cfg(unix)]
    group: ProcessGroup,
    state: State,
    first_sweep: FirstSweep,
    reaper: tokio::runtime::Handle,
    lifetime: Lifetime,
    started: std::time::Instant,
}

/// Minted only from the OS handle of this module's newly spawned child.
#[derive(Clone, Copy)]
pub(crate) struct ChildPid(u32);

impl std::fmt::Debug for ChildPid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChildPid").field(&self.0).finish()
    }
}

/// Whether the child has been reaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Running,
    /// Reaped with this exit code (`None` when ended by a signal).
    Reaped(Option<i32>),
    /// Cleanup authority was consumed into an observation-only reaper.
    Released,
}

impl Child {
    /// Release cleanup authority while retaining observation and direct-child reaping.
    ///
    /// The original Tokio runtime must live until this child exits or the
    /// launcher process ends. The returned handle never signals or sweeps a group.
    ///
    /// # Errors
    /// A child started without release capability is returned still owned.
    pub fn detach(mut self) -> Result<crate::Detached, crate::DetachRefused> {
        if self.lifetime == Lifetime::Bound {
            return Err(crate::DetachRefused::new(self));
        }
        self.observation.revoke();
        self.state = State::Released;
        Ok(crate::detached::start(
            self.inner.take(),
            ChildPid(self.pid),
            self.started,
            #[cfg(unix)]
            self.group,
            &self.reaper,
        ))
    }

    /// The child's process id (also its group id on Unix).
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Observe this actual run without gaining signalling or revocation authority.
    #[must_use]
    pub fn lifetime(&self) -> ProcessLifetime {
        self.observation.observer()
    }

    /// Ask the child's process group to stop.
    ///
    /// On Unix this signals every process in the group. Windows has no
    /// polite signal for a windowless process, so every signal terminates
    /// the child's job: the child and everything it started (through a
    /// `.cmd` shim, the shim's `cmd.exe`, the real harness and its children).
    /// After the child has been reaped this does nothing:
    /// [`wait`](Self::wait) already swept the group.
    ///
    /// # Errors
    /// Fails if the OS refuses to deliver the signal.
    pub fn signal(&mut self, signal: Signal) -> Result<(), SignalError> {
        self.observation.revoke();
        match self.state {
            State::Running => {}
            State::Reaped(_) | State::Released => return Ok(()),
        }
        #[cfg(unix)]
        {
            let group = self.group;
            let result = group.signal_with_leader(signal, |grace| self.reaped_within(grace));
            if matches!(self.state, State::Reaped(_)) {
                // The refusal path already performed the first post-reap sweep.
                // Cache it now; wait_checked must not signal a potentially reused
                // numeric group id later merely to manufacture a receipt.
                let _first = self.first_sweep.record(|| result.clone());
            }
            result
        }
        #[cfg(windows)]
        {
            let _ = signal;
            match self.inner.as_mut() {
                Some(inner) => inner.start_kill().map_err(SignalError::from),
                None => Ok(()),
            }
        }
    }

    /// Whether the leader has exited, looking for up to `grace`: if so it is
    /// reaped now, its code kept for [`wait`](Self::wait) (which would reap
    /// it next anyway). For a refused group signal
    /// ([`ProcessGroup::signal_with_leader`]); blocks, polling `try_wait`,
    /// which never blocks and succeeds only for an exited leader (`waitid`
    /// with `WNOWAIT`, which would not reap, is missing on macOS).
    #[cfg(unix)]
    fn reaped_within(&mut self, grace: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now().checked_add(grace);
        loop {
            let Some(inner) = self.inner.as_mut() else {
                return false;
            };
            match inner.try_wait() {
                Ok(Some(status)) => {
                    self.state = State::Reaped(status.code());
                    return true;
                }
                Ok(None) => {}
                Err(_) => return false,
            }
            if deadline.is_none_or(|deadline| std::time::Instant::now() >= deadline) {
                return false;
            }
            std::thread::sleep(REAP_POLL);
        }
    }

    /// Wait for the child to exit, then kill whatever is left of its group.
    ///
    /// Returns the exit code, or `None` if the child was ended by a signal.
    /// Cancel-safe: if the future is dropped early nothing is lost, and a
    /// later call waits again. Once the child has exited, further calls
    /// return the same code immediately.
    ///
    /// # Errors
    /// Fails if the OS cannot report the child's status.
    pub async fn wait(&mut self) -> Result<Option<i32>, WaitError> {
        match self.state {
            State::Reaped(code) => return Ok(code),
            State::Running => {}
            State::Released => {
                return Err(WaitError::from(std::io::Error::other(
                    "child ownership released",
                )));
            }
        }
        let Some(inner) = self.inner.as_mut() else {
            self.observation.revoke();
            return Err(WaitError::from(std::io::Error::other(
                "child handle absent",
            )));
        };
        let status = match os::wait_for_leader(inner).await {
            Ok(status) => status,
            Err(error) => {
                self.observation.revoke();
                return Err(error.into());
            }
        };
        self.observation.revoke();
        let code = status.code();
        self.state = State::Reaped(code);
        self.sweep();
        Ok(code)
    }

    /// Reap the owned leader and report the original group/job cleanup outcome.
    /// Legacy wait callers keep their existing leader-status behavior. A prior
    /// wait or signal-triggered reap reuses its first recorded outcome without
    /// signalling a delayed numeric group id. Cancellation preserves ownership.
    ///
    /// The receipt means leader reaped plus an accepted cleanup request (or an
    /// already absent group/job), not every descendant's observed OS death.
    ///
    /// # Errors
    /// Preserves the concrete wait failure or first sweep failure; never turns
    /// missing cleanup evidence into success or retries a possibly reused group.
    pub async fn wait_checked(&mut self) -> Result<OwnedGroupExit, CheckedWaitError> {
        let code = self.wait().await.map_err(CheckedWaitError::Wait)?;
        self.first_sweep.receipt(code, self.lifetime())
    }

    /// Kill leftovers of the group after the leader exited, so a harness's
    /// own children (MCP servers, shells) never outlive it.
    fn sweep(&mut self) {
        #[cfg(unix)]
        let swept = self.first_sweep.record(|| self.group.signal(Signal::Kill));
        #[cfg(windows)]
        let swept = self.first_sweep.record(|| match self.inner.as_mut() {
            Some(inner) => inner.start_kill().map_err(SignalError::from),
            None => Ok(()),
        });
        if let Err(error) = swept {
            tracing::warn!(pid = self.pid, %error, "cannot sweep process group");
        }
    }
}

/// Unix: kill the whole group of a child dropped before it was reaped; the
/// runtime's orphan queue then reaps the leader. (On Windows dropping the
/// job handle also kills a normally bound child. A releasable job receives
/// the explicit owned-handle stop before its handle closes.)
impl Drop for Child {
    fn drop(&mut self) {
        self.observation.revoke();
        if self.state == State::Running {
            #[cfg(unix)]
            let stopped = {
                let group = self.group;
                group.signal_with_leader(Signal::Kill, |grace| self.reaped_within(grace))
            };
            #[cfg(windows)]
            let stopped = match self.inner.as_mut() {
                Some(inner) => inner.start_kill().map_err(SignalError::from),
                None => Ok(()),
            };
            if let Err(error) = stopped {
                tracing::warn!(pid = self.pid, %error, "cannot kill dropped process group");
            }
        }
    }
}

#[cfg(test)]
#[path = "child/checked_tests.rs"]
mod checked_tests;

/// What differs between Unix process groups and Windows job objects.
///
/// [Decision Log]
/// - Purpose: on Windows, npm installs `claude-agent-acp` and `codex-acp` as
///   `.cmd` shims, so the direct child is `cmd.exe` and the adapter is its
///   child. Terminating the direct child left the adapter running (spending
///   API credit mid-turn) and holding the pipes, so a session never ended.
/// - Alternatives: assign the running child to a job after spawning it
///   (`win32job`; a child that starts its own children within the first
///   milliseconds escapes, and closing that race needs `unsafe` thread
///   resumption); our own Win32 FFI (the workspace forbids `unsafe`).
/// - Choice: `process-wrap`'s `JobObject` and `KillOnDrop`: the child
///   starts suspended, joins a new kill-on-close job, then resumes, so
///   nothing escapes, and closing the job (dropping the [`Child`]) kills
///   all of it.
/// - Constraint: `process-wrap`'s own `wait` also waits for every process
///   left in the job, which could block forever on a lingering grandchild.
///   [`wait_for_leader`] waits for the child itself, as on Unix, and
///   [`Child::sweep`] then terminates the job.
#[cfg(unix)]
pub(crate) mod os {
    use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

    pub(crate) type Inner = tokio::process::Child;

    pub(super) fn spawn(
        mut command: tokio::process::Command,
        _lifetime: super::Lifetime,
    ) -> std::io::Result<Inner> {
        let _ = command.process_group(0);
        command.spawn()
    }

    pub(super) const fn pipes(
        inner: &mut Inner,
    ) -> (Option<ChildStdin>, Option<ChildStdout>, Option<ChildStderr>) {
        (inner.stdin.take(), inner.stdout.take(), inner.stderr.take())
    }

    pub(crate) async fn wait_for_leader(
        inner: &mut Inner,
    ) -> std::io::Result<std::process::ExitStatus> {
        inner.wait().await
    }
}

/// See the Unix `os` module for the Decision Log.
#[cfg(windows)]
pub(crate) mod os {
    use process_wrap::tokio::{ChildWrapper, CommandWrap, JobObject, KillOnDrop};
    use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

    pub(crate) type Inner = Box<dyn ChildWrapper>;

    pub(super) fn spawn(
        command: tokio::process::Command,
        lifetime: super::Lifetime,
    ) -> std::io::Result<Inner> {
        let mut wrapped = CommandWrap::from(command);
        let _ = wrapped.wrap(JobObject);
        match lifetime {
            super::Lifetime::Bound => {
                let _wrapped = wrapped.wrap(KillOnDrop);
            }
            super::Lifetime::Releasable => {}
        }
        wrapped.spawn()
    }

    pub(super) fn pipes(
        inner: &mut Inner,
    ) -> (Option<ChildStdin>, Option<ChildStdout>, Option<ChildStderr>) {
        (
            inner.stdin().take(),
            inner.stdout().take(),
            inner.stderr().take(),
        )
    }

    /// The child itself, not the whole job: `JobObject` wraps the plain
    /// child directly (`KillOnDrop` only sets a flag), so its inner wrapper
    /// is the process we started.
    pub(crate) async fn wait_for_leader(
        inner: &mut Inner,
    ) -> std::io::Result<std::process::ExitStatus> {
        inner.inner_mut().wait().await
    }
}
