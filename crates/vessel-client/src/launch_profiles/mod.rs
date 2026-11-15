//! Named launch-profile calls and pure transitions; no home or credential I/O.
//!
//! Eligibility is a node read. Materialization copies concrete choices into
//! ordinary creation or one live bundle; there is no persistent id binding.

mod calls;
mod materialize;

pub use calls::{APPLY_OPTIONS_TIMEOUT, LaunchProfileClientError, LaunchProfileSupport};
pub use materialize::{LivePreparation, PreparedLiveProfile, materialize_draft, prepare_live};

#[cfg(test)]
mod materialize_tests;
