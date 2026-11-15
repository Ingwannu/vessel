//! Catalog actor and committed observation over the canonical registry/Store.
//! Pending methods expose no capability before the integrator assigns a version.

mod backend;
mod errors;
pub(crate) use service::Labels;
mod journal;
#[cfg(test)]
mod real_tests;
mod service;
