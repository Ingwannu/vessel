//! A run of a project's script or service: its actor and its start.
//!
//! The command runs piped through `vessel_proc::spawn` (its own process
//! group on Unix, a kill-on-close job object on Windows), standard input
//! closed; standard output and error are merged into the run's log as
//! they arrive. A service is started again when it fails ([`Backoff`]),
//! after asking the node whether its command is still approved.

use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::Instrument as _;
use vessel_proc::{Child, CommandSpec, Signal, Spawned};
use vessel_proto::SessionStatus;
use vessel_proto::terminals::{ServiceFailure, ServiceState, TerminalInfo};

use super::backoff::{Backoff, Retry};
use super::cleanup::{self, Facts};
use super::handle::{Shared, TerminalHandle};
use super::log::TermLog;
use super::{CleanupFact, Leftover, SCRIPT_GRACE, Scrollback};
use crate::ring::Ring;
use crate::stop::{Step, Stopper};

/// Chunks of output waiting for the actor: the pumps wait when it is full,
/// and the command then waits on its pipe. Nothing is dropped.
const OUTPUT_QUEUE: usize = 16;

mod output;
use output::{Drained, Pumps};

/// What a run starts with.
#[derive(Debug)]
pub struct ScriptStart<A> {
    /// Its info as it starts (kind `script` or `service`).
    pub info: TerminalInfo,
    /// The command through the platform's shell, its folder and its
    /// environment.
    pub spec: CommandSpec,
    /// A file to remove once the run has ended for good.
    pub leftover: Leftover,
    /// Kept running (a service) or run once (a script).
    pub service: bool,
    /// Asked before every start after the first: whether the set the
    /// command came from is still approved.
    pub approved: A,
    /// How much of its log is kept.
    pub scrollback: Scrollback,
    /// The node's shutdown.
    pub shutdown: CancellationToken,
    /// The node's tasks.
    pub tasks: TaskTracker,
}

/// Start a run. It answers at once: a command that cannot start ends the
/// run `failed` (a service backs off and tries again).
#[must_use]
pub fn start_script<A>(start: ScriptStart<A>) -> TerminalHandle
where
    A: Fn() -> bool + Send + Sync + 'static,
{
    let ScriptStart {
        info,
        spec,
        leftover,
        service,
        approved,
        scrollback,
        shutdown,
        tasks,
    } = start;
    let id = info.id;
    let (completion, cleanup) = cleanup::begin(id);
    let ring = Ring::bounded(scrollback.records, scrollback.bytes, info.next_seq);
    let kill = shutdown.child_token();
    // One restart at a time: a second one while the first is taken waits.
    let (restarts, restarting) = mpsc::channel(1);
    let shared = Arc::new(Shared {
        cleanup,
        info: watch::Sender::new(info),
        ring: Mutex::new(ring),
        input: None,
        kill: kill.clone(),
        restarts: Some(restarts),
        pid: AtomicU32::new(0),
        process: Mutex::new(None),
    });
    let actor = ScriptActor {
        log: TermLog::new(Arc::clone(&shared)),
        stop: Stopper::with_grace(kill, shutdown.clone(), SCRIPT_GRACE),
        spec,
        leftover,
        service,
        approved,
        restarting,
        attempted: false,
        process_cleanup: CleanupFact::Confirmed,
        io_cleanup: CleanupFact::Confirmed,
    };
    drop(
        tasks.spawn(
            async move {
                completion.finish(actor.run().await);
            }
            .instrument(tracing::info_span!("terminal", terminal = %id)),
        ),
    );
    TerminalHandle(shared)
}

/// Owns a run for its life.
struct ScriptActor<A> {
    log: TermLog,
    stop: Stopper,
    spec: CommandSpec,
    leftover: Leftover,
    service: bool,
    approved: A,
    restarting: mpsc::Receiver<()>,
    /// Once a process existed, status alone cannot prove descendant/I/O cleanup.
    attempted: bool,
    process_cleanup: CleanupFact,
    io_cleanup: CleanupFact,
}

/// How one process's run ended.
enum Ended {
    /// It exited, with this code (`None`: a signal).
    Exited(Option<i32>),
    /// It could not start.
    Spawn,
    /// It was stopped: by a kill or the node's stop.
    Stopped(Option<i32>),
    /// A person restarted it.
    Restart,
}

impl<A: Fn() -> bool> ScriptActor<A> {
    async fn run(mut self) -> Facts {
        self.log.status(SessionStatus::Starting);
        let mut backoff = Backoff::default();
        let mut attempt = 1_u32;
        let mut first = true;
        let status = loop {
            // Asked before every start, the first and a person's restart
            // included: an approval withdrawn meanwhile runs nothing more.
            if !(self.approved)() {
                let state = ServiceState::Failed {
                    reason: ServiceFailure::NotApproved,
                };
                let message = state.summary();
                if self.service {
                    self.log.service(state);
                }
                break SessionStatus::failed(message);
            }
            if self.service {
                self.log.service(ServiceState::Starting { attempt });
            }
            let started = Instant::now();
            let ended = self.once(&mut first, attempt).await;
            match ended {
                Ended::Restart => {
                    backoff.reset();
                    attempt = 1;
                    self.log.notice("vessel: restarted".to_owned());
                    continue;
                }
                Ended::Stopped(code) => {
                    if self.service {
                        self.log.service(ServiceState::Stopped);
                    }
                    break self.stopped(code);
                }
                Ended::Spawn if !self.service => {
                    break SessionStatus::failed("the command could not be started");
                }
                Ended::Exited(code) if !self.service => break SessionStatus::Exited { code },
                Ended::Spawn | Ended::Exited(_) => {}
            }
            // A service's process ended by itself.
            let code = match ended {
                Ended::Exited(code) => code,
                Ended::Spawn | Ended::Stopped(_) | Ended::Restart => None,
            };
            let ran = started.elapsed();
            let delay = match backoff.ended(code, ran) {
                Retry::Done => {
                    self.log.service(ServiceState::Exited { code: 0 });
                    break SessionStatus::Exited { code };
                }
                Retry::GiveUp { restarts } => {
                    let state = ServiceState::Failed {
                        reason: ServiceFailure::GaveUp { restarts },
                    };
                    let message = state.summary();
                    self.log.service(state);
                    break SessionStatus::failed(message);
                }
                Retry::After(delay) => delay,
            };
            match self.back_off(attempt, code, delay).await {
                Wait::Again => attempt = attempt.saturating_add(1),
                Wait::Restart => {
                    backoff.reset();
                    attempt = 1;
                    self.log.notice("vessel: restarted".to_owned());
                }
                Wait::Stopped => {
                    self.log.service(ServiceState::Stopped);
                    break self.stopped(code);
                }
            }
        };
        self.log.status(status);
        let auxiliary = match self.leftover.0.take() {
            Some(path) => match tokio::fs::remove_file(&path).await {
                Ok(()) => CleanupFact::Confirmed,
                Err(error) => {
                    tracing::debug!(%error, "could not remove a run's command file");
                    CleanupFact::Unconfirmed
                }
            },
            None => CleanupFact::Confirmed,
        };
        let mut facts = if self.attempted {
            Facts::unconfirmed()
        } else {
            Facts::not_started()
        };
        facts.process = self.process_cleanup;
        facts.io = self.io_cleanup;
        facts.auxiliary = auxiliary;
        facts
    }

    /// The final status of a run that was stopped.
    fn stopped(&self, code: Option<i32>) -> SessionStatus {
        if self.stop.interrupted() {
            SessionStatus::Interrupted
        } else {
            SessionStatus::Exited { code }
        }
    }

    /// Wait out a service's backoff, unless it is stopped or restarted.
    async fn back_off(&mut self, attempt: u32, code: Option<i32>, delay: Duration) -> Wait {
        let restart_at = chrono::Duration::from_std(delay)
            .ok()
            .and_then(|delay| Utc::now().checked_add_signed(delay))
            .unwrap_or_else(Utc::now);
        self.log.service(ServiceState::BackingOff {
            attempt,
            code,
            restart_at,
        });
        tokio::select! {
            () = tokio::time::sleep(delay) => Wait::Again,
            Some(()) = self.restarting.recv() => Wait::Restart,
            _step = self.stop.next() => Wait::Stopped,
        }
    }

    /// Run the command once, to its end, a stop or a restart.
    async fn once(&mut self, first: &mut bool, attempt: u32) -> Ended {
        // A spawn error can follow partial child/pipe setup. Only never
        // attempting a spawn proves that no child or worker could exist.
        self.attempted = true;
        let Ok(Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        }) = vessel_proc::spawn(&self.spec)
        else {
            self.process_cleanup = CleanupFact::Unconfirmed;
            self.io_cleanup = CleanupFact::Unconfirmed;
            // A service backs off and tries again; its state says so.
            self.log
                .notice("vessel: the command could not be started".to_owned());
            return Ended::Spawn;
        };
        drop(stdin);
        self.log.running(Some(child.lifetime()));
        if *first {
            *first = false;
            self.log.status(SessionStatus::Running);
        }
        if self.service {
            self.log.service(ServiceState::Running {
                attempt,
                since: Utc::now(),
            });
        }
        let (sender, mut output) = mpsc::channel(OUTPUT_QUEUE);
        let pumps = Pumps::start(stdout, stderr, sender);
        let mut output_open = true;
        // A restart stops the process as a kill does: politely, then its
        // group at the grace.
        let mut restart = false;
        let mut force_at: Option<Instant> = None;
        let code = loop {
            tokio::select! {
                biased;
                step = self.stop.next() => {
                    signal(&mut child, step);
                    self.stop.advance(step);
                }
                Some(()) = self.restarting.recv(), if !restart => {
                    restart = true;
                    signal(&mut child, Step::Ask);
                    force_at = Some(Instant::now().checked_add(SCRIPT_GRACE).unwrap_or_else(Instant::now));
                }
                code = child.wait() => {
                    if code.is_err() { self.process_cleanup = CleanupFact::Unconfirmed; }
                    break code.ok().flatten();
                },
                chunk = output.recv(), if output_open => match chunk {
                    Some(bytes) => self.log.output(bytes),
                    None => output_open = false,
                },
                () = tokio::time::sleep_until(force_at.unwrap_or_else(Instant::now)), if force_at.is_some() => {
                    signal(&mut child, Step::Force);
                    force_at = None;
                }
            }
        };
        self.log.running(None);
        if pumps.finish(&mut self.log, &mut output, output_open).await == Drained::Incomplete {
            self.io_cleanup = CleanupFact::Unconfirmed;
        }
        if !self.stop.is_idle() {
            Ended::Stopped(code)
        } else if restart {
            Ended::Restart
        } else {
            Ended::Exited(code)
        }
    }
}

/// How a service's backoff ended.
enum Wait {
    Again,
    Restart,
    Stopped,
}

/// `SIGTERM` first, then `SIGKILL`, for the whole group (on Windows every
/// signal terminates the job).
fn signal(child: &mut Child, step: Step) {
    let signal = match step {
        Step::Ask => Signal::Terminate,
        Step::Force => Signal::Kill,
    };
    if let Err(error) = child.signal(signal) {
        tracing::warn!(pid = child.pid(), %error, ?signal, "cannot signal a script");
    }
}
