//! A project's scripts and services (protocol 11, ADR 0031): the named
//! commands `vessel.toml` gives, and running them.
//!
//! [Decision Log]
//! - Purpose: Paseo's `paseo.json` scripts (of type `service` or not),
//!   GOAT's `goat.json` scripts and agent-orchestrator's launch file let a
//!   person run a repository's commands from the page, and keep a dev
//!   server running.
//! - Constraint: a repository's command is someone else's code (ADR
//!   0027): it runs only under a person's approval of the exact set, the
//!   same digest covering setup, teardown and the scripts.
//! - Choice: two tables of `vessel.toml` (`[scripts.<name>]`,
//!   `[services.<name>]`), listed in `ProjectCommands`; a run is a terminal
//!   (`TerminalKind::Script` or `Service`), so its output is read and
//!   followed as any terminal's.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::id::{ProjectId, SessionId, TerminalId};
use crate::node::ProtocolVersion;
use crate::terminals::TerminalInfo;
use crate::workspace::{CreateWorktreeOutcome, ProjectCommands};

mod entry;
mod service_env;
pub use entry::{InvalidScriptEntry, ScriptEntry, ScriptEntryView};
pub use service_env::ServiceEnvName;

/// A script's or a service's name in `vessel.toml`: 1 to 64 bytes of ASCII
/// letters, digits, `-`, `_` and `.`, starting with a letter or a digit.
/// A person types it (`vessel script run dev`), so it is kept plain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ScriptName(Arc<str>);

/// A name refused as a [`ScriptName`] (its first 80 characters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidScriptName(pub String);

impl fmt::Display for InvalidScriptName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a script's name is 1 to 64 letters, digits, `-`, `_` or `.`, starting with a \
             letter or a digit, not {:?}",
            self.0
        )
    }
}

impl std::error::Error for InvalidScriptName {}

impl ScriptName {
    /// The longest name, in bytes.
    pub const MAX_BYTES: usize = 64;

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ScriptName {
    type Err = InvalidScriptName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let first = text.chars().next();
        let fits = first.is_some_and(|first| first.is_ascii_alphanumeric())
            && text.len() <= Self::MAX_BYTES
            && text.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            });
        if fits {
            Ok(Self(Arc::from(text)))
        } else {
            Err(InvalidScriptName(text.chars().take(80).collect()))
        }
    }
}

impl fmt::Display for ScriptName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(ScriptName);

/// Run once, or kept running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ScriptKind {
    /// `[scripts.<name>]`: runs to its end.
    Script,
    /// `[services.<name>]`: kept running, started again when it fails.
    Service,
}

impl ScriptKind {
    /// The table it is written in.
    #[must_use]
    pub const fn table(self) -> &'static str {
        match self {
            Self::Script => "scripts",
            Self::Service => "services",
        }
    }
}

/// Whose checkout a script runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScriptPlace {
    /// A session's: the worktree it runs in, or its project's folder.
    Session {
        /// Which.
        session: SessionId,
    },
    /// A project's folder.
    Project {
        /// Which.
        project: ProjectId,
    },
}

/// `script.list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScriptList {
    /// The folder `vessel.toml` was read in.
    pub root: PathBuf,
    /// What it names, as it is now, and whether that set is approved.
    pub commands: ProjectCommands,
    /// The runs in this root, live and kept.
    pub runs: Vec<TerminalInfo>,
}

/// What `script.run` and `script.restart` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ScriptRunOutcome {
    /// It runs, in this terminal.
    Started {
        /// The run.
        terminal: Box<TerminalInfo>,
    },
    /// The file's set is not approved for its project; nothing ran.
    NeedsApproval {
        /// The set to approve (`project.approve`).
        commands: ProjectCommands,
    },
    /// Nothing ran, for this reason.
    Refused {
        /// Why.
        reason: ScriptRefusal,
    },
}

/// Why a script did not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScriptRefusal {
    /// The session runs in no project, so it has no project file.
    NoProject,
    /// The file names no script or service of that name.
    NoSuchScript,
    /// Its `dir` is not a folder inside the root.
    NoSuchDir,
    /// The file does not parse; nothing in it runs.
    InvalidFile {
        /// Why.
        message: String,
    },
    /// A run of it in this root is live already.
    AlreadyRunning {
        /// That run.
        terminal: TerminalId,
    },
    /// A reason a newer node knows.
    #[serde(other)]
    Unknown,
}

/// `script.list`: a place's scripts, their approval and their runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListScripts {
    /// Whose checkout.
    pub place: ScriptPlace,
}

/// `script.run`: run a script or start a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunScript {
    /// Whose checkout.
    pub place: ScriptPlace,
    /// Which.
    pub script: ScriptName,
}

/// `script.restart`: run a script's run again (in place while it runs, in
/// a new terminal once it ended).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestartScript {
    /// The run.
    pub terminal: TerminalId,
}

/// The first protocol whose clients show a project file's scripts.
const SCRIPTS_SHOWN: ProtocolVersion = ProtocolVersion::new(11);

/// What a client that predates scripts is shown of a set that names some:
/// a set it cannot approve, since it could not show what it would approve
/// (ADR 0031).
const NOT_SHOWN: &str = "this project's file names scripts or services, which this client \
                         cannot show; approve it from a newer client";

impl ProjectCommands {
    /// This set as a client speaking `protocol` may see it, or `None` when
    /// it needs no change: a listed set with scripts is `invalid` to a
    /// client before protocol 11, so it never holds the digest of a set it
    /// could not display; the set last approved (protocol 11) is left out
    /// for such a client, which knows no such field.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.for_project_fields(protocol, crate::workspace::PROJECT_FILE_KEYS)
    }

    // Keep both projections complete: an optional approved field needs a decision.
    fn needs_project_fields(&self) -> bool {
        match self {
            Self::Listed {
                setup: _,
                teardown: _,
                scripts,
                terminals,
                service_ports,
                digest: _,
                approved: _,
                last_approved,
            } => {
                requires_project_fields(scripts, terminals, service_ports.as_ref())
                    || last_approved.as_ref().is_some_and(|last| {
                        let crate::workspace::CommandSet {
                            setup: _,
                            teardown: _,
                            scripts,
                            terminals,
                            service_ports,
                        } = &last.commands;
                        requires_project_fields(scripts, terminals, service_ports.as_ref())
                    })
            }
            Self::None | Self::Invalid { .. } => false,
        }
    }

    fn for_project_fields(
        &self,
        protocol: ProtocolVersion,
        since: Option<ProtocolVersion>,
    ) -> Option<Self> {
        if since.is_none_or(|since| protocol < since) && self.needs_project_fields() {
            return Some(Self::Invalid {
                message: "this client cannot show the project file's terminal commands or service port configuration; approve from a newer client".to_owned(),
            });
        }
        if protocol >= SCRIPTS_SHOWN {
            return None;
        }
        match self {
            Self::Listed { scripts, .. } if !scripts.is_empty() => Some(Self::Invalid {
                message: NOT_SHOWN.to_owned(),
            }),
            Self::Listed {
                setup,
                teardown,
                digest,
                approved,
                scripts,
                terminals,
                service_ports,
                last_approved: Some(_),
            } => Some(Self::Listed {
                setup: setup.clone(),
                teardown: teardown.clone(),
                digest: digest.clone(),
                approved: *approved,
                scripts: scripts.clone(),
                terminals: terminals.clone(),
                service_ports: service_ports.clone(),
                last_approved: None,
            }),
            Self::None | Self::Listed { .. } | Self::Invalid { .. } => None,
        }
    }
}

// Shared by current and previous approvals so neither can leak undisplayed data.
fn requires_project_fields(
    scripts: &[ScriptEntry],
    terminals: &[crate::workspace::FileTerminal],
    service_ports: Option<&crate::workspace::ServicePorts>,
) -> bool {
    service_ports.is_some()
        || !terminals.is_empty()
        || scripts.iter().any(|script| script.port().is_some())
}

impl ScriptRunOutcome {
    /// Prevent approval through a script reply when its fields cannot be shown.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::NeedsApproval { commands } => commands
                .for_protocol(protocol)
                .map(|commands| Self::NeedsApproval { commands }),
            Self::Started { .. } | Self::Refused { .. } => None,
        }
    }
}

impl CreateWorktreeOutcome {
    /// This outcome as a client speaking `protocol` may see it (its
    /// commands, by [`ProjectCommands::for_protocol`]).
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::NeedsApproval { commands } => commands
                .for_protocol(protocol)
                .map(|commands| Self::NeedsApproval { commands }),
            Self::Created { .. } | Self::Refused { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_plain_and_bounded() {
        for good in ["dev", "test-all", "db.migrate", "9lives", "a_b"] {
            assert!(good.parse::<ScriptName>().is_ok(), "{good}");
        }
        let long = "a".repeat(ScriptName::MAX_BYTES + 1);
        for bad in [
            "",
            "-x",
            ".hidden",
            "with space",
            "ünï",
            "a/b",
            "$(x)",
            long.as_str(),
        ] {
            assert!(bad.parse::<ScriptName>().is_err(), "{bad}");
        }
        assert!(
            "a".repeat(ScriptName::MAX_BYTES)
                .parse::<ScriptName>()
                .is_ok()
        );
    }

    #[test]
    fn a_bad_name_does_not_decode() {
        assert!(serde_json::from_str::<ScriptName>(r#""rm -rf""#).is_err());
        assert_eq!(
            serde_json::from_str::<ScriptName>(r#""dev""#)
                .unwrap()
                .as_str(),
            "dev"
        );
    }

    #[test]
    fn an_older_client_cannot_approve_scripts_it_cannot_show() {
        let digest: crate::workspace::CommandsDigest = "a".repeat(64).parse().unwrap();
        let listed = |scripts: Vec<ScriptEntry>| ProjectCommands::Listed {
            setup: vec!["npm ci".parse().unwrap()],
            teardown: Vec::new(),
            digest: digest.clone(),
            approved: false,
            scripts,
            terminals: Vec::new(),
            service_ports: None,
            last_approved: None,
        };
        let dev = ScriptEntry::new(
            "dev".parse().unwrap(),
            ScriptKind::Service,
            "npm run dev".parse().unwrap(),
            None,
            None,
        )
        .unwrap();
        let with = listed(vec![dev]);
        assert!(matches!(
            with.for_protocol(ProtocolVersion::new(10)),
            Some(ProjectCommands::Invalid { .. })
        ));
        assert_eq!(with.for_protocol(ProtocolVersion::new(11)), None);
        assert_eq!(
            listed(Vec::new()).for_protocol(ProtocolVersion::new(10)),
            None
        );
        // The set last approved reaches only a client that knows it.
        let mut changed = listed(Vec::new());
        if let ProjectCommands::Listed { last_approved, .. } = &mut changed {
            *last_approved = Some(Box::new(crate::workspace::ApprovedCommands {
                commands: crate::workspace::CommandSet::default(),
                digest: digest.clone(),
                approved_at: chrono::DateTime::UNIX_EPOCH,
            }));
        }
        assert_eq!(changed.for_protocol(ProtocolVersion::new(11)), None);
        assert_eq!(
            changed.for_protocol(ProtocolVersion::new(10)),
            Some(listed(Vec::new()))
        );
        let outcome = CreateWorktreeOutcome::NeedsApproval { commands: with };
        assert!(matches!(
            outcome.for_protocol(ProtocolVersion::new(10)),
            Some(CreateWorktreeOutcome::NeedsApproval {
                commands: ProjectCommands::Invalid { .. }
            })
        ));
    }

    #[test]
    fn terminal_approvals_are_gated_on_every_reply_and_keep_legacy_defaults() {
        use crate::workspace::{CommandSet, FileTerminal};
        let old = r#"{"setup":[],"teardown":[],"scripts":[]}"#;
        assert!(
            serde_json::from_str::<CommandSet>(old)
                .unwrap()
                .terminals
                .is_empty()
        );
        let legacy = format!(
            r#"{{"state":"listed","setup":[],"teardown":[],"scripts":[],"digest":"{}","approved":false}}"#,
            "a".repeat(64)
        );
        assert!(matches!(
            serde_json::from_str::<ProjectCommands>(&legacy).unwrap(),
            ProjectCommands::Listed { terminals, .. } if terminals.is_empty()
        ));
        let terminal = FileTerminal {
            name: Some("logs".parse().unwrap()),
            command: "tail -f dev.log".parse().unwrap(),
        };
        let commands = ProjectCommands::Listed {
            setup: Vec::new(),
            teardown: Vec::new(),
            scripts: Vec::new(),
            terminals: vec![terminal],
            service_ports: None,
            digest: "b".repeat(64).parse().unwrap(),
            approved: false,
            last_approved: None,
        };
        let encoded = serde_json::to_string(&commands).unwrap();
        assert_eq!(
            serde_json::from_str::<ProjectCommands>(&encoded).unwrap(),
            commands
        );
        // A merge-time assignment enables exactly its threshold; current builds
        // leave it absent, so no guessed protocol number admits these commands.
        assert_eq!(
            commands.for_project_fields(ProtocolVersion::new(31), Some(ProtocolVersion::new(31))),
            None
        );
        assert!(matches!(
            commands.for_project_fields(ProtocolVersion::new(30), Some(ProtocolVersion::new(31))),
            Some(ProjectCommands::Invalid { .. })
        ));
        for protocol in [ProtocolVersion::new(10), crate::PROTOCOL_VERSION] {
            for reply in port_approval_tests::approval_replies(&commands) {
                let adapted =
                    serde_json::to_string(&reply.for_protocol(protocol).unwrap()).unwrap();
                assert!(!adapted.contains("tail -f dev.log"));
                assert!(!adapted.contains(&"b".repeat(64)));
                assert!(adapted.contains("invalid"));
            }
        }
    }

    #[test]
    fn a_refusal_a_newer_node_knows_reads_as_unknown() {
        let refusal: ScriptRefusal = serde_json::from_str(r#"{"kind":"busy"}"#).unwrap();
        assert_eq!(refusal, ScriptRefusal::Unknown);
    }
}

#[cfg(test)]
mod port_approval_tests;

#[cfg(test)]
mod allocation_approval_tests;
