//! Names-only projection at the transport boundary.
//!
//! [Decision Log]
//! - Purpose: Sync can describe shared configuration without exposing values.
//! - Constraints: MCP literal text, command arguments and endpoint URLs may
//!   hold credentials. A raw Manifest must never enter a UI event or log.
//! - Alternatives: retain a manifest and trust every renderer to filter it.
//! - Choice: consume it once into names and file metadata; retain secret
//!   references by their already-parsed names, never their resolved values.
//! - Why: the screen's type cannot represent an MCP value or secret value.
//! - Cost: transport details belong to machine setup, not this names-only view.

use super::Problem;
use std::collections::BTreeSet;
use vessel_proto::{
    BundlePath, ConfigValue, FleetMember, FleetNode, Generation, Manifest, McpServerName,
    McpTransport, NodeId, SecretName,
};

/// A shared file's size, formatted by the existing workspace formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSize(u64);
impl FileSize {
    /// A human-readable byte size, using the page's binary units.
    #[must_use]
    pub fn word(self) -> String {
        crate::workspace::size::size_word(self.0)
    }
}
/// Whether a shared file can run as a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// A regular data file.
    Data,
    /// An executable skill script.
    Executable,
}
/// A file's identity and metadata, without its content or hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedFile {
    /// Its canonical bundle-relative path.
    pub path: BundlePath,
    /// Its size, in readable units.
    pub size: FileSize,
    /// Its executable flag, as reported by the home.
    pub mode: FileMode,
}
/// The only published configuration a Sync event may carry.
#[derive(Debug, Clone)]
pub struct Published {
    /// The version the home currently publishes.
    pub generation: Generation,
    /// All shared paths, in the node's stable sorted order.
    pub files: Vec<SharedFile>,
    /// Tool servers, names only.
    pub servers: Vec<McpServerName>,
    /// Secret references, each once, sorted, never their values.
    pub required: BTreeSet<SecretName>,
}
impl Published {
    /// Drop all value-bearing transport data before an event is constructed.
    #[must_use]
    pub fn project(manifest: Manifest) -> Self {
        let required = manifest
            .mcp
            .values()
            .flat_map(McpTransport::values)
            .filter_map(|value| match value {
                ConfigValue::Secret(name) => Some(name.clone()),
                ConfigValue::Text(_) => None,
            })
            .collect();
        Self {
            generation: manifest.generation,
            files: manifest
                .files
                .into_iter()
                .map(|(path, file)| SharedFile {
                    path,
                    size: FileSize(file.size),
                    mode: if file.executable {
                        FileMode::Executable
                    } else {
                        FileMode::Data
                    },
                })
                .collect(),
            servers: manifest.mcp.into_keys().collect(),
            required,
        }
    }
}
/// Each independent read stays available if another one is refused.
#[derive(Debug)]
pub struct Reads {
    /// Projected publication; raw configuration has already been dropped.
    pub published: Result<Published, Problem>,
    /// What the home last heard from members.
    pub members: Result<Vec<FleetMember>, Problem>,
    /// What the selected machine knows of fleet connections.
    pub nodes: Result<Vec<FleetNode>, Problem>,
    /// A successful home-only read proves the selected machine is a home.
    pub peers: Result<Vec<FleetNode>, Problem>,
    /// Names from secret.list, never secret values.
    pub secrets: Result<Vec<SecretName>, Problem>,
}
/// Home identity is shown only when a home-only RPC proves it.
#[derive(Debug)]
pub enum Home {
    /// The selected node answered a home-only read successfully.
    Here {
        /// Its node identity, not a device identity.
        node: NodeId,
        /// Its own reported name.
        name: String,
    },
    /// No role/home field exists for members; no ordering guess is made.
    Unreported(Problem),
}
/// A secret's presence on the selected machine, without a value slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Its name appears in secret.list.
    Present,
    /// Its name is referenced but absent from a successful secret.list.
    Missing,
    /// secret.list could not be read.
    Unknown,
}
/// Whether a published server references a secret name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// A shared server names it.
    Shared,
    /// Stored here but not named by the successfully read publication.
    LocalOnly,
    /// The publication could not be read, so usage cannot be inferred.
    Unreported,
}
/// A secret row whose type cannot carry a secret value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    /// A validated name.
    pub name: SecretName,
    /// Whether the selected machine has it.
    pub presence: Presence,
    /// Whether the publication names it.
    pub usage: Usage,
}
impl Presence {
    /// The shared badge meaning for presence, not for secret contents.
    #[must_use]
    pub const fn variant(self) -> crate::badge::Variant {
        match self {
            Self::Present => crate::badge::Variant::Success,
            Self::Missing => crate::badge::Variant::Warning,
            Self::Unknown => crate::badge::Variant::Muted,
        }
    }
}
/// A safe, partial snapshot: one failed read does not discard other facts.
#[derive(Debug)]
pub struct Snapshot {
    /// Proven home information or an explicit unknown.
    pub home: Home,
    /// Shared names and file metadata.
    pub published: Result<Published, Problem>,
    /// Each member's last report, as the node sent it.
    pub members: Result<Vec<FleetMember>, Problem>,
    /// Fleet connectivity, using peers as a home-side fallback for nodes.
    pub connections: Result<Vec<FleetNode>, Problem>,
    /// Stored names only.
    pub secrets: Result<Vec<SecretName>, Problem>,
}
impl Snapshot {
    /// Combine independent replies without guessing a member's home from list order.
    #[must_use]
    pub fn new(node: NodeId, name: String, reads: Reads) -> Self {
        let home = match &reads.peers {
            Ok(_) => Home::Here { node, name },
            Err(_) if reads.published.is_ok() => Home::Here { node, name },
            Err(why) => Home::Unreported(*why),
        };
        Self {
            home,
            published: reads.published,
            members: reads.members,
            connections: reads.nodes.or(reads.peers),
            secrets: reads.secrets,
        }
    }
    /// Sorted union of referenced and stored names; a failed read never means absent.
    #[must_use]
    pub fn secret_rows(&self) -> Vec<Secret> {
        let required = self
            .published
            .as_ref()
            .ok()
            .map(|published| &published.required);
        let stored = self.secrets.as_ref().ok();
        let mut names: BTreeSet<SecretName> = required
            .into_iter()
            .flat_map(|names| names.iter().cloned())
            .collect();
        names.extend(stored.into_iter().flatten().cloned());
        names
            .into_iter()
            .map(|name| {
                let usage = match required {
                    Some(names) if names.contains(&name) => Usage::Shared,
                    Some(_) => Usage::LocalOnly,
                    None => Usage::Unreported,
                };
                let presence = match stored {
                    Some(names) if names.contains(&name) => Presence::Present,
                    Some(_) => Presence::Missing,
                    None => Presence::Unknown,
                };
                Secret {
                    name,
                    presence,
                    usage,
                }
            })
            .collect()
    }
    /// A member's comparison is unknown unless the current publication was read.
    #[must_use]
    pub fn member_state(&self, member: &FleetMember) -> Option<super::MemberState> {
        self.published
            .as_ref()
            .ok()
            .map(|published| super::MemberState::of(member, published.generation))
    }
}

/// Empty-state wording depends on which sources were actually read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretEmpty {
    /// Both the publication and stored-name list were read and name none.
    NoneReported,
    /// Stored names are empty, but publication references could not be checked.
    StoredOnly,
}
impl Snapshot {
    /// Never label an unsuccessful name read as an empty list.
    #[must_use]
    pub fn secret_empty(&self) -> Option<SecretEmpty> {
        let Ok(stored) = &self.secrets else {
            return None;
        };
        if !stored.is_empty() {
            return None;
        }
        match &self.published {
            Ok(published) if published.required.is_empty() => Some(SecretEmpty::NoneReported),
            Ok(_) => None,
            Err(_) => Some(SecretEmpty::StoredOnly),
        }
    }
}
impl Home {
    /// A proved role, not a guess about sync or connection status.
    #[must_use]
    pub const fn variant(&self) -> crate::badge::Variant {
        match self {
            Self::Here { .. } => crate::badge::Variant::Success,
            Self::Unreported(_) => crate::badge::Variant::Muted,
        }
    }
}
