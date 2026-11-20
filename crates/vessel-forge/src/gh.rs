//! The machine's `gh`, asked for a host's token (ADR 0029) and how it
//! clones (ADR 0088). Nothing else of `gh` is used: requests are this
//! crate's own.
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
//! - `gh config get git_protocol` (ADR 0088) runs the same way: the
//!   repositories a search lists clone as the machine's `gh` would, as in
//!   Paseo; anything but `ssh`, or a failure, is `https`.
//! - Rejected: every call through `gh api` (Paseo): a process per request
//!   (about 30 ms of CPU each), and no control of conditional requests,
//!   time limits per request or rate-limit headers short of parsing its
//!   output.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::Semaphore;
use vessel_proc::{CommandSpec, Signal, Spawned};
use vessel_proto::forge::{ForgeHost, GhState};

use crate::token::Token;

/// How the machine clones from GitHub (`gh config get git_protocol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneProtocol {
    /// `https://github.com/owner/name`.
    Https,
    /// `git@github.com:owner/name.git`.
    Ssh,
}

/// What one run of `gh` gave.
struct Ran {
    /// Its exit code (`None`: ended by a signal).
    code: Option<i32>,
    out: Vec<u8>,
    err: Vec<u8>,
}

/// How long `gh` may take.
const TIME: Duration = Duration::from_secs(10);
/// How many `gh` may run at once.
const AT_ONCE: usize = 2;
/// The most of its output read: a token is short.
const OUTPUT: u64 = 16 * 1024;
/// How long a host's clone protocol is kept: a person typing a search asks
/// it per debounced keystroke (node review of ADR 0088, note 2).
const KEEP_PROTOCOL: Duration = Duration::from_secs(60);

/// The machine's `gh`.
#[derive(Debug, Clone)]
pub struct Gh {
    program: PathBuf,
    folder: PathBuf,
    slots: Arc<Semaphore>,
    /// Each host's clone protocol, and when it was read.
    protocols: Arc<Mutex<HashMap<ForgeHost, (CloneProtocol, Instant)>>>,
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
            protocols: Arc::new(Mutex::new(HashMap::new())),
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
        self.spec(&["auth", "token", "--hostname", host.as_str()])
    }

    /// `gh` with `args`, under the rules every run keeps.
    fn spec(&self, args: &[&str]) -> CommandSpec {
        crate::token::SIGN_IN_VARIABLES
            .iter()
            .chain(std::iter::once(&"GH_HOST"))
            .fold(
                CommandSpec::new(&self.program)
                    .cwd(&self.folder)
                    .args(args.iter().copied()),
                CommandSpec::env_remove,
            )
            .env("GH_DEBUG", "")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("NO_COLOR", "1")
    }

    /// How the machine clones from `host`: `ssh` only when `gh` says so
    /// (Paseo's `resolveConfiguredCloneProtocol`).
    pub async fn git_protocol(&self, host: &ForgeHost) -> CloneProtocol {
        let kept = self
            .protocols
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(host)
            .filter(|(_, at)| at.elapsed() < KEEP_PROTOCOL)
            .map(|(protocol, _)| *protocol);
        if let Some(protocol) = kept {
            return protocol;
        }
        let spec = self.spec(&["config", "get", "git_protocol", "--host", host.as_str()]);
        // Only an answer `gh` gave is kept: a `gh` that could not start or
        // ran past its time says nothing about the machine, and keeping
        // that as `https` would pin https URLs on an ssh machine for a
        // minute (node review, P1 note 1).
        let Some(ran) = self.run(&spec).await else {
            return CloneProtocol::Https;
        };
        let protocol = if ran.code == Some(0)
            && String::from_utf8_lossy(&ran.out)
                .trim()
                .eq_ignore_ascii_case("ssh")
        {
            CloneProtocol::Ssh
        } else {
            CloneProtocol::Https
        };
        let _old = self
            .protocols
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(host.clone(), (protocol, Instant::now()));
        protocol
    }

    /// Run `spec` within the time and the slots; `None` when it could not
    /// start or took too long (it is then killed and reaped).
    async fn run(&self, spec: &CommandSpec) -> Option<Ran> {
        let _slot = self.slots.acquire().await.ok()?;
        let Spawned {
            mut child,
            stdin,
            stdout,
            stderr,
        } = vessel_proc::spawn(spec)
            .inspect_err(|error| tracing::debug!(%error, "could not start gh"))
            .ok()?;
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
            return None;
        };
        let code = code
            .inspect_err(|error| tracing::debug!(%error, "could not wait for gh"))
            .ok()?;
        Some(Ran { code, out, err })
    }

    /// `host`'s token, or what stopped `gh` giving one.
    ///
    /// # Errors
    /// [`GhState::SignedOut`] when `gh` has none for `host`,
    /// [`GhState::Failed`] when it could not run or took too long.
    pub async fn token(&self, host: &ForgeHost) -> Result<Token, GhState> {
        let ran = self
            .run(&self.token_spec(host))
            .await
            .ok_or(GhState::Failed)?;
        if ran.code == Some(0) {
            return Token::new(&String::from_utf8_lossy(&ran.out)).ok_or(GhState::SignedOut);
        }
        // Its words only to tell signed out from broken; the token is never
        // in them (it printed none).
        let said = String::from_utf8_lossy(&ran.err).to_ascii_lowercase();
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
    use super::*;

    /// A stand-in `gh` that answers as the real one does.
    fn fake(dir: &std::path::Path, script: &str) -> Gh {
        let program = dir.join("gh");
        vessel_testkit::write_executable(&program, &format!("#!/bin/sh\n{script}\n")).unwrap();
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

    #[tokio::test]
    async fn gh_s_clone_protocol_is_ssh_only_when_it_says_so() {
        let host: ForgeHost = "github.com".parse().unwrap();
        let ssh_dir = tempfile::tempdir().unwrap();
        let ssh = fake(
            ssh_dir.path(),
            "[ \"$1 $2 $3 $4 $5\" = \"config get git_protocol --host github.com\" ] && echo SSH",
        );
        assert_eq!(ssh.git_protocol(&host).await, CloneProtocol::Ssh);
        // Kept a minute: gh is not asked again (its program is gone).
        std::fs::remove_file(ssh_dir.path().join("gh")).unwrap();
        assert_eq!(ssh.git_protocol(&host).await, CloneProtocol::Ssh);
        let unset_dir = tempfile::tempdir().unwrap();
        let unset = fake(unset_dir.path(), "echo");
        assert_eq!(unset.git_protocol(&host).await, CloneProtocol::Https);
        let broken_dir = tempfile::tempdir().unwrap();
        let broken = fake(broken_dir.path(), "echo ssh; exit 1");
        assert_eq!(broken.git_protocol(&host).await, CloneProtocol::Https);
        let missing = Gh::new(
            unset_dir.path().join("nowhere"),
            unset_dir.path().to_path_buf(),
        );
        assert_eq!(missing.git_protocol(&host).await, CloneProtocol::Https);
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
