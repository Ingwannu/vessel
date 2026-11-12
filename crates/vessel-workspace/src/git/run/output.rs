//! Running a built command to its end, and what it printed
//! ([`GitOutput`]): a slot first, then the process, its output read within
//! its limits, and the kill past its time or when the caller cancels. What
//! runs, and with what environment, is decided before (`run`).
//!
//! [Decision Log]
//! - Standard input is closed at once; output past its limit is cut (the
//!   caller says whether that is an error) and the process is killed; a
//!   run past its time is killed and answers [`GitError::TimedOut`].

use tokio::io::{AsyncRead, AsyncReadExt};
use vessel_proc::{Signal, Spawned};

use super::{GitCommand, KIB};
use crate::git::GitError;

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
            message: crate::git::redact::tail(&self.stderr_text()),
        })
    }
}

impl GitCommand {
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
        let cancel = cancel.unwrap_or_default();
        // One deadline for the wait for a slot and the run: a call behind
        // slow ones answers within its time, and a cancelled one stops
        // waiting, rather than queueing until a slot frees.
        let started = tokio::time::Instant::now();
        let _slot = tokio::select! {
            slot = pool.slot() => slot?,
            () = tokio::time::sleep(limits.time) => {
                return Err(GitError::TimedOut { after: limits.time });
            }
            () = cancel.cancelled() => return Err(GitError::Cancelled),
        };
        let Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        } = vessel_proc::spawn(&spec).map_err(GitError::Spawn)?;
        // Nothing is ever typed into git.
        drop(stdin);
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
                () = tokio::time::sleep(limits.time.saturating_sub(started.elapsed())) => {
                    Ended::TimedOut
                }
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
