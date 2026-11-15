//! Saved-login report adapter: one opaque registry, existing probes/cache/policy only.
//! This adapter identifies configured bindings, not vendor account identities. Future
//! full source discovery plugs into the same report family rather than token accounting.
mod project;
mod registry;
pub(crate) use project::{entry, source};
pub(crate) use registry::{Binding, Registry};
