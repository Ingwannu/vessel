//! Direct drivers (ADR 0057): the harnesses that can be driven over their
//! own protocol instead of ACP, and the versions each driver was checked
//! against.
//!
//! [Decision Log]
//! - Purpose: Codex, Claude Code and `OpenCode` get drivers that speak their
//!   own protocols, as Paseo does. ACP stays the default for each until its
//!   direct driver has run on real agents, and a node setting chooses.
//! - Choice: what a driver supports is data next to the harness's other
//!   facts, so the node's setting, the version check and the launch read
//!   one table. `selectable` stays false until the driver's permissions
//!   slice lands: a driver that cannot ask a person must never run a
//!   session (the plan, `docs/plans/agents-own-protocols.md`). Codex's
//!   landed with its slice 2 (approvals and questions, the mode).
//! - Versions: a driver states the oldest and the newest agent version it
//!   was checked against. Below the oldest, the session runs on ACP; above
//!   the newest, it runs and its log says so, since vendors add
//!   fields far more often than they remove them (but they do remove them:
//!   Codex 0.166.0 has no `thread/rollback`).
//! - Command: the agent's own program (the harness's binary) with the
//!   arguments that switch it to its protocol, then the profile's
//!   environment and route exactly as an ACP launch has them
//!   ([`launch_direct`](crate::launch_direct)). What a session adds
//!   (resume, MCP configuration, permission flags) is the driver's to add
//!   from `vessel-agent`'s `AgentSetup`, in the driver's `launch` step
//!   before the agent is spawned, so this table does not change per
//!   driver slice.

use vessel_proc::CommandSpec;
use vessel_proto::HarnessKind;

use crate::env::HarnessEnv;
use crate::error::HarnessError;
use crate::harnesses::descriptor;
use crate::launch::with_profile;
use crate::lookup::Lookup;
use crate::profile::Profile;

/// The protocol a direct driver speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectProtocol {
    /// Codex's app-server: JSON-RPC over stdio (`codex app-server`).
    CodexAppServer,
    /// Claude Code's stream-json over stdio (`claude -p --input-format
    /// stream-json --output-format stream-json`).
    ClaudeStream,
    /// `OpenCode`'s HTTP API and event stream (`opencode serve`).
    OpencodeHttp,
}

/// A harness's direct driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direct {
    /// What it speaks.
    pub protocol: DirectProtocol,
    /// The arguments that make the agent's own program speak it.
    pub args: &'static [&'static str],
    /// The oldest agent version it was checked against.
    pub oldest: Version,
    /// The newest agent version it was checked against.
    pub newest_checked: Version,
    /// Whether the node may select it: only once it asks a person for
    /// permissions as ACP did.
    pub selectable: bool,
}

/// The direct driver of `kind`, if it has one.
#[must_use]
pub const fn direct(kind: HarnessKind) -> Option<&'static Direct> {
    match kind {
        HarnessKind::Codex => Some(&CODEX),
        HarnessKind::ClaudeCode => Some(&CLAUDE),
        HarnessKind::OpenCode => Some(&OPENCODE),
        HarnessKind::Gemini => None,
    }
}

/// Checked against the app-server schema of Codex 0.166.0 (ADR 0057).
/// Selectable from the driver's slice 2: it asks a person what Codex
/// asks, with Codex's own choices.
const CODEX: Direct = Direct {
    protocol: DirectProtocol::CodexAppServer,
    // JSON-RPC over stdio is the app-server's default transport.
    args: &["app-server"],
    oldest: Version::new(0, 166, 0),
    newest_checked: Version::new(0, 166, 0),
    selectable: true,
};

/// The Agent SDK the driver follows (0.3.246) speaks to Claude Code 2.1.246.
const CLAUDE: Direct = Direct {
    protocol: DirectProtocol::ClaudeStream,
    args: &[
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ],
    oldest: Version::new(2, 1, 246),
    newest_checked: Version::new(2, 1, 246),
    selectable: false,
};

/// `OpenCode`'s server as documented for 1.18.
const OPENCODE: Direct = Direct {
    protocol: DirectProtocol::OpencodeHttp,
    // On loopback, on a port the system picks; the driver reads it from
    // the server's first line.
    args: &["serve", "--hostname", "127.0.0.1", "--port", "0"],
    oldest: Version::new(1, 18, 23),
    newest_checked: Version::new(1, 18, 23),
    selectable: false,
};

/// An agent's version: the first `major.minor.patch` of its `--version`
/// line; anything after the patch (a pre-release, a build) is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// `major.minor.patch`.
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The first `x.y.z` in `line` (`codex-cli 0.166.0`, `2.1.322 (Claude
    /// Code)`, `1.18.23`); `None` without one.
    #[must_use]
    pub fn find(line: &str) -> Option<Self> {
        line.split(|character: char| !(character.is_ascii_digit() || character == '.'))
            .find_map(|word| {
                let mut parts = word.split('.');
                let major = parts.next()?.parse().ok()?;
                let minor = parts.next()?.parse().ok()?;
                let patch = parts.next()?.parse().ok()?;
                Some(Self::new(major, minor, patch))
            })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// How an installed agent's version fits its direct driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionFit {
    /// No version could be read: the driver is not used.
    Unknown,
    /// Older than the driver was checked against: the driver is not used.
    Below {
        /// The oldest checked.
        oldest: Version,
    },
    /// Within what the driver was checked against.
    Checked,
    /// Newer than the newest checked: used, and said in the session's log.
    Newer {
        /// The newest checked.
        newest_checked: Version,
    },
}

impl Direct {
    /// How `version_line` (the agent's `--version`) fits this driver.
    #[must_use]
    pub fn fit(&self, version_line: Option<&str>) -> VersionFit {
        let Some(version) = version_line.and_then(Version::find) else {
            return VersionFit::Unknown;
        };
        if version < self.oldest {
            VersionFit::Below {
                oldest: self.oldest,
            }
        } else if version > self.newest_checked {
            VersionFit::Newer {
                newest_checked: self.newest_checked,
            }
        } else {
            VersionFit::Checked
        }
    }
}

/// The command that prints the installed agent's version (`<binary>
/// --version`), under `profile`'s environment as its sessions would run;
/// `None` for an agent with no direct driver (an added one included).
///
/// # Errors
/// [`HarnessError::NotInstalled`] when the agent's program is missing.
pub fn version_check(
    lookup: &Lookup,
    env: &HarnessEnv,
    profile: &Profile,
) -> Result<Option<CommandSpec>, HarnessError> {
    let Some(harness) = profile.known() else {
        return Ok(None);
    };
    if direct(harness).is_none() {
        return Ok(None);
    }
    let binary = descriptor(harness).binary;
    let program = lookup
        .find(binary)
        .ok_or(HarnessError::NotInstalled { harness, binary })?;
    Ok(Some(with_profile(
        env.apply(CommandSpec::new(program).arg("--version"), harness),
        profile,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_the_first_triple_of_the_line() {
        assert_eq!(
            Version::find("codex-cli 0.166.0"),
            Some(Version::new(0, 166, 0))
        );
        assert_eq!(
            Version::find("2.1.322 (Claude Code)"),
            Some(Version::new(2, 1, 322))
        );
        assert_eq!(
            Version::find("0.150.0-alpha.11"),
            Some(Version::new(0, 150, 0))
        );
        assert_eq!(Version::find("1.18"), None);
        assert_eq!(Version::find("version unknown"), None);
    }

    #[test]
    fn a_version_fits_below_within_or_above_what_was_checked() {
        let codex = direct(HarnessKind::Codex).unwrap();
        assert_eq!(
            codex.fit(Some("codex-cli 0.156.1")),
            VersionFit::Below {
                oldest: Version::new(0, 166, 0)
            }
        );
        assert_eq!(codex.fit(Some("codex-cli 0.166.0")), VersionFit::Checked);
        assert_eq!(
            codex.fit(Some("codex-cli 0.170.2")),
            VersionFit::Newer {
                newest_checked: Version::new(0, 166, 0)
            }
        );
        assert_eq!(codex.fit(None), VersionFit::Unknown);
        assert!(direct(HarnessKind::Gemini).is_none());
        assert!(codex.selectable, "Codex's permissions slice has landed");
        assert!(
            [HarnessKind::ClaudeCode, HarnessKind::OpenCode]
                .into_iter()
                .all(|kind| direct(kind).is_some_and(|driver| !driver.selectable)),
            "no direct driver is selectable before its permissions slice"
        );
    }
}
