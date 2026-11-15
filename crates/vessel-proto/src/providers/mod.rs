//! Provider policy values (ADR 0081). A definition is not driver availability.
//! Requests remain unregistered until the shared method table admits them.

mod create;
mod text;

pub use create::{
    ApprovalFlow, AutoAccept, CreateConfig, CreateInput, CreateParent, CreatePolicy, Mode,
    ModeCatalogue, ModeRisk, ProviderKind,
};
pub use text::{InvalidProvider, ModeId, ProviderId};

use crate::ProtocolVersion;

/// Assigned only after provider catalogue, configuration and create consumers
/// are wired and reviewed; no current peer may call the unassigned methods.
pub const PROVIDERS: Option<ProtocolVersion> = None;

/// Whether a configured provider participates in discovery and creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enabled {
    /// Discovery and creation may be attempted, subject to driver support.
    On,
    /// Keep configuration but do not start a discovery process.
    Off,
}
