//! The browser tool surface (ADR 0069).
//!
//! The product engine in this slice is the absence: every command is
//! [`BrowserFailure::NoEngine`] and the tools are not advertised. The
//! in-memory double is compiled only for this crate's tests. No Chromium
//! is downloaded or started, and this crate does no I/O.
//!
//! [`BrowserFailure::NoEngine`]: vessel_proto::browser::BrowserFailure::NoEngine

mod deadline;
mod engine;
mod policy;
mod starts;

pub use deadline::{deadline, BROKER_DEADLINE};
pub use engine::Engine;
pub use policy::{offer, gate, Offer, Operate, Presence, Start};
pub use starts::Starts;

#[cfg(test)]
mod double;
