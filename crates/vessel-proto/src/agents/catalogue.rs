//! `agent.catalogue` (protocol 11, ADR 0032): the ACP registry, as the
//! node's last copy of it, each entry with what would run here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::text::{AgentProgram, RegistryId, RegistryVersion, Sha256Hex};
use crate::fleet::HttpUrl;
use crate::harness::HarnessKind;
use crate::name::EnvName;

/// `agent.catalogue`: the node's copy of the registry. Needs `read`;
/// `refresh`, which fetches it, is refused to an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetCatalogue {
    /// Fetch it first (unless the copy is younger than a minute).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub refresh: bool,
}

/// The node's copy of the registry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Catalogue {
    /// When it was fetched; absent before the first fetch since the node
    /// started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<DateTime<Utc>>,
    /// The registry's own version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<RegistryVersion>,
    /// Its entries Vessel reads, in the registry's order.
    pub entries: Vec<CatalogueEntry>,
    /// Entries left out: unreadable, past a bound, or pinning no exact
    /// version.
    pub skipped: u32,
}

/// One registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogueEntry {
    /// Its id.
    pub id: RegistryId,
    /// Its name.
    pub name: String,
    /// The version the registry pins.
    pub version: RegistryVersion,
    /// What it says of itself.
    pub description: String,
    /// Its site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<HttpUrl>,
    /// Its source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<HttpUrl>,
    /// Its licence, as the registry says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// What would run on this node.
    pub runs: CatalogueRun,
}

/// What a registry entry would run on this node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogueRun {
    /// One of the four, by its vendor's package: installed and used as
    /// that agent (`agent.install`), not added.
    Known {
        /// Which.
        harness: HarnessKind,
    },
    /// Started by a package runner (`npx`, `uvx`) that fetches the package
    /// at its first session, pinned to the exact version: exactly this
    /// would be added.
    Command {
        /// The runner.
        program: AgentProgram,
        /// Its arguments, the pinned package among them.
        args: Vec<String>,
        /// The variables the entry sets, as text.
        env: Vec<CatalogueVariable>,
        /// Whether the runner is on the node's search path.
        runner_found: bool,
    },
    /// Only an archive for this platform, which Vessel does not download:
    /// a person installs it, then adds it by command.
    Download {
        /// The archive.
        archive: HttpUrl,
        /// Its SHA-256, when the registry gives one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<Sha256Hex>,
    },
    /// Nothing for this platform.
    Unavailable,
}

/// A variable a registry entry sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CatalogueVariable {
    /// Its name.
    pub name: EnvName,
    /// Its value, as the registry gives it.
    pub value: String,
}
