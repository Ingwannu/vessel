//! Which protocol drives a new structured session (protocol 18, ADR 0057):
//! the node's setting for its agent, the drivers this build has, and the
//! installed agent's version.
//!
//! [Decision Log]
//! - Purpose: Codex, Claude Code and `OpenCode` get drivers over their own
//!   protocols. A person turns one on per agent (`node.set_settings`); the
//!   runtime decides at each start whether it can be used, and the
//!   session's log says why when it is not.
//! - Constraint: a driver runs only once it can ask a person for
//!   permissions (`Direct::selectable`, and `vessel_agent::Driver::built`
//!   for the code), and only on an agent version it was checked against:
//!   a vendor renames or removes fields between releases.
//! - Choice: decided when a session's agent is first launched (a creation,
//!   a start again, a probe, a listing) and kept for the session's life: a
//!   move, a park and wake and a rewind restart it with the same driver,
//!   since its options (ids and values) are the driver's. A setting
//!   changed later applies to sessions started after.
//! - Version: the agent's own `--version`, run under the session's profile
//!   with a short bound, and kept per program until the file changes, so
//!   a start does not run it each time. Unreadable or older than the
//!   driver's oldest: ACP, with a line in the log. Newer than the newest
//!   checked: the driver, with a line in the log (vendors add fields far
//!   more often than they remove them).
//! - Alternatives: refusing the session when the driver cannot run
//!   (ACP works, and the setting is a preference, not a requirement);
//!   checking the version once at node start (an upgrade while the node
//!   runs would not be seen).

use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Arc;
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use vessel_agent::Driver;
use vessel_harness::{Direct, DirectProtocol, Profile, VersionFit};
use vessel_proto::settings::{AgentDriver, AgentDrivers};
use vessel_proto::{HarnessKind, HarnessRef, SessionMode, SessionSpec};

use super::SessionRuntime;
use super::preparation::PreparationError;
use super::preparation_gate::{PreparationAdmission, PreparationCancellation, PreparationEffect};
use super::prepared::RetainedPreparation;
use crate::SessionError;

pub(super) mod version;
#[cfg(test)]
use version::read_version;

/// The node's setting and what is known of the installed agents. Each
/// lock is held for one read or one assignment, never across `.await`, so
/// a poisoned one is whole.
#[derive(Debug, Default)]
pub(crate) struct Drivers {
    /// As `node.settings` says now.
    setting: Mutex<AgentDrivers>,
    /// Each program's version line, by program.
    versions: Mutex<HashMap<PathBuf, VersionRead>>,
}

/// A program's `--version` line as read, and the file's modification time
/// then.
#[derive(Debug, Clone)]
struct VersionRead {
    modified: Option<SystemTime>,
    line: Option<String>,
}

/// What drives a session, and what its log says about the choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Chosen {
    /// The agent's own protocol; `None` for ACP.
    pub(crate) direct: Option<&'static Direct>,
    /// Lines for the session's log.
    pub(crate) notices: Vec<String>,
}

impl Chosen {
    const fn acp() -> Self {
        Self {
            direct: None,
            notices: Vec::new(),
        }
    }

    fn acp_because(line: String) -> Self {
        Self {
            direct: None,
            notices: vec![line],
        }
    }

    /// The driver that runs it.
    pub(crate) fn driver(&self) -> Driver {
        self.direct
            .map_or(Driver::Acp, |direct| driver_of(direct.protocol))
    }
}

/// The agent-side driver of `protocol`.
pub(crate) const fn driver_of(protocol: DirectProtocol) -> Driver {
    match protocol {
        DirectProtocol::CodexAppServer => Driver::CodexAppServer,
        DirectProtocol::ClaudeStream => Driver::ClaudeStream,
        DirectProtocol::OpencodeHttp => Driver::OpencodeHttp,
    }
}

impl SessionRuntime {
    /// Drive new sessions of each agent as `drivers` says (protocol 18). A
    /// session already running keeps its driver.
    pub fn set_drivers(&self, drivers: AgentDrivers) {
        *self
            .drivers
            .setting
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = drivers;
    }

    /// What drives a session of `spec` under `profile` started now.
    pub(crate) async fn choose_driver<T, F>(
        &self,
        spec: &SessionSpec,
        profile: &Profile,
        preparation: RetainedPreparation<T, F>,
    ) -> Result<Chosen, SessionError>
    where
        T: Send + Sync + 'static,
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        let setting = *self
            .drivers
            .setting
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let direct = match gate(spec, setting) {
            Gate::Acp => return Ok(Chosen::acp()),
            Gate::Refused(line) => return Ok(Chosen::acp_because(line)),
            Gate::Direct(direct) => direct,
        };
        let line = self.version_line(profile, preparation).await?;
        Ok(fit(spec_kind(spec), direct, line.as_deref()))
    }

    /// The installed agent's `--version` line under `profile`, kept until
    /// its program changes; `None` when it could not be read.
    async fn version_line<T, F>(
        &self,
        profile: &Profile,
        preparation: RetainedPreparation<T, F>,
    ) -> Result<Option<String>, SessionError>
    where
        T: Send + Sync + 'static,
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        let owned = profile.clone();
        let prepared = self
            .prepare_blocking(preparation.clone(), move || {
                let command = vessel_harness::version_check(&lookup, &env, &owned)
                    .map_err(SessionError::Launch)?;
                Ok(command.map(|command| {
                    let modified = std::fs::metadata(&command.program)
                        .and_then(|metadata| metadata.modified())
                        .ok();
                    (command, modified)
                }))
            })
            .await;
        let command = match prepared {
            Ok(command) => command,
            Err(PreparationError::Refused(reason)) => return Err(reason.into()),
            // Preserve the legacy unreadable-version fallback, never admission refusal.
            Err(PreparationError::Failed(_error)) => return Ok(None),
        };
        let Some((command, modified)) = command else {
            return Ok(None);
        };
        let program = command.program.clone();
        if let Some(kept) = self.drivers.kept(&program, modified) {
            return Ok(kept.line);
        }
        let line = self.version_owned(command, preparation).await?;
        self.drivers.remember(program, modified, line.clone());
        Ok(line)
    }

    async fn version_owned<T, F>(
        &self,
        command: vessel_proc::CommandSpec,
        preparation: RetainedPreparation<T, F>,
    ) -> Result<Option<String>, SessionError>
    where
        T: Send + Sync + 'static,
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        // These original owners and tracker exist before the logical commit.
        let tracked = self.tasks_token();
        let held = preparation;
        Self::admit_preparation(&held, PreparationEffect::Version).await?;
        // Kernel spawn follows the host's logical commit in this same poll.
        // The existing token and original context were retained before admission.
        let started = tokio::time::Instant::now();
        let Ok(spawned) = vessel_proc::spawn(&command) else {
            return Ok(None);
        };
        #[cfg(test)]
        let control = {
            let control = Arc::clone(&self.version_control);
            // The witness belongs to synchronous successful kernel spawn,
            // not to the later scheduling of the owned observer task.
            control.mark_started();
            control
        };
        let running = self.tasks.spawn(async move {
            let _tracked = tracked;
            let _held = held;
            #[cfg(not(test))]
            let stdin_owner = drop;
            #[cfg(test)]
            let stdin_owner = move |stdin| control.publish(stdin);
            version::observe_version(spawned, started, stdin_owner).await
        });
        running
            .await
            .map_err(SessionError::Internal)?
            .map_err(SessionError::VersionCleanup)
    }
}

impl Drivers {
    fn kept(&self, program: &std::path::Path, modified: Option<SystemTime>) -> Option<VersionRead> {
        let kept = self
            .versions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(program)
            .cloned()?;
        if modified.is_some() && kept.modified == modified {
            Some(kept)
        } else {
            None
        }
    }
    fn remember(&self, program: PathBuf, modified: Option<SystemTime>, line: Option<String>) {
        let _previous = self
            .versions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(program, VersionRead { modified, line });
    }
}

/// What the setting and this build allow, before the version is read.
#[derive(Debug, PartialEq, Eq)]
enum Gate {
    Acp,
    Refused(String),
    Direct(&'static Direct),
}

/// The agent `spec` runs, when it is one of the four.
const fn spec_kind(spec: &SessionSpec) -> Option<HarnessKind> {
    match &spec.harness {
        HarnessRef::Known(kind) => Some(*kind),
        HarnessRef::Added(_) => None,
    }
}

/// `spec`'s driver as far as `setting` and this build decide it.
fn gate(spec: &SessionSpec, setting: AgentDrivers) -> Gate {
    let (Some(kind), SessionMode::Structured { .. }) = (spec_kind(spec), &spec.mode) else {
        return Gate::Acp;
    };
    // `unknown` (a newer node's choice, read after a downgrade) runs as
    // ACP, as the setting says.
    if setting.get(kind) != Some(AgentDriver::Direct) {
        return Gate::Acp;
    }
    match vessel_harness::direct(kind) {
        Some(direct) if direct.selectable && driver_of(direct.protocol).built() => {
            Gate::Direct(direct)
        }
        Some(_) | None => Gate::Refused(format!(
            "{kind} is set to run over its own protocol, which this build of Vessel does not \
             offer yet; it runs over ACP"
        )),
    }
}

/// `direct` for an agent whose `--version` printed `line`.
fn fit(kind: Option<HarnessKind>, direct: &'static Direct, line: Option<&str>) -> Chosen {
    let name = kind.map_or_else(|| "the agent".to_owned(), |kind| kind.to_string());
    let found = line.and_then(vessel_harness::Version::find);
    match (direct.fit(line), found) {
        (VersionFit::Checked, _) => Chosen {
            direct: Some(direct),
            notices: Vec::new(),
        },
        (VersionFit::Newer { newest_checked }, Some(version)) => Chosen {
            direct: Some(direct),
            notices: vec![format!(
                "{name} {version} is newer than {newest_checked}, the newest its own protocol \
                 was checked against; something it changed may be missed"
            )],
        },
        (VersionFit::Below { oldest }, Some(version)) => Chosen::acp_because(format!(
            "{name} {version} is older than {oldest}, the oldest its own protocol was checked \
             against; it runs over ACP"
        )),
        (VersionFit::Unknown | VersionFit::Newer { .. } | VersionFit::Below { .. }, _) => {
            Chosen::acp_because(format!(
                "{name}'s version could not be read, so it runs over ACP rather than its own \
                 protocol"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_harness::Version;
    use vessel_proto::{AgentName, TerminalSize};

    use super::*;

    fn structured(kind: HarnessKind) -> SessionSpec {
        SessionSpec::new(
            kind,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
    }

    /// A driver this build does not offer yet, set anyway (a file from a
    /// newer build): ACP, with the reason in the log. One it offers
    /// (Codex's, from its slice 2): that driver, the version to decide.
    /// ACP, `unknown`, a terminal session and an added agent: ACP, nothing
    /// said.
    #[test]
    fn the_setting_and_the_build_decide_before_the_version() {
        let direct = AgentDrivers {
            codex: AgentDriver::Direct,
            open_code: AgentDriver::Direct,
            claude_code: AgentDriver::Unknown,
        };
        let Gate::Refused(line) = gate(&structured(HarnessKind::OpenCode), direct) else {
            panic!("OpenCode's own protocol is not offered yet");
        };
        assert!(line.contains("runs over ACP"), "{line}");
        assert!(matches!(
            gate(&structured(HarnessKind::Codex), direct),
            Gate::Direct(chosen) if chosen.protocol == DirectProtocol::CodexAppServer
        ));
        assert_eq!(
            gate(&structured(HarnessKind::ClaudeCode), direct),
            Gate::Acp
        );
        assert_eq!(gate(&structured(HarnessKind::Gemini), direct), Gate::Acp);
        let terminal = SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        );
        assert_eq!(gate(&terminal, direct), Gate::Acp);
        let added = SessionSpec::new(
            HarnessRef::Added("mine".parse::<AgentName>().unwrap()),
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        assert_eq!(gate(&added, direct), Gate::Acp);
    }

    /// Once offered, the version decides: within what was checked, the
    /// driver; newer, the driver and a line; older or unreadable, ACP and a
    /// line.
    #[test]
    fn the_version_decides_once_a_driver_is_offered() {
        static CHECKED: Direct = Direct {
            protocol: DirectProtocol::CodexAppServer,
            args: &["app-server"],
            oldest: Version::new(0, 166, 0),
            newest_checked: Version::new(0, 170, 0),
            selectable: true,
        };
        let codex = Some(HarnessKind::Codex);
        let at = |line| fit(codex, &CHECKED, line);
        assert_eq!(
            at(Some("codex-cli 0.168.2")),
            Chosen {
                direct: Some(&CHECKED),
                notices: Vec::new()
            }
        );
        let newer = at(Some("codex-cli 0.171.0"));
        assert_eq!(newer.driver(), Driver::CodexAppServer);
        assert!(newer.notices[0].contains("0.171.0 is newer than 0.170.0"));
        let older = at(Some("codex-cli 0.156.1"));
        assert_eq!(older.driver(), Driver::Acp);
        assert!(older.notices[0].contains("0.156.1 is older than 0.166.0"));
        let unread = at(None);
        assert_eq!(unread.driver(), Driver::Acp);
        assert!(unread.notices[0].contains("could not be read"));
    }

    /// The version is the program's first line; a program that fails says
    /// nothing usable.
    #[tokio::test]
    async fn a_version_is_the_programs_first_line_when_it_succeeds() {
        let fake = vessel_testkit::Fake::CodexAppServer.path().unwrap();
        let version = vessel_proc::CommandSpec::new(fake.clone()).arg("--version");
        assert_eq!(
            read_version(version).await.unwrap().as_deref(),
            Some("codex-cli 0.166.0")
        );
        let refused = vessel_proc::CommandSpec::new(fake).arg("--no-such-flag");
        assert_eq!(read_version(refused).await.unwrap(), None);
    }

    /// Every driver the harness table offers is one this build runs: the
    /// two flags move together.
    #[test]
    fn what_is_offered_is_built() {
        for kind in HarnessKind::ALL {
            if let Some(direct) = vessel_harness::direct(kind) {
                assert!(
                    !direct.selectable || driver_of(direct.protocol).built(),
                    "{kind}: offered but not built"
                );
            }
        }
    }
}
