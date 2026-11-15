//! Staged catalog observation (ADR 0086). Remove cfg(test) when the reviewed
//! registry storage and shared method table make the RPC wiring available.

mod errors;
mod journal;
mod service;
