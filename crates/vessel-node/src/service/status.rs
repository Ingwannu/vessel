//! `agent.status` (protocol 11, ADR 0032), as the service routes it: which
//! agents and profiles are asked about, who may refresh, and running the
//! checks. The facts and how they are read are `crate::readiness`'s.

use vessel_harness::Profile;
use vessel_proto::agents::{AgentReadiness, GetAgentStatus, SignInHelp, SignedIn, UnknownReason};
use vessel_proto::{ErrorCode, Harness, HarnessKind, HarnessRef, WireError};

use super::{Connection, NodeService};
use crate::readiness::{self, Checked, KEPT_PROBE, KEPT_STATUS, Key};

impl NodeService {
    /// `agent.status`.
    pub(crate) async fn agent_status(
        &self,
        connection: &Connection,
        request: GetAgentStatus,
    ) -> Result<Vec<AgentReadiness>, WireError> {
        let GetAgentStatus {
            agent,
            profile,
            refresh,
        } = request;
        if refresh {
            crate::added::person(connection.principal()).map_err(|_not| {
                WireError::new(
                    ErrorCode::Forbidden,
                    "a refresh runs the agent's checks: only a person asks for one; an agent \
                     reads what was found",
                )
            })?;
        }
        let pairs = self.status_pairs(agent.as_ref(), profile.as_ref(), refresh)?;
        let found = self.harnesses().await;
        if refresh {
            for (agent, profile) in &pairs {
                let installed = found.iter().any(|harness| harness.kind == *agent);
                self.refresh_status(agent, profile, installed).await;
            }
        }
        Ok(pairs
            .iter()
            .map(|(agent, profile)| self.readiness_of(agent, profile, &found))
            .collect())
    }

    /// The agents and profiles asked about: one, or every agent's default.
    fn status_pairs(
        &self,
        agent: Option<&HarnessRef>,
        profile: Option<&vessel_proto::ProfileName>,
        refresh: bool,
    ) -> Result<Vec<(HarnessRef, Profile)>, WireError> {
        let invalid = |message: &str| WireError::new(ErrorCode::InvalidRequest, message);
        let current = self.profiles.current();
        let Some(agent) = agent else {
            if refresh || profile.is_some() {
                return Err(invalid(
                    "a refresh, or a profile, names its agent (`agent`)",
                ));
            }
            let all = HarnessKind::ALL
                .into_iter()
                .map(HarnessRef::Known)
                .chain(
                    current
                        .added()
                        .all()
                        .map(|added| HarnessRef::Added(added.name().clone())),
                )
                .filter_map(|agent| {
                    let profile = current.default_of(&agent)?;
                    Some((agent, profile))
                })
                .collect();
            return Ok(all);
        };
        let profile = match profile {
            None => current.default_of(agent).ok_or_else(|| {
                WireError::new(
                    ErrorCode::NotFound,
                    format!("this node has no agent `{agent}`"),
                )
            })?,
            Some(name) => {
                let profile = current.get(name).cloned().ok_or_else(|| {
                    WireError::new(
                        ErrorCode::NotFound,
                        format!("this node has no profile `{name}`"),
                    )
                })?;
                if profile.harness() != *agent {
                    return Err(invalid(&format!(
                        "profile `{name}` is for {}, not {agent}",
                        profile.harness()
                    )));
                }
                profile
            }
        };
        Ok(vec![(agent.clone(), profile)])
    }

    /// The facts of `agent` under `profile` now.
    fn readiness_of(
        &self,
        agent: &HarnessRef,
        profile: &Profile,
        found: &[Harness],
    ) -> AgentReadiness {
        let key = Key {
            agent: agent.clone(),
            profile: profile.name().clone(),
        };
        let kept = self.readiness.get(&key);
        let here = found.iter().find(|harness| harness.kind == *agent);
        let program = match agent {
            HarnessRef::Known(kind) => vessel_harness::binary(*kind).parse().ok(),
            HarnessRef::Added(_) => profile.added().map(|added| added.program().clone()),
        };
        let installed = readiness::installed(here, program, kept.as_ref());
        let signed_in = if profile.model().is_some() {
            SignedIn::NotRequired
        } else if here.is_none() {
            SignedIn::Unknown {
                reason: UnknownReason::NotInstalled,
            }
        } else {
            kept.as_ref().map_or(
                SignedIn::Unknown {
                    reason: UnknownReason::NotChecked,
                },
                |kept| kept.signed_in.clone(),
            )
        };
        let now = chrono::Utc::now();
        AgentReadiness {
            agent: agent.clone(),
            profile: profile.name().clone(),
            installed,
            signed_in,
            checked_at: readiness::older(now, kept.map(|kept| kept.at)),
        }
    }

    /// Run `agent`'s checks under `profile` and keep what they say: its
    /// status command (Claude Code, Codex), or a handshake (the others).
    async fn refresh_status(&self, agent: &HarnessRef, profile: &Profile, installed: bool) {
        let key = Key {
            agent: agent.clone(),
            profile: profile.name().clone(),
        };
        let kept = self.readiness.get(&key);
        if kept.as_ref().is_some_and(Checked::recent) || !installed || profile.model().is_some() {
            return;
        }
        let help = match agent {
            HarnessRef::Known(kind) => crate::added::sign_in_help(*kind),
            HarnessRef::Added(_) => SignInHelp::default(),
        };
        let checked = match agent {
            HarnessRef::Known(kind @ (HarnessKind::ClaudeCode | HarnessKind::Codex)) => {
                let signed_in = self.status_command(*kind, profile, &help).await;
                let handshake = kept.and_then(|kept| kept.handshake);
                Checked::now(signed_in, handshake, KEPT_STATUS)
            }
            // The seven of ADR 0058 get their own sign-in step with their
            // readiness slice; until then their handshake says, as for an
            // added agent's.
            HarnessRef::Known(
                HarnessKind::Gemini
                | HarnessKind::OpenCode
                | HarnessKind::Grok
                | HarnessKind::Pi
                | HarnessKind::Omp
                | HarnessKind::Antigravity
                | HarnessKind::Muse
                | HarnessKind::Amp
                | HarnessKind::Cursor,
            )
            | HarnessRef::Added(_) => {
                let said = self.said(agent, profile).await;
                let (signed_in, handshake) = readiness::from_handshake(said, None, &help);
                Checked::now(signed_in, handshake, KEPT_PROBE)
            }
        };
        self.readiness.keep(key, checked);
    }

    /// What `kind`'s status command says of `profile`, unless a session of
    /// that profile runs.
    async fn status_command(
        &self,
        kind: HarnessKind,
        profile: &Profile,
        help: &SignInHelp,
    ) -> SignedIn {
        let unknown = |reason| SignedIn::Unknown { reason };
        let running = self.sessions.live_profiles();
        let wanted = (!profile.is_default()).then(|| profile.name().clone());
        if running
            .iter()
            .any(|(agent, name)| *agent == kind && *name == wanted)
        {
            return unknown(UnknownReason::InUse);
        }
        if self.profiles.home(profile).is_none() {
            return unknown(UnknownReason::NoHome);
        }
        let (lookup, env, owned) = (
            self.lookup.clone(),
            self.profiles.env().clone(),
            profile.clone(),
        );
        let check =
            tokio::task::spawn_blocking(move || vessel_harness::login_check(&lookup, &env, &owned))
                .await;
        let Ok(Ok(Some(command))) = check else {
            return unknown(UnknownReason::Failed {
                step: vessel_proto::agents::CheckStep::Spawn,
            });
        };
        match readiness::run_status(command, kind).await {
            SignedIn::No { .. } => SignedIn::No { help: help.clone() },
            said @ (SignedIn::Yes | SignedIn::NotRequired | SignedIn::Unknown { .. }) => said,
        }
    }

    /// What a handshake of `agent` under `profile` says, from a probe
    /// (ADR 0022's, sharing its slot and its ten minutes).
    async fn said(&self, agent: &HarnessRef, profile: &Profile) -> readiness::Said {
        let resolved = match self.resolve(agent, (!profile.is_default()).then(|| profile.name())) {
            Ok(resolved) => resolved,
            Err(_error) => return Err(readiness::HandshakeFailure::Spawn),
        };
        self.probed(
            agent.clone(),
            resolved,
            (!profile.is_default()).then(|| profile.name().clone()),
        )
        .await
        .said
    }
}
