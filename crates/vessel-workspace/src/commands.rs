//! Running a project's approved commands (setup and teardown) in a
//! worktree, their output kept in a bounded log (ADR 0024).
//!
//! [Decision Log]
//! - Purpose: a worktree is prepared by the commands its project's file
//!   names, once a person approved that set; the output is what a page
//!   shows while it runs and what explains a failure afterwards.
//! - These are shell commands by nature (`npm ci && cp …`), so they go
//!   through the platform's shell: `/bin/sh -c <command>` on Unix, the
//!   command handed over as one argument; on Windows `cmd.exe /d /c` on a
//!   temporary `.cmd` file holding it, since `cmd.exe` reads its command
//!   line with its own quoting rules and a file needs none.
//! - Each command is its own process group (`vessel-proc`), stopped whole
//!   at the time limit or on cancel; standard input is closed; standard
//!   output and error go, as they arrive, into one log file of at most
//!   [`MAX_LOG`] bytes (past that a line says so and the rest is dropped,
//!   while the command keeps running and is still waited for).
//! - The first command that fails stops the rest: a later command usually
//!   needs what the earlier one made.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vessel_proc::{CommandSpec, Signal, Spawned};
use vessel_proto::workspace::{ProjectCommand, SetupFailure};

/// The most output kept per worktree.
pub const MAX_LOG: u64 = 1024 * 1024;
/// How long a setup may take in all.
pub const SETUP_LIMIT: Duration = Duration::from_secs(30 * 60);
/// How long a teardown may take in all.
pub const TEARDOWN_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Where and how commands run.
#[derive(Debug, Clone)]
pub struct Run<'a> {
    /// The worktree.
    pub dir: &'a Path,
    /// Added to the node's environment (`VESSEL_PROJECT_ROOT`, …).
    pub env: Vec<(String, OsString)>,
    /// The log file, appended to.
    pub log: &'a Path,
    /// A folder for temporary files (Windows command files).
    pub scratch: &'a Path,
    /// The time limit for all of them.
    pub limit: Duration,
    /// Stops them.
    pub cancel: CancellationToken,
}

/// What a log reader is told: how long the log is now.
pub type Written = tokio::sync::watch::Sender<u64>;

/// Run `commands` in order; `Ok(None)` when all exited with 0.
///
/// # Errors
/// The log file cannot be written.
pub async fn run(
    commands: &[ProjectCommand],
    run: &Run<'_>,
    written: &Written,
) -> std::io::Result<Option<SetupFailure>> {
    let mut log = Log::open(run.log, written).await?;
    // The limits are minutes; an instant that far ahead always exists.
    let deadline = tokio::time::Instant::now()
        .checked_add(run.limit)
        .unwrap_or_else(tokio::time::Instant::now);
    for (index, command) in commands.iter().enumerate() {
        log.line(&format!("$ {}", command.as_str())).await?;
        let outcome = one(command, index, run, deadline, &mut log).await?;
        if let Some(failure) = outcome {
            return Ok(Some(failure));
        }
    }
    Ok(None)
}

/// One command, to its end, the deadline or a cancel.
async fn one(
    command: &ProjectCommand,
    index: usize,
    run: &Run<'_>,
    deadline: tokio::time::Instant,
    log: &mut Log<'_>,
) -> std::io::Result<Option<SetupFailure>> {
    let (spec, script) = shell(command, index, run).await?;
    let spawned = vessel_proc::spawn(&spec);
    let Ok(Spawned {
        mut child,
        stdin,
        stdout,
        stderr,
    }) = spawned
    else {
        remove(script).await;
        log.line("vessel: the shell did not start").await?;
        return Ok(Some(SetupFailure::Spawn));
    };
    drop(stdin);
    let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(16);
    let pumps = tokio::spawn(pump_both(stdout, stderr, sender));
    let ended = loop {
        tokio::select! {
            chunk = receiver.recv() => match chunk {
                Some(chunk) => log.write(&chunk).await?,
                None => break Ended::Closed,
            },
            () = tokio::time::sleep_until(deadline) => break Ended::TimedOut,
            () = run.cancel.cancelled() => break Ended::Cancelled,
        }
    };
    let code = match ended {
        Ended::Closed => child.wait().await.ok().flatten(),
        Ended::TimedOut | Ended::Cancelled => {
            if let Err(error) = child.signal(Signal::Kill) {
                tracing::debug!(%error, "a setup command had already ended");
            }
            let _code = child.wait().await;
            None
        }
    };
    pumps.abort();
    remove(script).await;
    let index = u32::try_from(index).unwrap_or(u32::MAX);
    Ok(match ended {
        Ended::TimedOut => {
            log.line("vessel: stopped at the time limit").await?;
            Some(SetupFailure::TimedOut)
        }
        Ended::Cancelled => {
            log.line("vessel: stopped").await?;
            Some(SetupFailure::Exit {
                command: index,
                code: None,
            })
        }
        Ended::Closed if code == Some(0) => None,
        Ended::Closed => {
            log.line(&format!(
                "vessel: exited with {}",
                code.map_or_else(|| "a signal".to_owned(), |code| code.to_string())
            ))
            .await?;
            Some(SetupFailure::Exit {
                command: index,
                code,
            })
        }
    })
}

enum Ended {
    Closed,
    TimedOut,
    Cancelled,
}

/// Both pipes into one channel, as they arrive.
async fn pump_both(
    stdout: impl AsyncRead + Unpin,
    stderr: impl AsyncRead + Unpin,
    sender: mpsc::Sender<Vec<u8>>,
) {
    let out = pump(stdout, sender.clone());
    let err = pump(stderr, sender);
    let ((), ()) = tokio::join!(out, err);
}

async fn pump(mut stream: impl AsyncRead + Unpin, sender: mpsc::Sender<Vec<u8>>) {
    let mut chunk = vec![0_u8; 8 * 1024];
    while let Ok(read) = stream.read(&mut chunk).await {
        if read == 0 {
            return;
        }
        if sender
            .send(chunk.get(..read).unwrap_or_default().to_vec())
            .await
            .is_err()
        {
            return;
        }
    }
}

/// The command as the platform's shell runs it, and a script file to
/// remove afterwards (Windows).
async fn shell(
    command: &ProjectCommand,
    index: usize,
    run: &Run<'_>,
) -> std::io::Result<(CommandSpec, Option<PathBuf>)> {
    let base = |program: &str| {
        let mut spec = CommandSpec::new(program).cwd(run.dir);
        for (name, value) in &run.env {
            spec = spec.env(name, value);
        }
        spec
    };
    if cfg!(windows) {
        let script = run
            .scratch
            .join(format!("vessel-command-{index}-{}.cmd", std::process::id()));
        tokio::fs::write(&script, format!("@echo off\r\n{}\r\n", command.as_str())).await?;
        let spec = base("cmd.exe").args(["/d", "/c"]).arg(&script);
        Ok((spec, Some(script)))
    } else {
        Ok((base("/bin/sh").arg("-c").arg(command.as_str()), None))
    }
}

async fn remove(script: Option<PathBuf>) {
    if let Some(script) = script
        && let Err(error) = tokio::fs::remove_file(&script).await
    {
        tracing::debug!(%error, "could not remove a command file");
    }
}

/// The bounded log file.
struct Log<'a> {
    file: tokio::fs::File,
    length: u64,
    full: bool,
    written: &'a Written,
}

impl<'a> Log<'a> {
    async fn open(path: &Path, written: &'a Written) -> std::io::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        let length = file.metadata().await?.len();
        Ok(Self {
            file,
            full: length >= MAX_LOG,
            length,
            written,
        })
    }

    async fn line(&mut self, text: &str) -> std::io::Result<()> {
        self.write(format!("{text}\n").as_bytes()).await
    }

    async fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if self.full {
            return Ok(());
        }
        let room = usize::try_from(MAX_LOG.saturating_sub(self.length)).unwrap_or(usize::MAX);
        let kept = bytes.get(..room.min(bytes.len())).unwrap_or_default();
        self.file.write_all(kept).await?;
        self.length = self
            .length
            .saturating_add(u64::try_from(kept.len()).unwrap_or(u64::MAX));
        if kept.len() < bytes.len() {
            self.full = true;
        }
        self.file.flush().await?;
        let _receivers = self.written.send_replace(self.length);
        Ok(())
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    fn commands(list: &[&str]) -> Vec<ProjectCommand> {
        list.iter()
            .map(|command| command.parse().unwrap())
            .collect()
    }

    fn setup(temp: &Path) -> (PathBuf, Written) {
        (temp.join("log"), tokio::sync::watch::channel(0).0)
    }

    #[tokio::test]
    async fn commands_run_in_order_with_their_environment_and_stop_at_a_failure() {
        let temp = tempfile::tempdir().unwrap();
        let (log, written) = setup(temp.path());
        let run_in = Run {
            dir: temp.path(),
            env: vec![("VESSEL_BRANCH".to_owned(), "topic".into())],
            log: &log,
            scratch: temp.path(),
            limit: SETUP_LIMIT,
            cancel: CancellationToken::new(),
        };
        let ok = run(
            &commands(&["echo one", "echo $VESSEL_BRANCH 1>&2"]),
            &run_in,
            &written,
        )
        .await
        .unwrap();
        assert_eq!(ok, None);
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.contains("$ echo one\none\n"), "{text}");
        assert!(text.contains("topic"), "{text}");
        assert_eq!(*written.borrow(), u64::try_from(text.len()).unwrap());

        let failed = run(&commands(&["exit 3", "touch never"]), &run_in, &written)
            .await
            .unwrap();
        assert_eq!(
            failed,
            Some(SetupFailure::Exit {
                command: 0,
                code: Some(3)
            })
        );
        assert!(!temp.path().join("never").exists(), "the rest does not run");
    }

    #[tokio::test]
    async fn the_time_limit_and_a_cancel_stop_a_command_and_the_log_is_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let (log, written) = setup(temp.path());
        let slow = Run {
            dir: temp.path(),
            env: Vec::new(),
            log: &log,
            scratch: temp.path(),
            limit: Duration::from_millis(300),
            cancel: CancellationToken::new(),
        };
        let timed_out = run(&commands(&["sleep 30"]), &slow, &written)
            .await
            .unwrap();
        assert_eq!(timed_out, Some(SetupFailure::TimedOut));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let stopped = Run {
            cancel,
            limit: SETUP_LIMIT,
            ..slow.clone()
        };
        let cancelled = run(&commands(&["sleep 30"]), &stopped, &written)
            .await
            .unwrap();
        assert!(matches!(
            cancelled,
            Some(SetupFailure::Exit { code: None, .. })
        ));

        let loud = Run {
            limit: SETUP_LIMIT,
            ..slow
        };
        let printed = run(
            &commands(&["head -c 3000000 /dev/zero | tr '\\0' x"]),
            &loud,
            &written,
        )
        .await
        .unwrap();
        assert_eq!(printed, None, "the command finishes though the log is full");
        assert!(std::fs::metadata(&log).unwrap().len() <= MAX_LOG);
    }
}
