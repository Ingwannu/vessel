//! The terminal-mode session actor: a PTY's output becomes events, and its
//! silence makes the session idle.

use std::pin::Pin;
use std::time::Duration;

use bytes::Bytes;
use tokio::time::{Instant, Sleep};
use vessel_harness::WrittenMcpFile;
use vessel_proto::{SessionEvent, SessionStatus, TerminalBytes, TerminalOutput};
use vessel_pty::{PtyOutput, PtyProcess};

use crate::lifecycle::{Lifecycle, drain_deadline};

/// How long a terminal may print nothing before its session counts as idle
/// on the board (ADR 0019).
///
/// A terminal has no turns Vessel can see, so output is the only sign of
/// work. Long enough that a harness thinking between bursts of output, or
/// a spinner that redraws every second, still reads as working; short
/// enough that the board shows a finished turn within seconds. Tuning it is
/// a judgement about real harnesses, not a setting: no caller needs another
/// value, and a test checks the timer with tokio's paused clock instead.
pub(crate) const QUIET: Duration = Duration::from_secs(5);

/// Owns a PTY program for the life of a terminal session.
pub(crate) struct TerminalActor {
    pub(crate) pty: PtyProcess,
    pub(crate) output: PtyOutput,
    pub(crate) life: Lifecycle,
    /// The session's MCP configuration file, removed when the actor ends
    /// (the program has exited by then).
    pub(crate) _mcp_file: Option<WrittenMcpFile>,
}

impl TerminalActor {
    /// Run until the program has exited and its final status is published.
    pub(crate) async fn run(mut self) {
        // The program is already running when the actor starts.
        self.life.starting(&mut self.pty).await;
        self.life
            .status(&mut self.pty, SessionStatus::Running)
            .await;

        let mut output_open = true;
        let mut quiet = Quiet::new();
        let code = loop {
            tokio::select! {
                biased;
                step = self.life.stop.next() => self.life.apply(&mut self.pty, step),
                code = self.pty.wait() => break code,
                chunk = self.output.next(), if output_open => match chunk {
                    Some(bytes) => {
                        quiet.output();
                        self.life.publish(&mut self.pty, terminal(bytes)).await;
                    }
                    None => output_open = false,
                },
                Some(call) = self.life.model_calls.recv() => {
                    self.life.publish(&mut self.pty, SessionEvent::ModelCall(call)).await;
                }
                Some(change) = self.life.metadata.recv() => {
                    self.life.metadata_changed(&mut self.pty, change).await;
                }
                Ok(()) = self.life.forge.changed() => self.life.forge_changed(),
                () = quiet.elapsed() => self.life.out.quiet(),
            }
        };
        if output_open {
            self.drain().await;
        }
        self.life.drain_model_calls(&mut self.pty).await;

        let status = if self.life.stop.interrupted() {
            SessionStatus::Interrupted
        } else {
            SessionStatus::Exited { code }
        };
        self.life.status(&mut self.pty, status).await;
    }

    /// Publish what the PTY still has after the exit, for a bounded time.
    async fn drain(&mut self) {
        let deadline = drain_deadline();
        loop {
            tokio::select! {
                chunk = self.output.next() => match chunk {
                    Some(bytes) => self.life.publish(&mut self.pty, terminal(bytes)).await,
                    None => return,
                },
                () = tokio::time::sleep_until(deadline) => {
                    tracing::warn!(
                        pid = self.pty.pid(),
                        "terminal output still open after exit; a process escaped the group"
                    );
                    return;
                }
            }
        }
    }
}

fn terminal(bytes: Bytes) -> SessionEvent {
    SessionEvent::Terminal(TerminalOutput {
        data: TerminalBytes::new(bytes),
    })
}

/// The terminal's quiet timer: one resettable timer in the actor's loop,
/// which fires once [`QUIET`] after the last output.
///
/// [Decision Log]
/// - Purpose: the board must turn a terminal idle when its output stops,
///   and only then; output alone must not wake info watchers.
/// - Alternatives: a periodic check of the time since the last output (a
///   poll, which AGENTS.md section 4 rules out, and late by up to its
///   period); a timer per output chunk (a timer allocation per chunk).
/// - Choice: one `Sleep`, pushed back on every chunk. Pushing a deadline
///   later is cheap in tokio (the timer entry records the new deadline and
///   is re-registered only when the old one comes due), so a burst of
///   output costs a clock read and a store per chunk. After it fires the
///   timer is disarmed until the next output, so a quiet terminal costs
///   nothing.
/// - Cost: one boxed `Sleep` per terminal session (a `Sleep` cannot move
///   once polled, and the actor owns it across loop turns).
struct Quiet {
    sleep: Pin<Box<Sleep>>,
    /// Waiting for a quiet stretch; false once it came, until the next
    /// output.
    armed: bool,
}

impl Quiet {
    /// Armed from the start: a program that never prints is quiet after
    /// [`QUIET`].
    fn new() -> Self {
        Self {
            sleep: Box::pin(tokio::time::sleep(QUIET)),
            armed: true,
        }
    }

    /// Output came: quiet only [`QUIET`] from now.
    fn output(&mut self) {
        // An instant past the end of time cannot be armed; the terminal
        // then stays active, the safe side for a board.
        if let Some(deadline) = Instant::now().checked_add(QUIET) {
            self.sleep.as_mut().reset(deadline);
            self.armed = true;
        }
    }

    /// Resolves once [`QUIET`] has passed since the last output, once per
    /// quiet stretch; pending while disarmed. Cancel-safe: dropped before
    /// it resolves, the timer stays as it was.
    async fn elapsed(&mut self) {
        if !self.armed {
            return std::future::pending().await;
        }
        self.sleep.as_mut().await;
        self.armed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the quiet stretch has come by now, without waiting.
    async fn has_elapsed(quiet: &mut Quiet) -> bool {
        tokio::time::timeout(Duration::ZERO, quiet.elapsed())
            .await
            .is_ok()
    }

    /// Just short of a quiet stretch.
    const ALMOST: Duration = Duration::from_millis(4_999);

    #[tokio::test(start_paused = true)]
    async fn quiet_comes_once_quiet_after_the_last_output() {
        assert_eq!(ALMOST + Duration::from_millis(1), QUIET);
        let mut quiet = Quiet::new();
        tokio::time::advance(ALMOST).await;
        assert!(!has_elapsed(&mut quiet).await, "not yet");

        // Output pushes it back: a full stretch from the last output.
        quiet.output();
        tokio::time::advance(ALMOST).await;
        assert!(!has_elapsed(&mut quiet).await, "pushed back by output");
        quiet.output();
        tokio::time::advance(ALMOST).await;
        assert!(!has_elapsed(&mut quiet).await, "pushed back again");
        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(has_elapsed(&mut quiet).await, "quiet");

        // Once per stretch: disarmed until the next output.
        tokio::time::advance(QUIET.saturating_mul(3)).await;
        assert!(!has_elapsed(&mut quiet).await, "fired already");
        quiet.output();
        tokio::time::advance(QUIET).await;
        assert!(has_elapsed(&mut quiet).await, "armed again by output");
    }

    #[tokio::test(start_paused = true)]
    async fn a_program_that_never_prints_is_quiet() {
        let mut quiet = Quiet::new();
        tokio::time::advance(QUIET).await;
        assert!(has_elapsed(&mut quiet).await);
    }
}
