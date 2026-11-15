//! Plugin catalog ownership and captured-action rules, without GPUI or I/O.
//!
//! Parsed values belong to vessel-proto. This layer retains each node's last
//! safe snapshot and rejects stale replies/other-node actions. Appearance's
//! owner resolves theme preferences; this registry never selects another host.

mod placement;
mod registry;
mod surface;

// Staged external-resource validation. Production registration follows the
// typed catalog-resource DTO and sole app Hub publication/language lifecycle.
#[cfg(test)]
mod translations;

pub use placement::ResolvedPluginPanel;
pub use surface::{
    FieldValue, MAX_SURFACE_EDIT_BYTES, Submission, SurfaceProblem, SurfaceRequest, SurfaceState,
};

pub use registry::{
    CatalogBudget, CatalogTicket, HostSupport, Installation, Problem, Registry, SnapshotApplied,
};
