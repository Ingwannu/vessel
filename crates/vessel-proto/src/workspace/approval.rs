//! The set a person last approved for a project (protocol 11): what the
//! page shows beside a set that is not approved, so a person sees what
//! changed since they last approved (ADR 0027's approval, ADR 0031's
//! scripts).
//!
//! [Decision Log]
//! - Purpose: an approval covers a digest, and a set that changed has a
//!   new one; a person asked to approve it again should see what changed,
//!   not read the whole set as if new (the page track found the wire had
//!   no way to say it).
//! - Constraint: the node kept only digests, which say that a set changed
//!   and not how; the set a digest covers is known only where the node
//!   described it (`project.commands`, `script.list`, a `needs_approval`).
//! - Alternatives: the page keeping what it showed (lost with the tab, and
//!   another page or the CLI approved it); the node reading the file again
//!   at the approval (the set approved may come from a base commit, not
//!   the folder); a list of every approved set (the page needs one).
//! - Choice: the node keeps, with each approval from protocol 11 on, the
//!   set it had described under that digest (store schema 11), and a set
//!   that is not approved carries the project's newest approval whose set
//!   is known.
//! - Cost: an approval row holds its set (a file of at most 64 KiB parsed;
//!   most are a few hundred bytes); the node holds the four newest known
//!   sets per project in memory. An approval given before protocol 11 has
//!   no set, and is never shown as the last one.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::text::{CommandsDigest, ProjectCommand};
use super::{FileTerminal, ServicePorts};
use crate::scripts::ScriptEntry;

/// What a project's file names to run: setup and teardown, and its
/// scripts and services (protocol 11). Bounded by the file it was parsed
/// from: at most 64 KiB, 32 commands of each kind, 32 scripts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommandSet {
    /// Run when a worktree is created.
    pub setup: Vec<ProjectCommand>,
    /// Run before a worktree is removed.
    pub teardown: Vec<ProjectCommand>,
    /// Its scripts and services, in name order; left out when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<ScriptEntry>,
    /// Terminal commands covered by this approval (ADR 0066).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminals: Vec<FileTerminal>,
    /// Retain every allocation choice covered by the approval's digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_ports: Option<ServicePorts>,
}

/// A set a person approved for a project, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ApprovedCommands {
    /// What was approved.
    pub commands: CommandSet,
    /// Its digest, as `project.approve` was given it.
    pub digest: CommandsDigest,
    /// When (the newest approval of it).
    pub approved_at: DateTime<Utc>,
}
