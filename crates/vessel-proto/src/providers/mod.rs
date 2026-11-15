//! Provider policy values (ADR 0081). A definition is not driver availability.
//! Requests remain unregistered until the shared method table admits them.

mod catalogue;
mod config;
mod create;
mod launch;
mod snapshot;
mod text;

pub use catalogue::{
    Catalogue, CatalogueError, DefaultChoice, MAX_CATALOGUE_ENTRIES, MAX_CHOICE_ID, Model,
    ModelSpec, ModelVisibility, Models, ThinkingChoice,
};
pub use config::{ConfigurationError, MAX_PROVIDERS, ProviderOverride, ProviderOverrides};
pub use create::{
    ApprovalFlow, AutoAccept, CreateConfig, CreateInput, CreateParent, CreatePolicy, Mode,
    ModeCatalogue, ModeRisk, ProviderKind,
};
pub use launch::{
    Arguments, CommandEdit, DisallowedTools, LaunchConfigError, LaunchSettings, MAX_LAUNCH_ITEMS,
    MAX_LAUNCH_TEXT, ProviderEnvironment,
};
pub use snapshot::{
    CatalogueDirectory, CatalogueScope, DiagnosticRead, DiscoveryFailure, DiscoveryPhase,
    GetProviderDiagnostic, GetProviders, InvalidCatalogueDirectory, ProviderDiagnostic,
    ProviderSnapshotEntry, ProviderState, ResolveProviderCreate, SnapshotSource, UnavailableReason,
};
pub use text::{InvalidProvider, MAX_MODE_ID, MAX_PROVIDER_ID, ModeId, ProviderId};

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
