//! Provider policy, independent of transport availability (ADR 0081).
//! No process, credential lookup or network access occurs in this module.

mod create;
mod launch;
mod manifest;
mod models;
mod options;
mod registry;
mod selection;
mod transport;

pub use create::{CreateError, resolve_create};
pub use launch::{
    CommandPrefix, LaunchOrigin, PARENT_SESSION_VARIABLES, ProviderLaunch, ProviderLaunchError,
    ToolEnforcement, merge_launch, provider_launch, resolve_prefix,
};
pub use manifest::{Builtin, ManifestError, builtin_manifest, builtin_named};
pub use models::{MergedModels, merge_models, normalize};
pub use options::{CatalogueOptionsError, catalogue_from_options};
pub use registry::{
    ConfigurationRequirement, PluginDefinition, PluginRevision, ProviderDefinition,
    ProviderRegistry, ProviderSource, RegistryError,
};
pub use selection::{ModelSelection, SelectionError, select_model};
pub use transport::{ProviderProtocol, ProviderTransport, provider_transport};
