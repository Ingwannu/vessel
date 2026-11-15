//! Provider policy, independent of transport availability (ADR 0081).
//! No process, credential lookup or network access occurs in this module.

mod create;
mod launch;
mod manifest;

pub use create::{CreateError, resolve_create};
pub use launch::{
    CommandPrefix, LaunchOrigin, PARENT_SESSION_VARIABLES, ProviderLaunch, ProviderLaunchError,
    ToolEnforcement, merge_launch, provider_launch, resolve_prefix,
};
pub use manifest::{Builtin, ManifestError, builtin_manifest};
