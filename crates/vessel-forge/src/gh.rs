//! The machine's `gh`, asked for a host's token (ADR 0029). Nothing else
//! of `gh` is used: requests are this crate's own.
//!
//! [Decision Log]
//! - Purpose: most people who use GitHub from a terminal are signed in
//!   with `gh` (its keyring or its own file); asking it, as
//!   agent-orchestrator and GOAT do, needs no setting in Vessel.
//! - Choice: `gh auth token --hostname <host>`, no shell, in the node's
//!   data folder (never a repository's, whose configuration `gh` would
//!   read), with `GH_DEBUG` emptied (it would print the token, GOAT's
//!   trap), prompts and the update check off; ten seconds, two at once.
//!   Its standard output is the token and is never logged; its standard
//!   error is reduced to a [`GhState`].
//! - Rejected: every call through `gh api` (Paseo): a process per request
//!   (about 30 ms of CPU each), and no control of conditional requests,
//!   time limits per request or rate-limit headers short of parsing its
//!   output.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::Semaphore;
use vessel_proc::{CommandSpec, Signal, Spawned};
use vessel_proto::forge::{ForgeHost, GhState};

use crate::token::Token;

/// How long `gh` may take.
const TIME: Duration = Duration::from_secs(10);
/// How many `gh` may run at once.
const AT_ONCE: usize = 2;
/// The most of its output read: a token is short.
const OUTPUT: u64 = 16 * 1024;

/// The machine's `gh`.
#[derive(Debug, Clone)]
pub struct Gh {
    program: PathBuf,
    folder: PathBuf,
    slots: Arc<Semaphore>,
}

impl Gh {
    /// `program` (found by the binary on its search path), run in `folder`
    /// (the node's data folder: outside any repository).
    #[must_use]
    pub fn new(program: PathBuf, folder: PathBuf) -> Self {
        Self {
            program,
            folder,
            slots: Arc::new(Semaphore::new(AT_ONCE)),
        }
    }

    /// The command that asks `gh` for `host`'s token. Without the token
    /// variables and `GH_HOST` (the join of wave 2's first half): `gh`
    /// answers `auth token` from them before its stored logins, an
    /// enterprise token for every host that is not `github.com`, so a host
    /// a remote names (an agent can set a remote) would be handed the
    /// node's enterprise token. The node reads those variables itself, for
    /// the hosts they are for (`EnvTokens`); `gh` answers only from the
    /// logins it stores per host.
    fn token_spec(&self, host: &ForgeHost) -> CommandSpec {
        crate::token::SIGN_IN_VARIABLES
            .iter()
            .chain(std::iter::once(&"GH_HOST"))
            .fold(
                CommandSpec::new(&self.program)
                    .cwd(&self.folder)
                    .args(["auth", "token", "--hostname"])
                    .arg(host.as_str()),
                CommandSpec::env_remove,
            )
            .env("GH_DEBUG", "")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("NO_COLOR", "1")
    }

    /// `host`'s token, or what stopped `gh` giving one.
    ///
    /// # Errors
    /// [`GhState::SignedOut`] when `gh` has none for `host`,
    /// [`GhState::Failed`] when it could not run or took too long.
    pub async fn token(&self, host: &ForgeHost) -> Result<Token, GhState> {
        let _slot = self
            .slots
            .acquire()
            .await
            .map_err(|_closed| GhState::Failed)?;
        let spec = self.token_spec(host);
        let Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        } = vessel_proc::spawn(&spec).map_err(|error| {
            tracing::debug!(%error, "could not start gh");
            GhState::Failed
        })?;
        drop(stdin);
        let read = async {
            let (out, err) = tokio::join!(read_some(stdout), read_some(stderr));
            let code = child.wait().await;
            (out, err, code)
        };
        let Ok((out, err, code)) = tokio::time::timeout(TIME, read).await else {
            if let Err(error) = child.signal(Signal::Kill) {
                tracing::debug!(%error, "gh had already ended");
            }
            let _reaped = child.wait().await;
            return Err(GhState::Failed);
        };
        match code {
            Ok(Some(0)) => Token::new(&String::from_utf8_lossy(&out)).ok_or(GhState::SignedOut),
            Ok(_) => {
                // Its words only to tell signed out from broken; the
                // token is never in them (it printed none).
                let said = String::from_utf8_lossy(&err).to_ascii_lowercase();
                if said.contains("no oauth token")
                    || said.contains("not logged")
                    || said.contains("gh auth login")
                {
                    Err(GhState::SignedOut)
                } else {
                    tracing::debug!("gh auth token failed");
                    Err(GhState::Failed)
                }
            }
            Err(error) => {
                tracing::debug!(%error, "could not wait for gh");
                Err(GhState::Failed)
            }
        }
    }
}

/// At most [`OUTPUT`] bytes of `stream`.
async fn read_some(stream: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Err(error) = stream.take(OUTPUT).read_to_end(&mut bytes).await {
        tracing::debug!(%error, "reading gh's output failed");
    }
    bytes
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// A stand-in `gh` that answers as the real one does.
    fn fake(dir: &std::path::Path, script: &str) -> Gh {
        let program = dir.join("gh");
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        Gh::new(program, dir.to_path_buf())
    }

    #[tokio::test]
    async fn gh_s_token_signed_out_and_failure_are_told_apart() {
        let temp = tempfile::tempdir().unwrap();
        let host: ForgeHost = "github.com".parse().unwrap();
        let signed_in = fake(
            temp.path(),
            "[ \"$GH_DEBUG\" = \"\" ] && [ \"$3\" = \"--hostname\" ] && echo gho_test_token",
        );
        assert!(signed_in.token(&host).await.is_ok());
        let out = tempfile::tempdir().unwrap();
        let signed_out = fake(
            out.path(),
            "echo 'no oauth token found for github.com' >&2; exit 1",
        );
        assert_eq!(
            signed_out.token(&host).await.unwrap_err(),
            GhState::SignedOut
        );
        let broken = tempfile::tempdir().unwrap();
        let failed = fake(broken.path(), "echo 'segfault' >&2; exit 2");
        assert_eq!(failed.token(&host).await.unwrap_err(), GhState::Failed);
        let missing = Gh::new(temp.path().join("nowhere"), temp.path().to_path_buf());
        assert_eq!(missing.token(&host).await.unwrap_err(), GhState::Failed);
    }

    /// `gh` answers from the environment's tokens first (an enterprise
    /// one for any host but `github.com`): it never sees them, so a host
    /// a remote names gets only what `gh` stored for it.
    #[test]
    fn gh_never_sees_the_environment_s_tokens() {
        let gh = Gh::new("/usr/bin/gh".into(), "/data".into());
        let spec = gh.token_spec(&"evil.example".parse().unwrap());
        for name in crate::token::SIGN_IN_VARIABLES
            .iter()
            .chain(std::iter::once(&"GH_HOST"))
        {
            assert_eq!(
                spec.env_change(name),
                Some(&vessel_proc::EnvValue::Remove),
                "{name}"
            );
        }
    }
}
