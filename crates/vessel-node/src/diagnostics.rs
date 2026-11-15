//! Staged diagnostic collection mechanisms, not a registered node service.
//!
//! The wiring commit after shared compatibility admission and method metadata
//! removes cfg(test). Until then tests drive the real bounded actors and
//! resource leases; no product API is widened for an absent wire caller.
mod admission;
#[cfg(test)]
mod deadline;
mod inventory;
mod journal;
mod process;
