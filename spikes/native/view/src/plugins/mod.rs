//! Plugin catalog ownership and captured-action rules, without GPUI or I/O.
//!
//! Parsed values belong to vessel-proto. This layer retains each node's last
//! safe snapshot and rejects stale replies/other-node actions. Appearance's
//! owner resolves theme preferences; this registry never selects another host.

mod registry;

pub use registry::{
    CatalogBudget, CatalogTicket, HostSupport, Installation, Problem, Registry, SnapshotApplied,
};
