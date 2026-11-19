//! `agent.diagnostic` (ADR 0111, TRACKER 568, 67 and 87) as the service
//! routes it: the agent's facts as `agent.status` reads them, every match
//! for its program on the search path, and a new probe of its catalogue
//! whose answer `harness.options` takes next (`Probes::refresh`).
//!
//! [Decision Log]
//! - Purpose: Paseo's provider diagnostic, from the facts the node already
//!   keeps and one new probe, so a person sees why an agent does not work
//!   here.
//! - Existing assets: `status_pairs`, `refresh_status` and `readiness_of`
//!   (`agent.status`), `probed_with` (`harness.options`'s probe and its
//!   slots), `Lookup::find_all`.
//! - Choice: the probe's steps are read from what the probe already
//!   records (its handshake's outcome and the options' error), not from new
//!   instrumentation of the agent driver; a step's reason is the options'
//!   error when there is one, since that is the message `harness.options`
//!   gives a form today.
//! - Paths in the reply have the node user's home folded to `~`.

use std::path::{Path, PathBuf};

use chrono::Utc;
use tokio::time::Instant;
use vessel_agent::{Refusal, SPEAKS};
use vessel_proto::agents::{
    AgentDiagnostic, AgentProbe, CatalogStatus, CommandSource, DiagnosticCommand,
    GetAgentDiagnostic, Installed, MAX_MATCHES, MAX_TEXT, MAX_TITLE, ProbeStep, bounded_text,
};
use vessel_proto::{
    DurationMs, ErrorCode, HarnessKind, HarnessOptions, HarnessRef, OptionCategory, OptionKind,
    StructuredSupport, WireError,
};

use super::{Connection, NodeService, wire};
use crate::probes::Answer;
use crate::readiness::HandshakeFailure;

impl NodeService {
    /// `agent.diagnostic`.
    pub(crate) async fn agent_diagnostic(
        &self,
        _: &Connection,
        request: GetAgentDiagnostic,
    ) -> Result<AgentDiagnostic, WireError> {
        let GetAgentDiagnostic { agent, profile } = request;
        self.refuse_installing(&agent)
            .map_err(|error| wire(&error))?;
        let Some((agent, chosen)) = self
            .status_pairs(Some(&agent), profile.as_ref(), true)?
            .into_iter()
            .next()
        else {
            return Err(WireError::new(
                ErrorCode::Internal,
                "an agent's status names its agent",
            ));
        };
        let found = self.harnesses().await;
        let here = found.iter().any(|harness| harness.kind == agent);
        self.refresh_status(&agent, &chosen, here).await;
        let readiness = self.readiness_of(&agent, &chosen, &found);
        let command = command(&agent, &chosen);
        let matches = {
            let (lookup, program) = (self.lookup.clone(), command.program.clone());
            tokio::task::spawn_blocking(move || lookup.find_all(&program, MAX_MATCHES))
                .await
                .unwrap_or_default()
        };
        let resolved = self
            .resolve(&agent, profile.as_ref())
            .map_err(|error| wire(&error))?;
        let runs_here = matches!(
            readiness.installed,
            Installed::Found {
                structured: StructuredSupport::Available,
                ..
            } | Installed::TooOld { .. }
                | Installed::TooNew { .. }
        );
        let started = Instant::now();
        let answer = self
            .probed_with(agent.clone(), resolved, profile, true)
            .await;
        let took = DurationMs::from_duration(started.elapsed());
        let structured_unavailable = matches!(
            readiness.installed,
            Installed::Found {
                structured: StructuredSupport::Unavailable { .. },
                ..
            }
        );
        let probe = if structured_unavailable {
            // Nothing was started: the agent runs no conversations here.
            AgentProbe {
                spawn: ProbeStep::NotReached,
                initialize: ProbeStep::NotReached,
                session: ProbeStep::NotReached,
                agent_version: None,
                took,
            }
        } else {
            probe(&answer, took)
        };
        Ok(AgentDiagnostic {
            title: bounded_text(&title(&agent, &chosen), MAX_TITLE),
            profile: chosen.name().clone(),
            agent,
            command,
            matches: matches.iter().map(|path| folded(path)).collect(),
            installed: folded_installed(readiness.installed),
            signed_in: readiness.signed_in,
            probe,
            status: status(&answer.options, runs_here),
            checked_at: Utc::now(),
        })
    }
}

/// Paseo's label for one of the four; an added agent's label (its name
/// without one), marked as an ACP agent, as Paseo's generic ACP provider.
fn title(agent: &HarnessRef, profile: &vessel_harness::Profile) -> String {
    match agent {
        HarnessRef::Known(HarnessKind::ClaudeCode) => "Claude Code".to_owned(),
        HarnessRef::Known(HarnessKind::Codex) => "Codex".to_owned(),
        HarnessRef::Known(HarnessKind::Gemini) => "Gemini CLI".to_owned(),
        HarnessRef::Known(HarnessKind::OpenCode) => "OpenCode".to_owned(),
        HarnessRef::Added(name) => {
            let label = profile
                .added()
                .and_then(|added| added.info().label)
                .map_or_else(|| name.to_string(), |label| label.to_string());
            format!("{label} (ACP)")
        }
    }
}

fn command(agent: &HarnessRef, profile: &vessel_harness::Profile) -> DiagnosticCommand {
    match (agent, profile.added()) {
        (HarnessRef::Known(kind), _) => DiagnosticCommand {
            source: CommandSource::BuiltIn,
            program: vessel_harness::binary(*kind).to_owned(),
            args: Vec::new(),
        },
        (HarnessRef::Added(_), Some(added)) => {
            let info = added.info();
            DiagnosticCommand {
                source: CommandSource::Added,
                program: info.program.to_string(),
                args: info.args,
            }
        }
        (HarnessRef::Added(name), None) => DiagnosticCommand {
            source: CommandSource::Added,
            program: name.to_string(),
            args: Vec::new(),
        },
    }
}

/// The probe's steps, from its handshake's outcome and the options it read.
fn probe(answer: &Answer, took: DurationMs) -> AgentProbe {
    let failed = |fallback: &str| ProbeStep::Failed {
        reason: bounded_text(
            answer
                .options
                .as_ref()
                .err()
                .map_or(fallback, |error| error.message.as_str()),
            MAX_TEXT,
        ),
    };
    let session = || match &answer.options {
        Ok(_) => ProbeStep::Ok,
        Err(error) => ProbeStep::Failed {
            reason: bounded_text(&error.message, MAX_TEXT),
        },
    };
    let (spawn, initialize, session, agent_version) = match &answer.said {
        Ok(introduction) => (
            ProbeStep::Ok,
            ProbeStep::Ok,
            session(),
            introduction.version.clone(),
        ),
        Err(HandshakeFailure::Spawn) => (
            failed("cannot start the agent"),
            ProbeStep::NotReached,
            ProbeStep::NotReached,
            None,
        ),
        Err(HandshakeFailure::Exited) => (
            ProbeStep::Ok,
            failed("the agent exited before it answered"),
            ProbeStep::NotReached,
            None,
        ),
        Err(HandshakeFailure::TimedOut { after }) => (
            ProbeStep::Ok,
            failed(&format!("no answer within {} ms", after.get())),
            ProbeStep::NotReached,
            None,
        ),
        Err(HandshakeFailure::Refused {
            refusal,
            introduction,
        }) => {
            let version = introduction.version.clone();
            match refusal {
                Refusal::SignIn => (
                    ProbeStep::Ok,
                    ProbeStep::Ok,
                    failed("it needs a sign-in first"),
                    version,
                ),
                Refusal::SessionNew => (
                    ProbeStep::Ok,
                    ProbeStep::Ok,
                    failed("it refused a new session"),
                    version,
                ),
                Refusal::Version { speaks } => (
                    ProbeStep::Ok,
                    failed(&format!("it speaks ACP {speaks}, Vessel speaks {SPEAKS}")),
                    ProbeStep::NotReached,
                    version,
                ),
                Refusal::Initialize | Refusal::TimedOut { .. } => (
                    ProbeStep::Ok,
                    failed("it did not complete initialize"),
                    ProbeStep::NotReached,
                    version,
                ),
            }
        }
    };
    AgentProbe {
        spawn,
        initialize,
        session,
        agent_version,
        took,
    }
}

fn status(options: &Result<HarnessOptions, WireError>, runs_here: bool) -> CatalogStatus {
    if !runs_here {
        return CatalogStatus::Unavailable;
    }
    match options {
        Ok(options) => CatalogStatus::Ready {
            models: models(options),
        },
        Err(error) => CatalogStatus::Error {
            message: bounded_text(&error.message, MAX_TEXT),
        },
    }
}

/// How many models the agent offers: its model option's values.
fn models(options: &HarnessOptions) -> u32 {
    let count = options
        .options
        .iter()
        .find(|option| option.category == OptionCategory::Model)
        .map_or(0, |option| {
            if let OptionKind::Select { values, .. } = &option.kind {
                values.len()
            } else {
                0
            }
        });
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// `path` with the home folder read as `~`.
pub(super) fn folded(path: &Path) -> PathBuf {
    dirs::home_dir()
        .and_then(|home| {
            path.strip_prefix(home)
                .ok()
                .map(|rest| Path::new("~").join(rest))
        })
        .unwrap_or_else(|| path.to_owned())
}

fn folded_installed(installed: Installed) -> Installed {
    match installed {
        Installed::Found {
            path,
            version,
            structured,
        } => Installed::Found {
            path: folded(&path),
            version,
            structured,
        },
        Installed::TooOld {
            path,
            version,
            speaks,
            needs,
        } => Installed::TooOld {
            path: folded(&path),
            version,
            speaks,
            needs,
        },
        Installed::TooNew {
            path,
            version,
            speaks,
            needs,
        } => Installed::TooNew {
            path: folded(&path),
            version,
            speaks,
            needs,
        },
        other @ (Installed::Missing { .. } | Installed::Unknown { .. }) => other,
    }
}
