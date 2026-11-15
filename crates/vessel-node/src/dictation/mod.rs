//! Staged dictation mechanisms, compiled only for tests until RPC wiring.
//!
//! The wiring commit after shared protocol-accessor and call-table support
//! removes the outer test gate. These helpers do not start a live service;
//! tests exercise the real audio/identity code without widening its API.

// Idle pooling must not retain more connections than the stream commit budget.
const MAX_IN_FLIGHT_COMMITS: usize = 2;

mod audio;
mod endpoint;
mod multipart;
mod owner;
mod provider_json;
mod roots;
