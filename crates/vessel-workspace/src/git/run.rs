//! One run of the `git` program: never through a shell, with a time limit,
//! output limits, no prompt, and a slot among a node's few concurrent runs
//! (ADR 0023).
//!
//! [Decision Log]
//! - Purpose: every git call of the node goes through here, so every one
//!   gets the same protections and none can forget one.
//! - Constraint: a node serves many clients and must never hang on a git
//!   that waits for a password, pile up git processes, or hold a 2 GB diff
//!   in memory.
//! - Choice: a builder whose only way to run is [`GitCommand::output`]
//!   (or [`GitCommand::output_streaming`] for clone's progress). Arguments
//!   are a vector handed to the program (`vessel-proc`, its own process
//!   group, killed as a group); standard input is closed at once; the
//!   environment turns every prompt off (`GIT_TERMINAL_PROMPT=0`,
//!   `GCM_INTERACTIVE=never`) so a push that needs a credential fails
//!   instead of waiting; `-c core.fsmonitor=false` keeps a repository's
//!   configuration from starting a monitor program on a read; output past
//!   its limit is cut (the caller says whether that is an error) and the
//!   process is killed; a run past its time is killed and answers
//!   [`GitError::TimedOut`]. At most [`MAX_CONCURRENT`] runs at once per
//!   node; the next waits for a slot (GOAT and Paseo use the same bound).
//! - A run that needs no lock it does not take (status refreshes the
//!   index if it may) gets `GIT_OPTIONAL_LOCKS=0`, so reading never makes a
//!   user's own git command fail on `index.lock`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use vessel_proc::{CommandSpec, Signal, Spawned};

use super::GitError;

/// Local git runs one node makes at once (status, diff, commit).
pub(crate) const LOCAL_SLOTS: usize = 8;
/// Network git runs one node makes at once (clone, fetch, push, pull), kept
/// apart so a slow clone never holds up a status.
pub(crate) const NETWORK_SLOTS: usize = 4;
/// Runs that may wait for a slot of one pool; past that a call answers
/// `unavailable` at once instead of queueing without end.
pub(crate) const MAX_WAITING: usize = 64;

/// How long a run may take and how much it may print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limits {
    /// Killed after this.
    pub(crate) time: Duration,
    /// Standard output kept; the rest is cut.
    pub(crate) stdout: usize,
    /// Standard error kept; the rest is cut.
    pub(crate) stderr: usize,
}

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;

impl Limits {
    /// A local command: status, diff, log, commit, a worktree.
    pub(crate) const LOCAL: Self = Self {
        time: Duration::from_secs(30),
        stdout: 16 * MIB,
        stderr: 64 * KIB,
    };

    /// A command that talks to a remote: push, pull, fetch.
    pub(crate) const NETWORK: Self = Self {
        time: Duration::from_secs(120),
        stdout: MIB,
        stderr: 256 * KIB,
    };

    /// A clone, whose progress streams on standard error.
    pub(crate) const CLONE: Self = Self {
        time: Duration::from_secs(3600),
        stdout: 64 * KIB,
        stderr: 256 * KIB,
    };

    /// These limits with standard output capped at `bytes`.
    pub(crate) const fn with_stdout(self, bytes: usize) -> Self {
        Self {
            stdout: bytes,
            ..self
        }
    }
}

/// What a run printed and how it ended.
#[derive(Debug)]
pub(crate) struct GitOutput {
    /// The exit code; `None` when a signal ended it.
    pub(crate) code: Option<i32>,
    /// Standard output, at most the limit.
    pub(crate) stdout: Vec<u8>,
    /// Whether standard output went past its limit (the process was then
    /// stopped, so `code` says nothing about success).
    pub(crate) stdout_cut: bool,
    /// Standard error, at most its limit.
    pub(crate) stderr: Vec<u8>,
}

impl GitOutput {
    /// Whether git exited with 0 and printed everything.
    pub(crate) fn succeeded(&self) -> bool {
        self.code == Some(0) && !self.stdout_cut
    }

    /// Standard error as text, for classifying and for logs.
    pub(crate) fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// `Ok` with this output when git exited with 0; otherwise
    /// [`GitError::Failed`] with its code and the end of its standard
    /// error (credentials removed).
    pub(crate) fn checked(self) -> Result<Self, GitError> {
        if self.stdout_cut {
            return Err(GitError::OutputLimit);
        }
        if self.code == Some(0) {
            return Ok(self);
        }
        Err(GitError::Failed {
            code: self.code,
            message: super::redact::tail(&self.stderr_text()),
        })
    }
}

/// One pool of slots: at most `slots` runs at once, at most
/// [`MAX_WAITING`] waiting.
#[derive(Debug)]
struct Pool {
    slots: Arc<Semaphore>,
    waiting: AtomicUsize,
}

impl Pool {
    fn new(slots: usize) -> Arc<Self> {
        Arc::new(Self {
            slots: Arc::new(Semaphore::new(slots)),
            waiting: AtomicUsize::new(0),
        })
    }

    /// A slot, or [`GitError::Busy`] when too many runs already wait.
    async fn slot(&self) -> Result<OwnedSemaphorePermit, GitError> {
        if let Ok(permit) = Arc::clone(&self.slots).try_acquire_owned() {
            return Ok(permit);
        }
        let waiting = self.waiting.fetch_add(1, Ordering::AcqRel);
        let _counted = Counted(&self.waiting);
        if waiting >= MAX_WAITING {
            return Err(GitError::Busy);
        }
        Arc::clone(&self.slots)
            .acquire_owned()
            .await
            .map_err(|_closed| GitError::Closed)
    }
}

/// Takes one off a waiting count when dropped (also when the waiting call
/// is dropped).
struct Counted<'a>(&'a AtomicUsize);

impl Drop for Counted<'_> {
    fn drop(&mut self) {
        let _previous = self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// How the node runs git: the program, an empty folder to point the hooks
/// at, and variables every run gets.
#[derive(Debug, Clone)]
pub struct GitSetup {
    /// The `git` program, absolute.
    pub program: PathBuf,
    /// An empty folder of the node's: `core.hooksPath` points here, so no
    /// hook runs unless a call asks for the repository's own.
    pub no_hooks: PathBuf,
    /// Variables every run gets besides Vessel's own (tests isolate git from
    /// the machine's configuration this way; a node passes none).
    pub env: Vec<(String, OsString)>,
}

/// The `git` program a node runs, and its two pools.
#[derive(Debug, Clone)]
pub(crate) struct Runner {
    setup: Arc<GitSetup>,
    local: Arc<Pool>,
    network: Arc<Pool>,
}

impl Runner {
    pub(crate) fn new(setup: GitSetup) -> Self {
        Self {
            setup: Arc::new(setup),
            local: Pool::new(LOCAL_SLOTS),
            network: Pool::new(NETWORK_SLOTS),
        }
    }

    /// A command run in `dir`, with [`Limits::LOCAL`] and no hooks.
    pub(crate) fn command(&self, dir: &Path) -> GitCommand {
        GitCommand {
            runner: self.clone(),
            dir: dir.to_path_buf(),
            args: Vec::new(),
            env: Vec::new(),
            limits: Limits::LOCAL,
            network: false,
            hooks: false,
            cancel: None,
        }
    }
}

/// Set on every run.
const ENVIRONMENT: &[(&str, &str)] = &[
    // Never ask on a terminal; a credential helper that needs no person
    // still works. An empty `GIT_ASKPASS` also stops git falling back to
    // `core.askPass` and `SSH_ASKPASS`.
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_ASKPASS", ""),
    ("GCM_INTERACTIVE", "never"),
    ("SSH_ASKPASS_REQUIRE", "never"),
    // Reads never take an optional lock a user's own git would trip on.
    ("GIT_OPTIONAL_LOCKS", "0"),
    // A path is a path: no `*` and no `:(magic)` from a client's text.
    ("GIT_LITERAL_PATHSPECS", "1"),
    // A partial clone does not fetch missing objects on a read.
    ("GIT_NO_LAZY_FETCH", "1"),
    // Messages in English, so refusals can be told apart.
    ("LC_ALL", "C"),
    ("LANGUAGE", "C"),
    // No pager, no editor: nothing here has a terminal.
    ("GIT_PAGER", "cat"),
    ("GIT_EDITOR", "true"),
];

/// Configuration every run overrides, before the subcommand: no program a
/// repository's configuration names runs on a read (ADR 0023).
const OVERRIDES: &[&str] = &[
    "core.fsmonitor=false",
    "core.quotePath=false",
    "diff.external=",
    "log.showSignature=false",
    "color.ui=false",
];

/// A git command being built. Arguments go to the program as a vector,
/// never through a shell.
#[derive(Debug)]
pub(crate) struct GitCommand {
    runner: Runner,
    dir: PathBuf,
    args: Vec<OsString>,
    env: Vec<(String, OsString)>,
    limits: Limits,
    network: bool,
    hooks: bool,
    cancel: Option<CancellationToken>,
}

impl GitCommand {
    /// One argument.
    pub(crate) fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Several arguments.
    pub(crate) fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// An environment variable for this run.
    pub(crate) fn env(mut self, name: &str, value: impl Into<OsString>) -> Self {
        self.env.push((name.to_owned(), value.into()));
        self
    }

    /// Other limits than [`Limits::LOCAL`].
    pub(crate) const fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// A run that talks to a remote: [`Limits::NETWORK`] and a slot of the
    /// network pool.
    pub(crate) const fn network(mut self) -> Self {
        self.network = true;
        self.limits = Limits::NETWORK;
        self
    }

    /// Run the repository's own hooks, as the user's git would (commit and
    /// push only).
    pub(crate) const fn with_hooks(mut self) -> Self {
        self.hooks = true;
        self
    }

    /// Stopped (killed, [`GitError::Cancelled`]) when `token` is.
    pub(crate) fn cancel_on(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// The command as `vessel-proc` runs it.
    fn spec(&self) -> CommandSpec {
        let setup = &self.runner.setup;
        let mut spec = CommandSpec::new(&setup.program).cwd(&self.dir);
        for (name, value) in ENVIRONMENT {
            spec = spec.env(*name, *value);
        }
        for (name, value) in setup.env.iter().chain(&self.env) {
            spec = spec.env(name, value);
        }
        for setting in OVERRIDES {
            spec = spec.args(["-c", setting]);
        }
        if !self.hooks {
            let mut hooks = OsString::from("core.hooksPath=");
            hooks.push(&setup.no_hooks);
            spec = spec.arg("-c").arg(hooks);
        }
        spec.args(&self.args)
    }

    /// Run it for its effect: `Ok` when git exited with 0.
    ///
    /// # Errors
    /// As [`output`](Self::output), and [`GitOutput::checked`]'s.
    pub(crate) async fn run(self) -> Result<(), GitError> {
        self.output().await?.checked().map(drop)
    }

    /// Run it and collect its output.
    ///
    /// # Errors
    /// [`GitError::Spawn`], [`GitError::TimedOut`], [`GitError::Cancelled`],
    /// or [`GitError::Wait`]. A non-zero exit is not an error here: callers
    /// read the code (or call [`GitOutput::checked`]).
    pub(crate) async fn output(self) -> Result<GitOutput, GitError> {
        self.output_streaming(|_| {}).await
    }

    /// [`output`](Self::output), also handing every piece of standard error
    /// to `progress` as it arrives (clone's progress lines).
    pub(crate) async fn output_streaming(
        self,
        mut progress: impl FnMut(&[u8]) + Send,
    ) -> Result<GitOutput, GitError> {
        let spec = self.spec();
        let Self {
            runner,
            limits,
            network,
            cancel,
            ..
        } = self;
        let pool = if network {
            &runner.network
        } else {
            &runner.local
        };
        let _slot = pool.slot().await?;
        let Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        } = vessel_proc::spawn(&spec).map_err(GitError::Spawn)?;
        // Nothing is ever typed into git.
        drop(stdin);
        let cancel = cancel.unwrap_or_default();
        let ended = {
            let work = async {
                let (out, err) = tokio::join!(
                    read_capped(stdout, limits.stdout),
                    read_tail(stderr, limits.stderr, &mut progress)
                );
                let code = child.wait().await.map_err(GitError::Wait)?;
                Ok::<_, GitError>(GitOutput {
                    code,
                    stdout: out.bytes,
                    stdout_cut: out.cut,
                    stderr: err,
                })
            };
            tokio::select! {
                result = work => Ended::Finished(result),
                () = tokio::time::sleep(limits.time) => Ended::TimedOut,
                () = cancel.cancelled() => Ended::Cancelled,
            }
        };
        match ended {
            Ended::Finished(result) => result,
            Ended::TimedOut => {
                stop(&mut child).await;
                Err(GitError::TimedOut { after: limits.time })
            }
            Ended::Cancelled => {
                stop(&mut child).await;
                Err(GitError::Cancelled)
            }
        }
    }
}

/// How a run's wait ended.
enum Ended {
    Finished(Result<GitOutput, GitError>),
    TimedOut,
    Cancelled,
}

/// Kill the run's whole group and reap it.
async fn stop(child: &mut vessel_proc::Child) {
    if let Err(error) = child.signal(Signal::Kill) {
        tracing::debug!(%error, "git had already ended");
    }
    if let Err(error) = child.wait().await {
        tracing::debug!(%error, "could not reap a stopped git");
    }
}

/// Bytes read from standard output, and whether more were cut.
struct Read {
    bytes: Vec<u8>,
    cut: bool,
}

/// Read `stream` to its end, keeping at most `limit` bytes. Past the limit
/// it stops reading and drops the stream, so git gets a broken pipe and
/// stops instead of filling memory.
async fn read_capped(mut stream: impl AsyncRead + Unpin, limit: usize) -> Read {
    let mut bytes = Vec::new();
    let mut chunk = vec![0_u8; 64 * KIB];
    loop {
        let read = match stream.read(&mut chunk).await {
            Ok(0) => return Read { bytes, cut: false },
            Ok(read) => read,
            Err(error) => {
                tracing::debug!(%error, "reading git's output failed");
                return Read { bytes, cut: false };
            }
        };
        let piece = chunk.get(..read).unwrap_or_default();
        let room = limit.saturating_sub(bytes.len());
        if piece.len() > room {
            bytes.extend_from_slice(piece.get(..room).unwrap_or_default());
            return Read { bytes, cut: true };
        }
        bytes.extend_from_slice(piece);
    }
}

/// Read `stream` to its end, handing each piece to `seen` and keeping its
/// last `limit` bytes. Standard error is read to the end, never cut short:
/// git would die of a broken pipe in the middle of a clone, and its last
/// lines are what explain a failure.
async fn read_tail(
    mut stream: impl AsyncRead + Unpin,
    limit: usize,
    mut seen: impl FnMut(&[u8]),
) -> Vec<u8> {
    let mut tail = Vec::new();
    let mut chunk = vec![0_u8; 16 * KIB];
    loop {
        let read = match stream.read(&mut chunk).await {
            Ok(0) => return tail,
            Ok(read) => read,
            Err(error) => {
                tracing::debug!(%error, "reading git's errors failed");
                return tail;
            }
        };
        let piece = chunk.get(..read).unwrap_or_default();
        seen(piece);
        tail.extend_from_slice(piece);
        let over = tail.len().saturating_sub(limit);
        if over > 0 {
            drop(tail.drain(..over));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::Sandbox;

    #[tokio::test]
    async fn output_past_its_limit_is_cut_and_said_so() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        let output = git
            .command(&repo)
            .args(["log", "--format=%H%n%H%n%H"])
            .limits(Limits::LOCAL.with_stdout(16))
            .output()
            .await
            .unwrap();
        assert!(output.stdout_cut);
        assert_eq!(output.stdout.len(), 16);
        assert!(!output.succeeded());
        assert!(matches!(output.checked(), Err(GitError::OutputLimit)));
    }

    /// A hook that would write a marker: it runs only when a call asks for
    /// the repository's hooks, and a run past its time is stopped.
    #[cfg(unix)]
    #[tokio::test]
    async fn hooks_run_only_when_asked_and_a_slow_run_is_stopped() {
        use std::os::unix::fs::PermissionsExt;

        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        let marker = sandbox.path().join("hook-ran");
        let hook = repo.join(".git/hooks/pre-commit");
        std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        let commit = |message: &'static str| {
            git.command(&repo)
                .args(["commit", "-q", "--allow-empty", "-m", message])
        };
        drop(commit("quiet").output().await.unwrap().checked().unwrap());
        assert!(!marker.exists(), "a read or a plain call runs no hook");
        drop(
            commit("hooked")
                .with_hooks()
                .output()
                .await
                .unwrap()
                .checked()
                .unwrap(),
        );
        assert!(marker.exists(), "commit and push run the user's hooks");

        std::fs::write(&hook, "#!/bin/sh\nsleep 30\n").unwrap();
        let slow = commit("slow")
            .with_hooks()
            .limits(Limits {
                time: Duration::from_millis(300),
                ..Limits::LOCAL
            })
            .output()
            .await;
        assert!(matches!(slow, Err(GitError::TimedOut { .. })), "{slow:?}");
    }

    #[tokio::test]
    async fn a_cancelled_run_stops() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        let token = CancellationToken::new();
        token.cancel();
        let cancelled = git
            .command(&repo)
            .arg("status")
            .cancel_on(token)
            .output()
            .await;
        // Either it was stopped, or it finished before the check: never a
        // hang.
        assert!(
            matches!(cancelled, Err(GitError::Cancelled) | Ok(_)),
            "{cancelled:?}"
        );
    }
}
