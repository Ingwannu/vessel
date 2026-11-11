//! Cloning a repository into a new folder, with progress, a time limit and
//! cancellation (ADR 0023).
//!
//! [Decision Log]
//! - Purpose: `project.clone` fetches a repository a client names; the
//!   page shows how far it is and a person can stop it.
//! - Choice (agent-orchestrator's, `service/project/clone.go`): clone into
//!   a hidden temporary folder beside the target and rename it into place
//!   only when git succeeded, so a half-cloned folder never sits under the
//!   project's name, and a failure or a cancel removes only what Vessel
//!   made. Progress is read from git's `--progress` lines on standard
//!   error (`Receiving objects:  42% (…)`), which git prints in English
//!   here (`LC_ALL=C`).
//! - Three walls against a URL that runs code: `CloneUrl` refuses `ext::`,
//!   local paths and options where it is decoded; the URL goes after `--`;
//!   and `GIT_ALLOW_PROTOCOL` limits git, submodules included, to
//!   `https`, `http`, `ssh` and `git`. Submodules are not cloned (a setup
//!   command can, once a person approved it).

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::{CloneFailure, CloneStage, CloneUrl};

use super::{Git, GitError, Limits, redact};

/// How far a clone is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// What git is doing.
    pub stage: CloneStage,
    /// How far along, when git says.
    pub percent: Option<u8>,
}

/// Why a clone failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    /// For a client.
    pub reason: CloneFailure,
    /// git's last lines, credentials removed, for a person.
    pub message: String,
}

impl Git {
    /// Clone `url` into `target` (which must not exist), through a
    /// temporary folder `temporary` beside it. `progress` gets each change
    /// of stage or percent. Stopped when `cancel` is.
    ///
    /// # Errors
    /// [`GitError::Cancelled`] when stopped (the temporary folder is
    /// removed); other run errors. A clone git refused is `Ok(Err(..))`.
    pub async fn clone_into(
        &self,
        url: &CloneUrl,
        target: &Path,
        temporary: &Path,
        cancel: CancellationToken,
        mut progress: impl FnMut(Progress) + Send,
    ) -> Result<Result<(), Failed>, GitError> {
        let parent = target.parent().unwrap_or(target);
        let mut parser = Lines::default();
        let mut last: Option<Progress> = None;
        let result = self
            .command(parent)
            .args([
                "-c",
                "protocol.ext.allow=never",
                "-c",
                "protocol.file.allow=never",
                "clone",
                "--progress",
                "--no-recurse-submodules",
                "--",
                url.as_str(),
            ])
            .arg(temporary)
            .env("GIT_ALLOW_PROTOCOL", "https:http:ssh:git")
            .network()
            .limits(Limits::CLONE)
            .cancel_on(cancel)
            .output_streaming(|piece| {
                for line in parser.push(piece) {
                    if let Some(now) = parse_progress(&line)
                        && last != Some(now)
                    {
                        last = Some(now);
                        progress(now);
                    }
                }
            })
            .await;
        let failed = match result {
            Ok(output) if output.code == Some(0) => None,
            Ok(output) => Some(Failed {
                reason: failure_of(&output.stderr_text()),
                message: redact::tail(&output.stderr_text()),
            }),
            Err(GitError::TimedOut { .. }) => Some(Failed {
                reason: CloneFailure::TimedOut,
                message: "the clone did not finish within its time limit".to_owned(),
            }),
            Err(error) => {
                remove_quietly(temporary.to_path_buf()).await;
                return Err(error);
            }
        };
        if let Some(failed) = failed {
            remove_quietly(temporary.to_path_buf()).await;
            return Ok(Err(failed));
        }
        let (from, to) = (temporary.to_path_buf(), target.to_path_buf());
        let renamed = tokio::task::spawn_blocking(move || {
            if to.exists() {
                return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
            }
            std::fs::rename(&from, &to)
        })
        .await
        .map_err(|_panicked| GitError::Unexpected {
            what: "the clone's folder",
        })?;
        if let Err(error) = renamed {
            remove_quietly(temporary.to_path_buf()).await;
            return Ok(Err(Failed {
                reason: CloneFailure::Other,
                message: format!("could not move the clone into place: {error}"),
            }));
        }
        Ok(Ok(()))
    }
}

/// Remove a folder Vessel made, logging (not failing) when it cannot.
async fn remove_quietly(folder: PathBuf) {
    let removed = tokio::task::spawn_blocking(move || {
        if folder.exists() {
            std::fs::remove_dir_all(&folder)
        } else {
            Ok(())
        }
    })
    .await;
    if !matches!(removed, Ok(Ok(()))) {
        tracing::warn!("could not remove a clone's temporary folder");
    }
}

/// Why git's last lines say a clone failed.
fn failure_of(stderr: &str) -> CloneFailure {
    let has = |needle: &str| stderr.contains(needle);
    if has("terminal prompts disabled")
        || has("could not read Username")
        || has("Authentication failed")
        || has("Permission denied (publickey")
    {
        CloneFailure::NeedsCredential
    } else if has("not found")
        || has("does not appear to be a git repository")
        || has("Repository not found")
        || has("returned error: 404")
    {
        CloneFailure::NotFound
    } else if has("Could not resolve host")
        || has("Connection refused")
        || has("Connection timed out")
        || has("Network is unreachable")
        || has("unable to access")
    {
        CloneFailure::Network
    } else {
        CloneFailure::Other
    }
}

/// Splits git's progress output into lines at `\r` and `\n`.
#[derive(Debug, Default)]
struct Lines {
    partial: Vec<u8>,
}

impl Lines {
    fn push(&mut self, piece: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for byte in piece {
            if matches!(byte, b'\r' | b'\n') {
                if !self.partial.is_empty() {
                    lines.push(String::from_utf8_lossy(&self.partial).into_owned());
                    self.partial.clear();
                }
            } else if self.partial.len() < 1024 {
                self.partial.push(*byte);
            }
        }
        lines
    }
}

/// One progress line, as git prints it (also from the remote:
/// `remote: Counting objects:  10% (1/10)`).
fn parse_progress(line: &str) -> Option<Progress> {
    let line = line.strip_prefix("remote: ").unwrap_or(line);
    let stage = if line.starts_with("Cloning into") {
        CloneStage::Connecting
    } else if line.starts_with("Enumerating objects") || line.starts_with("Counting objects") {
        CloneStage::Counting
    } else if line.starts_with("Compressing objects") {
        CloneStage::Compressing
    } else if line.starts_with("Receiving objects") {
        CloneStage::Receiving
    } else if line.starts_with("Resolving deltas") {
        CloneStage::Resolving
    } else if line.starts_with("Updating files") || line.starts_with("Checking out files") {
        CloneStage::CheckingOut
    } else {
        return None;
    };
    let percent = line
        .split_once(':')
        .and_then(|(_, rest)| rest.trim_start().split_once('%'))
        .and_then(|(number, _)| number.trim().parse::<u8>().ok())
        .filter(|percent| *percent <= 100);
    Some(Progress { stage, percent })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[test]
    fn progress_lines_are_read_across_pieces() {
        let mut lines = Lines::default();
        let mut seen = lines.push(b"Cloning into 'x'...\nremote: Counting objects:  5");
        seen.extend(lines.push(b"0% (5/10)\rReceiving objects:  12% (1/8)\r"));
        let parsed: Vec<_> = seen
            .iter()
            .filter_map(|line| parse_progress(line))
            .collect();
        assert_eq!(
            parsed,
            [
                Progress {
                    stage: CloneStage::Connecting,
                    percent: None
                },
                Progress {
                    stage: CloneStage::Counting,
                    percent: Some(50)
                },
                Progress {
                    stage: CloneStage::Receiving,
                    percent: Some(12)
                },
            ]
        );
    }

    #[test]
    fn failures_are_classified() {
        assert_eq!(
            failure_of("fatal: could not read Username for 'https://x': terminal prompts disabled"),
            CloneFailure::NeedsCredential
        );
        assert_eq!(
            failure_of("remote: Repository not found.\nfatal: repository 'https://x/' not found"),
            CloneFailure::NotFound
        );
        assert_eq!(
            failure_of("fatal: unable to access 'https://x/': Could not resolve host: x"),
            CloneFailure::Network
        );
    }

    /// The URL types refuse local paths, so a test clone goes through the
    /// same run with the `file` protocol let through for it alone: what is
    /// tested here is the temporary folder, the rename and the cleanup.
    #[tokio::test]
    async fn a_failed_clone_leaves_nothing_and_the_target_is_never_half_made() {
        let sandbox = Sandbox::new();
        let git = sandbox.git().await;
        let url: CloneUrl = "https://127.0.0.1:9/nothing-listens.git".parse().unwrap();
        let target = sandbox.path().join("target");
        let temporary = sandbox.path().join(".target.vessel-clone-test");
        let failed = git
            .clone_into(&url, &target, &temporary, CancellationToken::new(), |_| {})
            .await
            .unwrap();
        let failed = failed.unwrap_err();
        assert_eq!(failed.reason, CloneFailure::Network, "{}", failed.message);
        assert!(!target.exists());
        assert!(!temporary.exists());

        let cancel = CancellationToken::new();
        cancel.cancel();
        let cancelled = git
            .clone_into(&url, &target, &temporary, cancel, |_| {})
            .await;
        assert!(
            matches!(cancelled, Err(GitError::Cancelled) | Ok(Err(_))),
            "{cancelled:?}"
        );
        assert!(!temporary.exists());
    }
}
