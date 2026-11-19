//! The report's sections added by ADR 0111 (TRACKER 567), Paseo's daemon
//! report sections Vessel lacked: the machine (System), its disk, its
//! workspaces, what runs (Paseo's "Agents") and two tools' versions. Each
//! is numbers and closed states only, as ADR 0074's sections are; none
//! names a path, a host or a person's word.

use serde::{Deserialize, Serialize};

use super::{DiagnosticCount, DiagnosticSection, DiagnosticVersion};
use crate::HarnessKind;

/// The machine the node runs on (Paseo's "System").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSystem {
    /// Processors the node may use.
    pub cpus: DiagnosticCount,
    /// Load averages, where the system reports them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load: Option<DiagnosticLoad>,
    /// Memory, where the system reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<DiagnosticSystemMemory>,
}

/// Load averages over 1, 5 and 15 minutes, in hundredths (`0.42` is 42).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticLoad {
    /// Over one minute.
    pub one: u32,
    /// Over five minutes.
    pub five: u32,
    /// Over fifteen minutes.
    pub fifteen: u32,
}

/// The machine's memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSystemMemory {
    /// Bytes available to new work.
    pub available_bytes: DiagnosticCount,
    /// Bytes in all.
    pub total_bytes: DiagnosticCount,
}

/// The file system of the node's data directory (Paseo's "Disk"); never
/// its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticDisk {
    /// Bytes free for the node.
    pub available_bytes: DiagnosticCount,
    /// Bytes in all.
    pub total_bytes: DiagnosticCount,
}

/// The node's workspaces (Paseo's "Workspaces": projects, workspaces and
/// workspaces by kind), from its registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticWorkspaces {
    /// Projects with a workspace.
    pub projects: DiagnosticCount,
    /// Active workspaces.
    pub active: DiagnosticCount,
    /// Of those, worktrees the node made (the rest are a project's own
    /// folder).
    pub managed: DiagnosticCount,
    /// Archived workspaces, and ones being archived or restored.
    pub archived: DiagnosticCount,
}

/// What runs now (Paseo's "Agents": by provider, and pending permissions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticActivity {
    /// Live sessions of each built-in agent that has one, in Vessel's
    /// order.
    pub builtin: Vec<DiagnosticAgentCount>,
    /// Live sessions of added agents, together (their names are a
    /// person's words, ADR 0074).
    pub added: DiagnosticCount,
    /// Live sessions waiting on a person's answer to a permission.
    pub waiting: DiagnosticCount,
}

/// Live sessions of one built-in agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticAgentCount {
    /// Which agent.
    pub harness: HarnessKind,
    /// How many.
    pub live: DiagnosticCount,
}

/// Two tools the node's workspaces use (Paseo's "Tools").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticTools {
    /// `git`.
    pub git: DiagnosticTool,
    /// GitHub's `gh`.
    pub gh: DiagnosticTool,
}

/// One tool, as its `--version` answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DiagnosticTool {
    /// It answered; its version when it read as one.
    Found {
        /// The numbers of its version, when it gave three.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<DiagnosticVersion>,
    },
    /// Not on the node's search path.
    Missing,
    /// It did not answer in time.
    TimedOut,
    /// It failed.
    Failed,
}

/// The sections ADR 0111 added, together: sent only to a peer speaking
/// `DIAGNOSTICS_WIRE_PROTOCOL`, since an older one reads the report's
/// fields as closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticMore {
    /// The machine.
    pub system: DiagnosticSection<DiagnosticSystem>,
    /// The data directory's file system.
    pub disk: DiagnosticSection<DiagnosticDisk>,
    /// Projects and worktrees.
    pub workspaces: DiagnosticSection<DiagnosticWorkspaces>,
    /// What runs.
    pub activity: DiagnosticSection<DiagnosticActivity>,
    /// `git` and `gh`.
    pub tools: DiagnosticSection<DiagnosticTools>,
}
