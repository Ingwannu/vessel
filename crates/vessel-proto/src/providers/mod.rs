//! Provider policy values (ADR 0081). A definition is not driver availability.
//! Method-table rows remain pending until integrator protocol assignment.

mod catalogue;
mod config;
mod create;
mod launch;
mod output_schema;
mod snapshot;
mod text;

pub use catalogue::{
    Catalogue, CatalogueError, DefaultChoice, MAX_CATALOGUE_ENTRIES, MAX_CHOICE_ID, Model,
    ModelSpec, ModelVisibility, Models, ThinkingChoice, ThinkingChoices,
};
pub use config::{ConfigurationError, MAX_PROVIDERS, ProviderOverride, ProviderOverrides};
pub use create::{
    ApprovalFlow, AutoAccept, CreateConfig, CreateInput, CreateParent, CreatePolicy, Mode,
    ModeCatalogue, ModeRisk, ProviderKind,
};
pub use launch::{
    Arguments, CommandEdit, DisallowedTools, LaunchConfigError, LaunchSettings, MAX_LAUNCH_ITEMS,
    MAX_LAUNCH_TEXT, ProviderEnvironment, ToolPattern,
};
pub use output_schema::{MAX_OUTPUT_SCHEMA_BYTES, OutputSchema, OutputSchemaError};
pub use snapshot::{
    CatalogueDirectory, CatalogueScope, DiagnosticRead, DiscoveryFailure, DiscoveryPhase,
    GetProviderDiagnostic, GetProviders, InvalidCatalogueDirectory, ProviderDiagnostic,
    ProviderSnapshotEntry, ProviderState, ResolveProviderCreate, SetProviderEnabled,
    SnapshotSource, UnavailableReason,
};
pub use text::{InvalidProvider, MAX_MODE_ID, MAX_PROVIDER_ID, ModeId, ProviderId};

use crate::ProtocolVersion;

/// Assigned only after provider snapshot, diagnostic and participation consumers
/// are wired and reviewed; atomic creation has its own future contract.
pub const PROVIDERS: Option<ProtocolVersion> = None;

/// Whether a configured provider participates in discovery and creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "ProviderEnabled"))]
#[serde(rename_all = "snake_case")]
pub enum Enabled {
    /// Discovery and creation may be attempted, subject to driver support.
    On,
    /// Keep configuration but do not start a discovery process.
    Off,
}
