//! A shell in a PTY: its actor and its start.

use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::Instrument as _;
use vessel_proc::CommandSpec;
use vessel_proto::SessionStatus;
use vessel_proto::terminals::TerminalInfo;
use vessel_pty::{PtyOutput, PtyProcess, WindowSize};

use super::cleanup::{self, Facts};
use super::handle::{Shared, TerminalHandle};
use super::log::TermLog;
use super::{Scrollback, TerminalError};
use crate::lifecycle::{Stoppable as _, drain_deadline};
use crate::ring::Ring;
use crate::stop::Stopper;

/// What a shell starts with.
#[derive(Debug)]
pub struct ShellStart {
    /// Its info as it starts (the node fills its place, folder, kind,
    /// title and creator).
    pub info: TerminalInfo,
    /// The shell, its folder and its environment.
    pub spec: CommandSpec,
    /// Its window, as the wire gives it (the node needs no PTY types).
    pub size: vessel_proto::TerminalSize,
    /// How much of its log is kept.
    pub scrollback: Scrollback,
    /// The node's shutdown: a terminal's own stop is a child of it.
    pub shutdown: CancellationToken,
    /// The node's tasks: the actor is one, so the node's stop waits for
    /// its end.
    pub tasks: TaskTracker,
}

/// Start a shell. Its first records are `status` `starting`, then
/// `running`.
///
/// # Errors
/// [`TerminalError`] when it does not start (nothing is left behind).
pub async fn start_shell(start: ShellStart) -> Result<TerminalHandle, TerminalError> {
    let ShellStart {
        mut info,
        spec,
        size,
        scrollback,
        shutdown,
        tasks,
    } = start;
    if shutdown.is_cancelled() {
        return Err(TerminalError::ShuttingDown);
    }
    let window = WindowSize {
        cols: size.cols,
        rows: size.rows,
    };
    // `openpty`, `fork` and `exec` and thread creation all block.
    let (pty, output) = tokio::task::spawn_blocking(move || PtyProcess::spawn(&spec, window))
        .await
        .map_err(TerminalError::Internal)?
        .map_err(TerminalError::Spawn)?;
    info.size = Some(size);
    let id = info.id;
    let (completion, cleanup) = cleanup::begin(id);
    let ring = Ring::bounded(scrollback.records, scrollback.bytes, info.next_seq);
    let kill = shutdown.child_token();
    let shared = Arc::new(Shared {
        cleanup,
        info: watch::Sender::new(info),
        ring: Mutex::new(ring),
        input: Some(pty.input()),
        kill: kill.clone(),
        restarts: None,
        pid: AtomicU32::new(pty.pid()),
        process: Mutex::new(Some(pty.lifetime())),
    });
    let actor = ShellActor {
        log: TermLog::new(Arc::clone(&shared)),
        stop: Stopper::new(kill, shutdown),
        pty,
        output,
    };
    drop(
        tasks.spawn(
            async move {
                completion.finish(actor.run().await);
            }
            .instrument(tracing::info_span!("terminal", terminal = %id)),
        ),
    );
    Ok(TerminalHandle(shared))
}

/// Owns a shell for its life.
struct ShellActor {
    log: TermLog,
    stop: Stopper,
    pty: PtyProcess,
    output: PtyOutput,
}

impl ShellActor {
    async fn run(mut self) -> Facts {
        self.log.status(SessionStatus::Starting);
        self.log.status(SessionStatus::Running);
        let session = self.pty.pid();
        let mut output_open = true;
        let code = loop {
            tokio::select! {
                biased;
                step = self.stop.next() => {
                    self.pty.stop(step);
                    self.stop.advance(step);
                }
                code = self.pty.wait() => break code,
                chunk = self.output.next(), if output_open => match chunk {
                    Some(bytes) => self.log.output(bytes),
                    None => output_open = false,
                },
            }
        };
        // The jobs an interactive shell put in groups of their own are
        // still in its session, holding the PTY open (ADR 0031): they end
        // with it, and then its output ends too.
        sweep_session(session).await;
        if output_open {
            self.drain().await;
        }
        let status = if self.stop.interrupted() {
            SessionStatus::Interrupted
        } else {
            SessionStatus::Exited { code }
        };
        self.log.status(status);
        // Legacy wait conflates a signalled exit and a missing waiter; sweeps
        // and bounded output drain likewise do not certify physical cleanup.
        // Preserve those unknowns until their actual owners expose proof.
        let mut facts = Facts::unconfirmed();
        facts.auxiliary = super::CleanupFact::Confirmed;
        facts
    }

    /// What the PTY still holds after the exit, for a bounded time.
    async fn drain(&mut self) {
        let deadline = drain_deadline();
        loop {
            tokio::select! {
                chunk = self.output.next() => match chunk {
                    Some(bytes) => self.log.output(bytes),
                    None => return,
                },
                () = tokio::time::sleep_until(deadline) => {
                    tracing::debug!("a shell's output stays open after its exit");
                    return;
                }
            }
        }
    }
}

/// Kill whatever is left in the shell's session (Linux; elsewhere nothing
/// can be found without a privileged call, and the shell's own `SIGHUP`
/// to its jobs is what ends them).
async fn sweep_session(session: u32) {
    let swept = tokio::task::spawn_blocking(move || vessel_proc::kill_session(session)).await;
    match swept {
        Ok(Ok(0)) => {}
        Ok(Ok(killed)) => tracing::debug!(killed, "a shell's jobs ended with it"),
        Ok(Err(error)) => tracing::warn!(%error, "cannot end a shell's jobs"),
        Err(error) => tracing::warn!(%error, "the sweep of a shell's session stopped"),
    }
}
