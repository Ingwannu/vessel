//! A worktree's setup and teardown: running the approved commands as a
//! task of the node's, their log, and reading it back (ADR 0027).
//!
//! [Decision Log]
//! - The commands that run are the ones parsed from the file whose digest
//!   was checked against the approvals, handed over as parsed: never a
//!   second read of the file, which an agent in the worktree could change
//!   in between.
//! - One log file per worktree in the node's data folder (not in the
//!   worktree, where it would show as a change and could be edited by what
//!   runs there). Setup and teardown append to it; `worktree.output` pages
//!   it, and waits on the log's length (a `watch` the runner bumps on
//!   every write, and once more when the run ends, so a waiter wakes for
//!   the end too).
//! - While a setup or teardown runs, the worktree holds its token in
//!   `running`; `worktree.setup`, `worktree.remove` and `session.create`
//!   see it and refuse, so two runs never share a folder.

mod observation;
mod progress;
// Staged until the reviewed registry gate/start-lease adapter can authorize it.
#[cfg(test)]
mod bootstrap;

pub(super) use progress::Observation;

use std::ffi::OsString;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use vessel_proto::TerminalBytes;
use vessel_proto::WorktreeId;
use vessel_proto::workspace::{
    ProjectCommand, SetupFailure, SetupOutput, SetupState, WorktreeInfo,
};
use vessel_workspace::commands::{self, MAX_LOG, Run, SETUP_LIMIT, TEARDOWN_LIMIT};

use super::{MAX_WAIT_SECS, WorkspaceError, Workspaces};

/// The most `worktree.output` answers at once.
const PAGE: u64 = 64 * 1024;
/// The most of a setup's end a session's notice repeats.
const NOTICE_TAIL: u64 = 2 * 1024;

/// Which run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Setup,
    Teardown,
}

impl Workspaces {
    /// A worktree's log file.
    pub(super) fn log_path(&self, worktree: WorktreeId) -> PathBuf {
        self.dirs.logs.join(format!("{worktree}.log"))
    }

    /// The environment a project's commands get, on top of the node's.
    fn command_env(&self, info: &WorktreeInfo) -> Vec<(String, OsString)> {
        let mut env = vec![(
            "VESSEL_WORKTREE".to_owned(),
            info.path.clone().into_os_string(),
        )];
        if let Some(root) = self
            .state()
            .projects
            .get(&info.project)
            .map(|entry| entry.info.root.clone())
        {
            env.push(("VESSEL_PROJECT_ROOT".to_owned(), root.into_os_string()));
        }
        if let Some(branch) = &info.branch {
            env.push(("VESSEL_BRANCH".to_owned(), branch.as_str().into()));
        }
        env
    }

    /// Start `commands` as the worktree's setup, in the background: the
    /// worktree is `running` from here until they end.
    ///
    /// # Errors
    /// `conflict` while another run holds the worktree.
    pub(super) fn start_setup(
        self: &Arc<Self>,
        worktree: WorktreeId,
        commands: Vec<ProjectCommand>,
    ) -> Result<WorktreeInfo, WorkspaceError> {
        let cancel = self.stop.child_token();
        let observation =
            Observation::new(worktree, &commands).ok_or(WorkspaceError::SetupPlanTooLarge)?;
        let (info, written) = {
            let mut state = self.state();
            let entry = state
                .worktrees
                .get_mut(&worktree)
                .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
            if entry.running.is_some() {
                return Err(WorkspaceError::Conflict(
                    "setup or teardown runs in this worktree".to_owned(),
                ));
            }
            entry.running = Some(cancel.clone());
            entry.observation = Some(observation);
            entry.info.setup = SetupState::Running {
                started_at: Utc::now(),
            };
            (entry.info.clone(), Arc::clone(&entry.written))
        };
        let workspaces = Arc::clone(self);
        let running = info.clone();
        drop(self.tasks.spawn(async move {
            let failure = workspaces
                .run_commands(&running, &commands, Kind::Setup, cancel, &written)
                .await;
            let succeeded = failure.is_none();
            let finished_at = Utc::now();
            let setup = match failure {
                None => SetupState::Succeeded { finished_at },
                Some(failure) => SetupState::Failed {
                    failure,
                    finished_at,
                },
            };
            let ended = {
                let mut state = workspaces.state();
                state.worktrees.get_mut(&worktree).map(|entry| {
                    entry.running = None;
                    entry.info.setup = setup;
                    if let Some(observation) = &mut entry.observation {
                        observation.finish(succeeded);
                        entry.observed.send_modify(|()| {});
                    }
                    entry.info.clone()
                })
            };
            // Wake the readers waiting for the end.
            written.send_modify(|_| {});
            if let Some(info) = ended
                && let Err(error) = workspaces.save_worktree(info).await
            {
                tracing::warn!(%error, "could not record a setup's end");
            }
        }));
        Ok(info)
    }

    /// Run `commands` in the worktree to their end; the failure, if any.
    /// A log that cannot be written is a failure to start.
    pub(super) async fn run_commands(
        &self,
        info: &WorktreeInfo,
        commands: &[ProjectCommand],
        kind: Kind,
        cancel: CancellationToken,
        written: &commands::Written,
    ) -> Option<SetupFailure> {
        let log = self.log_path(info.id);
        let run = Run {
            dir: &info.path,
            env: self.command_env(info),
            removed: &self.removed,
            log: &log,
            scratch: &self.dirs.scratch,
            limit: match kind {
                Kind::Setup => SETUP_LIMIT,
                Kind::Teardown => TEARDOWN_LIMIT,
            },
            cancel,
        };
        let generation = if kind == Kind::Setup {
            self.state()
                .worktrees
                .get(&info.id)
                .and_then(|entry| entry.observation.as_ref())
                .map(|observation| observation.snapshot.cursor.run)
        } else {
            None
        };
        match commands::run_observed(commands, &run, written, |event| {
            if let Some(generation) = generation {
                self.observe_setup(info.id, generation, event);
            }
        })
        .await
        {
            Ok(failure) => failure,
            Err(error) => {
                tracing::warn!(%error, "a worktree's log cannot be written");
                Some(SetupFailure::Spawn)
            }
        }
    }

    /// A page of the worktree's log from `from`; with `wait`, after more
    /// is written or the run ends (at most [`MAX_WAIT_SECS`]).
    ///
    /// # Errors
    /// `not_found`.
    pub(crate) async fn read_output(
        &self,
        worktree: WorktreeId,
        from: u64,
        wait: Option<u32>,
    ) -> Result<SetupOutput, WorkspaceError> {
        let (mut written, running) = {
            let state = self.state();
            let entry = state
                .worktrees
                .get(&worktree)
                .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
            (entry.written.subscribe(), entry.running.is_some())
        };
        let log = self.log_path(worktree);
        let length = file_length(&log).await;
        if let Some(seconds) = wait
            && running
            && from >= length
        {
            let limit = Duration::from_secs(seconds.min(MAX_WAIT_SECS).into());
            // Not marked seen here: a write (or the run's end) between the
            // subscription and now is what this wait is for, and marking it
            // seen would sleep the whole wait past it.
            let _changed = tokio::time::timeout(limit, written.changed()).await;
        }
        let setup = self
            .state()
            .worktrees
            .get(&worktree)
            .map(|entry| entry.info.setup.clone())
            .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
        let (bytes, length) = read_page(&log, from, PAGE).await?;
        let start = from.min(length);
        let next = start.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        Ok(SetupOutput {
            from: start,
            bytes: TerminalBytes::new(bytes),
            next,
            truncated: length >= MAX_LOG,
            setup,
        })
    }

    /// The last lines of the worktree's log, for a session's notice.
    pub(super) async fn log_tail(&self, worktree: WorktreeId) -> String {
        let log = self.log_path(worktree);
        let length = file_length(&log).await;
        let from = length.saturating_sub(NOTICE_TAIL);
        let Ok((bytes, _)) = read_page(&log, from, NOTICE_TAIL).await else {
            return String::new();
        };
        let text = String::from_utf8_lossy(&bytes);
        // A cut first line is dropped.
        let text = if from > 0 {
            text.split_once('\n').map_or("", |(_, rest)| rest)
        } else {
            &text
        };
        text.trim_end().to_owned()
    }
}

/// Append one line to a log, outside a run (the node stopped one).
pub(super) async fn note(log: &Path, line: &str) {
    let opened = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .await;
    let written = match opened {
        Ok(mut file) => file.write_all(format!("{line}\n").as_bytes()).await,
        Err(error) => Err(error),
    };
    if let Err(error) = written {
        tracing::debug!(%error, "a worktree's log cannot be written");
    }
}

async fn file_length(path: &Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map_or(0, |metadata| metadata.len())
}

/// At most `limit` bytes from `from` (none past the end), and the length.
async fn read_page(path: &Path, from: u64, limit: u64) -> Result<(Bytes, u64), WorkspaceError> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Bytes::new(), 0));
        }
        Err(error) => return Err(WorkspaceError::Io(error.to_string())),
    };
    let io = |error: std::io::Error| WorkspaceError::Io(error.to_string());
    let length = file.metadata().await.map_err(io)?.len();
    let start = from.min(length);
    let _at = file.seek(SeekFrom::Start(start)).await.map_err(io)?;
    let mut bytes = Vec::new();
    let _read = file.take(limit).read_to_end(&mut bytes).await.map_err(io)?;
    Ok((Bytes::from(bytes), length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_page_never_reads_past_the_end() {
        let folder = tempfile::tempdir().unwrap();
        let log = folder.path().join("a.log");
        tokio::fs::write(&log, b"one\ntwo\n").await.unwrap();
        let (bytes, length) = read_page(&log, 4, 64).await.unwrap();
        assert_eq!((&bytes[..], length), (&b"two\n"[..], 8));
        let (bytes, _) = read_page(&log, 100, 64).await.unwrap();
        assert!(bytes.is_empty());
        let (bytes, length) = read_page(&folder.path().join("none.log"), 0, 64)
            .await
            .unwrap();
        assert_eq!((bytes.len(), length), (0, 0));
    }
}
