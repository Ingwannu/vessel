//! `agent.status` (protocol 11, ADR 0032): whether each agent is installed
//! and signed in under each profile, as typed facts with a reason when one
//! is not known.
//!
//! [Decision Log]
//! - Purpose: a person sees, before a session fails, that an agent is
//!   missing, speaks another ACP version, or needs a sign-in, and how to
//!   sign in.
//! - Constraint: no turn (a turn costs the owner money), and no check while
//!   a session of that profile runs: a status command or a handshake can
//!   refresh a token the session is using. A check's output may hold an
//!   account's address: it never leaves the node, only the answer.
//! - Alternatives: checking on every call (an agent start per profile per
//!   page view); at node start (stale after a sign-in, and slow starts);
//!   inferring sign-in from a failed session (too late, and a turn spent).
//! - Choice: whether it is installed is found on every call (as
//!   `harness.list` does); whether it is signed in, and what its handshake
//!   said, only when a person asks (`refresh`, which an agent may not), then
//!   kept here by agent and profile, with when it was found. A refresh runs
//!   the agent's own status command when it has one (Claude Code, Codex;
//!   `vessel_harness::read_login_status`), then a handshake without a
//!   prompt (`vessel_agent::probe`): `auth_required` there says "not signed
//!   in" whatever the command said, and its `initialize` gives an added
//!   agent's version and the sign-in methods it offers.
//! - Bounds: one entry per agent and profile, dropped when the agent is
//!   changed or removed; past [`MAX_KEPT`] the oldest go. The status
//!   command is bounded by [`STATUS_TIMEOUT`] and its output by
//!   `vessel_harness::MAX_STATUS_OUTPUT`; the handshake by the node's
//!   handshake timeout.

pub(crate) mod diagnostics;

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::io::AsyncReadExt as _;
use vessel_agent::CommandSpec;
use vessel_agent::{Introduction, Refusal};
use vessel_harness::{LoginStatus, MAX_STATUS_OUTPUT};
use vessel_proto::agents::{
    AgentProgram, AgentVersion, CheckStep, Installed, SignInHelp, SignedIn, UnknownReason,
};
use vessel_proto::{DurationMs, Harness, HarnessKind, HarnessRef, ProfileName};

/// How long a status command may take.
pub(crate) const STATUS_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a status command's answer is kept.
pub(crate) const KEPT_STATUS: Duration = Duration::from_secs(5 * 60);
/// How long a handshake's answer is kept (the probes' own time).
pub(crate) const KEPT_PROBE: Duration = crate::probes::FRESH;
/// A refresh of one agent and profile runs at most this often; one
/// sooner answers what was kept.
pub(crate) const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// The most agent and profile pairs kept.
const MAX_KEPT: usize = 1024;

/// One agent under one profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub(crate) agent: HarnessRef,
    pub(crate) profile: ProfileName,
}

/// What the last refresh found.
#[derive(Debug, Clone)]
pub(crate) struct Checked {
    pub(crate) signed_in: SignedIn,
    /// What its handshake said, when it got that far.
    pub(crate) handshake: Option<Handshake>,
    pub(crate) at: DateTime<Utc>,
    /// Kept until then.
    pub(crate) until: DateTime<Utc>,
}

impl Checked {
    /// Found `at` now, kept for `kept`.
    pub(crate) fn now(signed_in: SignedIn, handshake: Option<Handshake>, kept: Duration) -> Self {
        let at = Utc::now();
        let until = chrono::Duration::from_std(kept)
            .ok()
            .and_then(|kept| at.checked_add_signed(kept))
            .unwrap_or(at);
        Self {
            signed_in,
            handshake,
            at,
            until,
        }
    }

    /// Whether a refresh now would run again.
    pub(crate) fn recent(&self) -> bool {
        chrono::Duration::from_std(REFRESH_EVERY)
            .ok()
            .and_then(|every| self.at.checked_add_signed(every))
            .is_some_and(|next| Utc::now() < next)
    }
}

/// What a handshake said of the agent.
#[derive(Debug, Clone)]
pub(crate) enum Handshake {
    /// It speaks Vessel's ACP version.
    Spoke(Introduction),
    /// It speaks another.
    OtherVersion {
        speaks: u16,
        introduction: Introduction,
    },
}

/// The facts kept from refreshes.
#[derive(Debug, Default)]
pub(crate) struct Readiness {
    kept: Mutex<HashMap<Key, Checked>>,
}

impl Readiness {
    /// What was found of `key` and is still kept.
    pub(crate) fn get(&self, key: &Key) -> Option<Checked> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        match kept.get(key) {
            Some(checked) if Utc::now() <= checked.until => Some(checked.clone()),
            Some(_) => {
                drop(kept.remove(key));
                None
            }
            None => None,
        }
    }

    pub(crate) fn keep(&self, key: Key, checked: Checked) {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.len() >= MAX_KEPT && !kept.contains_key(&key) {
            let oldest = kept
                .iter()
                .min_by_key(|(_key, checked)| checked.at)
                .map(|(key, _checked)| key.clone());
            if let Some(oldest) = oldest {
                drop(kept.remove(&oldest));
            }
        }
        drop(kept.insert(key, checked));
    }

    /// Forget what was found of `agent` (it was changed or removed).
    pub(crate) fn forget(&self, agent: &HarnessRef) {
        self.kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|key, _checked| key.agent != *agent);
    }
}

/// Whether the agent is installed: from `found` (this call's discovery)
/// and what its last handshake said.
pub(crate) fn installed(
    found: Option<&Harness>,
    program: Option<AgentProgram>,
    checked: Option<&Checked>,
) -> Installed {
    let Some(found) = found else {
        return match program {
            Some(program) => Installed::Missing { program },
            None => Installed::Unknown {
                reason: UnknownReason::NotInstalled,
            },
        };
    };
    let said = checked.and_then(|checked| checked.handshake.as_ref());
    // The four's `--version`; an added agent's own word in its handshake.
    let version = found
        .version
        .as_deref()
        .and_then(|version| version.parse::<AgentVersion>().ok())
        .or_else(|| match said {
            Some(Handshake::Spoke(introduction) | Handshake::OtherVersion { introduction, .. }) => {
                introduction.version.clone()
            }
            None => None,
        });
    let path = found.path.clone();
    let needs = vessel_agent::SPEAKS;
    match said {
        Some(Handshake::OtherVersion { speaks, .. }) if *speaks < needs => Installed::TooOld {
            path,
            version,
            speaks: *speaks,
            needs,
        },
        Some(Handshake::OtherVersion { speaks, .. }) => Installed::TooNew {
            path,
            version,
            speaks: *speaks,
            needs,
        },
        Some(Handshake::Spoke(_)) | None => Installed::Found {
            path,
            version,
            structured: found.structured.clone(),
        },
    }
}

/// What a status command said, as a fact.
pub(crate) async fn run_status(command: CommandSpec, harness: HarnessKind) -> SignedIn {
    let mut command = tokio::process::Command::from(command.to_command());
    let _configured = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let Ok(mut child) = command.spawn() else {
        return unknown(UnknownReason::Failed {
            step: CheckStep::Spawn,
        });
    };
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return unknown(UnknownReason::Failed {
            step: CheckStep::Spawn,
        });
    };
    let limit = u64::try_from(MAX_STATUS_OUTPUT)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let read = async {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (mut stdout, mut stderr) = (stdout.take(limit), stderr.take(limit));
        let (read_out, read_err) =
            tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
        let status = child.wait().await;
        (read_out.and(read_err).and(status), out, err)
    };
    match tokio::time::timeout(STATUS_TIMEOUT, read).await {
        Err(_elapsed) => unknown(UnknownReason::TimedOut {
            after: DurationMs::from_duration(STATUS_TIMEOUT),
        }),
        Ok((Err(_error), _out, _err)) => unknown(UnknownReason::Failed {
            step: CheckStep::Exited,
        }),
        Ok((Ok(_status), out, err)) => {
            match vessel_harness::read_login_status(harness, &out, &err) {
                LoginStatus::Yes => SignedIn::Yes,
                LoginStatus::No => SignedIn::No {
                    help: SignInHelp::default(),
                },
                LoginStatus::Unread => unknown(UnknownReason::Unread),
            }
        }
    }
}

/// What a handshake's outcome says, with `command` what the status command
/// said (when there is one): its signed-in fact, and what it said of the
/// agent.
pub(crate) fn from_handshake(
    outcome: Result<Introduction, HandshakeFailure>,
    command: Option<SignedIn>,
    help: &SignInHelp,
) -> (SignedIn, Option<Handshake>) {
    let with_methods = |introduction: &Introduction| SignInHelp {
        methods: introduction.methods.clone(),
        ..help.clone()
    };
    match outcome {
        Ok(introduction) => {
            let signed_in = match command {
                Some(SignedIn::No { .. }) => SignedIn::No {
                    help: with_methods(&introduction),
                },
                Some(said) => said,
                None => unknown(UnknownReason::NotSaid),
            };
            (signed_in, Some(Handshake::Spoke(introduction)))
        }
        Err(HandshakeFailure::Refused {
            refusal: Refusal::SignIn,
            introduction,
        }) => (
            SignedIn::No {
                help: with_methods(&introduction),
            },
            Some(Handshake::Spoke(introduction)),
        ),
        Err(HandshakeFailure::Refused {
            refusal: Refusal::Version { speaks },
            introduction,
        }) => (
            command.unwrap_or_else(|| {
                unknown(UnknownReason::Failed {
                    step: CheckStep::Initialize,
                })
            }),
            Some(Handshake::OtherVersion {
                speaks,
                introduction,
            }),
        ),
        Err(failure) => {
            let reason = match failure {
                HandshakeFailure::Refused {
                    refusal: Refusal::Initialize,
                    ..
                } => UnknownReason::Failed {
                    step: CheckStep::Initialize,
                },
                HandshakeFailure::Refused {
                    refusal: Refusal::TimedOut { after },
                    ..
                } => UnknownReason::TimedOut {
                    after: DurationMs::from_duration(after),
                },
                HandshakeFailure::Refused {
                    refusal: Refusal::SessionNew | Refusal::SignIn | Refusal::Version { .. },
                    ..
                } => UnknownReason::Failed {
                    step: CheckStep::SessionNew,
                },
                HandshakeFailure::TimedOut { after } => UnknownReason::TimedOut { after },
                HandshakeFailure::Spawn => UnknownReason::Failed {
                    step: CheckStep::Spawn,
                },
                HandshakeFailure::Exited => UnknownReason::Failed {
                    step: CheckStep::Exited,
                },
            };
            (command.unwrap_or_else(|| unknown(reason)), None)
        }
    }
}

/// What a probe's handshake said, or how it failed.
pub(crate) type Said = Result<Introduction, HandshakeFailure>;

/// How a probe of `error` failed, as `agent.status` reads it.
pub(crate) fn failure_of(error: &crate::error::ServiceError) -> HandshakeFailure {
    let crate::error::ServiceError::Session(vessel_session::SessionError::Probe(probe)) = error
    else {
        // Its launch: not installed, a secret missing, a profile gone.
        return HandshakeFailure::Spawn;
    };
    match probe {
        vessel_agent::ProbeError::Spawn(_)
        | vessel_agent::ProbeError::CatalogUnsupported { .. } => HandshakeFailure::Spawn,
        vessel_agent::ProbeError::CatalogTimedOut { limit }
        | vessel_agent::ProbeError::TimedOut { limit, .. } => HandshakeFailure::TimedOut {
            after: DurationMs::from_duration(*limit),
        },
        vessel_agent::ProbeError::Disconnected { disconnect, .. } => match disconnect {
            vessel_agent::Disconnect::Handshake {
                refused: Some(refused),
                ..
            } => match &refused.refusal {
                Refusal::TimedOut { after } => HandshakeFailure::TimedOut {
                    after: DurationMs::from_duration(*after),
                },
                Refusal::Version { .. }
                | Refusal::SignIn
                | Refusal::Initialize
                | Refusal::SessionNew => HandshakeFailure::Refused {
                    refusal: refused.refusal.clone(),
                    introduction: refused.introduction.clone(),
                },
            },
            vessel_agent::Disconnect::Handshake { refused: None, .. }
            | vessel_agent::Disconnect::Closed
            | vessel_agent::Disconnect::Protocol { .. } => HandshakeFailure::Exited,
        },
    }
}

/// How a handshake for `agent.status` failed.
#[derive(Debug, Clone)]
pub(crate) enum HandshakeFailure {
    /// The agent refused it, for a reason Vessel reads.
    Refused {
        refusal: Refusal,
        introduction: Introduction,
    },
    /// It did not complete in time.
    TimedOut { after: DurationMs },
    /// The agent (or its launch) did not start.
    Spawn,
    /// It went away before answering.
    Exited,
}

const fn unknown(reason: UnknownReason) -> SignedIn {
    SignedIn::Unknown { reason }
}

/// The earlier of two instants.
pub(crate) fn older(a: DateTime<Utc>, b: Option<DateTime<Utc>>) -> DateTime<Utc> {
    b.map_or(a, |b| a.min(b))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vessel_proto::agents::AuthMethod;
    use vessel_proto::{StructuredSupport, TerminalSupport};

    use super::*;

    fn found(version: Option<&str>) -> Harness {
        Harness {
            kind: HarnessKind::Gemini.into(),
            path: PathBuf::from("/bin/agent"),
            version: version.map(str::to_owned),
            structured: StructuredSupport::Available,
            terminal: TerminalSupport::Available,
        }
    }

    fn introduction() -> Introduction {
        Introduction {
            version: Some("2.0.0".parse().unwrap()),
            methods: vec![AuthMethod {
                id: "login".parse().unwrap(),
                name: "Log in".to_owned(),
                description: None,
            }],
        }
    }

    fn checked(handshake: Handshake) -> Checked {
        Checked::now(
            SignedIn::Unknown {
                reason: UnknownReason::NotSaid,
            },
            Some(handshake),
            KEPT_PROBE,
        )
    }

    #[test]
    fn another_acp_version_is_too_old_or_too_new_and_a_version_comes_from_the_handshake() {
        let program = || Some("agent".parse().unwrap());
        assert_eq!(
            installed(None, program(), None),
            Installed::Missing {
                program: "agent".parse().unwrap()
            }
        );
        // No `--version` (an added agent): its handshake's word.
        let spoke = checked(Handshake::Spoke(introduction()));
        let Installed::Found { version, .. } =
            installed(Some(&found(None)), program(), Some(&spoke))
        else {
            panic!("found");
        };
        assert_eq!(version.unwrap().as_str(), "2.0.0");
        for (speaks, newer) in [(0, false), (2, true)] {
            let other = checked(Handshake::OtherVersion {
                speaks,
                introduction: introduction(),
            });
            let fact = installed(Some(&found(Some("1.0"))), program(), Some(&other));
            let expected = if newer {
                matches!(
                    fact,
                    Installed::TooNew {
                        speaks: 2,
                        needs: 1,
                        ..
                    }
                )
            } else {
                matches!(
                    fact,
                    Installed::TooOld {
                        speaks: 0,
                        needs: 1,
                        ..
                    }
                )
            };
            assert!(expected, "{speaks}: {fact:?}");
        }
    }

    #[test]
    fn a_handshake_asking_for_a_sign_in_wins_and_lists_its_methods() {
        let help = SignInHelp::default();
        let refused = Err(HandshakeFailure::Refused {
            refusal: Refusal::SignIn,
            introduction: introduction(),
        });
        let (signed_in, _said) = from_handshake(refused, Some(SignedIn::Yes), &help);
        let SignedIn::No { help } = signed_in else {
            panic!("not signed in: {signed_in:?}");
        };
        assert_eq!(help.methods[0].id.as_str(), "login");
        let (signed_in, said) = from_handshake(Ok(introduction()), None, &help);
        assert_eq!(
            signed_in,
            SignedIn::Unknown {
                reason: UnknownReason::NotSaid
            }
        );
        assert!(matches!(said, Some(Handshake::Spoke(_))));
        let timed_out = Err(HandshakeFailure::TimedOut {
            after: DurationMs::from_duration(Duration::from_secs(60)),
        });
        assert!(matches!(
            from_handshake(timed_out, None, &help).0,
            SignedIn::Unknown {
                reason: UnknownReason::TimedOut { .. }
            }
        ));
    }

    #[test]
    fn what_is_kept_goes_with_its_agent_and_past_its_time() {
        let readiness = Readiness::default();
        let key = Key {
            agent: HarnessKind::Gemini.into(),
            profile: "gemini".parse().unwrap(),
        };
        readiness.keep(key.clone(), checked(Handshake::Spoke(introduction())));
        assert!(readiness.get(&key).is_some_and(|kept| kept.recent()));
        readiness.forget(&HarnessKind::Gemini.into());
        assert!(readiness.get(&key).is_none());
        let mut stale = checked(Handshake::Spoke(introduction()));
        stale.until = stale.at - chrono::Duration::seconds(1);
        readiness.keep(key.clone(), stale);
        assert!(readiness.get(&key).is_none(), "past its time");
    }
}
