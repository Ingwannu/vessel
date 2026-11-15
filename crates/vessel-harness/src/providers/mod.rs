//! Provider policy, independent of transport availability (ADR 0081).
//! No process, credential lookup or network access occurs in this module.

mod create;
mod manifest;

pub use create::{CreateError, resolve_create};
pub use manifest::{Builtin, ManifestError, builtin_manifest};
