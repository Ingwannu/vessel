//! The `git` program, run safely (ADR 0023): finding it, checking its
//! version once, and every run's protections ([`run`]).

mod clone;
mod diff;
pub mod gitfile;
mod history;
pub mod metadata;
mod panel_history;
mod patch;
mod redact;
mod ref_read;
mod refusal;
mod repo;
mod repository_lock;
mod run;
mod search;
mod status;
mod upstream;
mod worktree;
mod write;

use std::path::Path;
use std::time::Duration;

use vessel_proto::workspace::{GitSupport, GitVersion};

pub use clone::{Failed as CloneFailed, Progress as CloneProgress};
pub use diff::{Comparison, ComparisonError, Resolved};
pub use panel_history::{HistoryError, HistoryField, HistoryQuery};
pub(crate) use redact::{redact, tail};
pub use run::GitSetup;
pub use run::own_config::OwnConfigRefusal;
pub(crate) use run::{GitCommand, GitOutput, Limits};
pub use status::MAX_ENTRIES;
pub use upstream::{Checkout, RemoteAddress};
pub use worktree::{AddRefusal, NewBranch, SnapshotRefusal};

/// A usable `git`: found, and at least [`GitVersion::MINIMUM`].
#[derive(Debug, Clone)]
pub struct Git {
    runner: run::Runner,
    /// An owned comparison's cancellation reaches every helper command,
    /// including base/index preparation, rather than only its final diff.
    cancel: Option<tokio_util::sync::CancellationToken>,
    version: GitVersion,
    /// Shared by every clone: the runs that change a repository's
    /// worktrees take turns per repository (`repository_lock`).
    worktrees: std::sync::Arc<repository_lock::RepositoryLocks>,
}

/// What looking for git found.
#[derive(Debug, Clone)]
pub struct Found {
    /// What to report (`project.list`).
    pub support: GitSupport,
    /// The git to run, when it is usable.
    pub git: Option<Git>,
}

impl Git {
    /// Ask the program `setup` names (found by the node on its `PATH`) for
    /// its version, once. `None` is a node without git.
    pub async fn probe(setup: Option<GitSetup>) -> Found {
        let missing = Found {
            support: GitSupport::Missing,
            git: None,
        };
        let Some(setup) = setup else {
            return missing;
        };
        // git reads `core.hooksPath` relative to the repository it runs in:
        // a relative empty folder would be the repository's own, whose
        // hooks it ships (and would run on a worktree's creation).
        if !setup.no_hooks.is_absolute() {
            tracing::warn!("the empty hooks folder is not an absolute path; git is not used");
            return missing;
        }
        let runner = run::Runner::new(setup);
        let output = runner
            .command(Path::new("."))
            .outside_repository()
            .arg("--version")
            .limits(Limits {
                time: Duration::from_secs(10),
                stdout: 4096,
                stderr: 4096,
            })
            .output()
            .await;
        let version = match output {
            Ok(output) if output.succeeded() => {
                GitVersion::parse(&String::from_utf8_lossy(&output.stdout))
            }
            Ok(output) => {
                tracing::warn!(code = ?output.code, "`git --version` failed");
                None
            }
            Err(error) => {
                tracing::warn!(%error, "could not run git");
                None
            }
        };
        match version {
            None => missing,
            Some(version) if version < GitVersion::MINIMUM => {
                tracing::warn!(%version, minimum = %GitVersion::MINIMUM, "git is too old for the workspace calls");
                Found {
                    support: GitSupport::TooOld {
                        version,
                        minimum: GitVersion::MINIMUM,
                    },
                    git: None,
                }
            }
            Some(version) => Found {
                support: GitSupport::Found { version },
                git: Some(Self {
                    runner,
                    cancel: None,
                    version,
                    worktrees: std::sync::Arc::default(),
                }),
            },
        }
    }

    /// Its version.
    #[must_use]
    pub const fn version(&self) -> GitVersion {
        self.version
    }

    /// Bind a tracked operation's cancellation to all of its subprocesses.
    /// Existing unbound callers retain their current runner behaviour.
    #[must_use]
    pub fn cancellable(&self, token: tokio_util::sync::CancellationToken) -> Self {
        Self {
            cancel: Some(token),
            ..self.clone()
        }
    }

    /// A command run in `dir`.
    pub(crate) fn command(&self, dir: &Path) -> GitCommand {
        let command = self.runner.command(dir);
        match &self.cancel {
            Some(token) => command.cancel_on(token.clone()),
            None => command,
        }
    }
}

/// A git run that went wrong.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The program could not be started.
    #[error("could not start git")]
    Spawn(#[source] vessel_proc::SpawnError),
    /// Its exit could not be read.
    #[error("could not wait for git")]
    Wait(#[source] vessel_proc::WaitError),
    /// It ran past its time and was stopped.
    #[error("git did not finish within {} s and was stopped", after.as_secs())]
    TimedOut {
        /// The limit.
        after: Duration,
    },
    /// It was stopped because the caller no longer wanted it.
    #[error("git was stopped")]
    Cancelled,
    /// The node is shutting down.
    #[error("the node is stopping")]
    Closed,
    /// Too many git runs already wait for a slot.
    #[error("the node is running as much git as it allows; try again")]
    Busy,
    /// It printed more than the limit for that call.
    #[error("git printed more than this call reads")]
    OutputLimit,
    /// It exited with an error.
    #[error("git failed ({}): {message}", code.map_or_else(|| "killed".to_owned(), |code| code.to_string()))]
    Failed {
        /// Its exit code.
        code: Option<i32>,
        /// The end of its standard error, credentials removed.
        message: String,
    },
    /// Its output was not what this git version prints.
    #[error("git printed something unexpected ({what})")]
    Unexpected {
        /// What was being read.
        what: &'static str,
    },
    /// The repository's own configuration sets something Vessel's git
    /// does not run with (ADR 0030). Its value is never said.
    #[error("this repository's own configuration sets `{key}`: {why}")]
    OwnConfig {
        /// The key, as git lists it.
        key: String,
        /// Why the run was refused.
        why: OwnConfigRefusal,
    },
    /// The repository's own configuration is larger than a run reads
    /// before it (ADR 0030).
    #[error("this repository's own configuration is larger than Vessel reads before running git")]
    OwnConfigTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A relative empty hooks folder would be a repository's own: git is
    /// not used at all then.
    #[tokio::test]
    async fn a_relative_hooks_folder_means_no_git() {
        let program = which::which("git").expect("the tests need git on PATH");
        let found = Git::probe(Some(GitSetup {
            program,
            no_hooks: "workspace/no-hooks".into(),
            env: Vec::new(),
        }))
        .await;
        assert!(found.git.is_none());
        assert_eq!(found.support, GitSupport::Missing);
    }
}
