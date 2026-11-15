//! Bounded node dictation: current settings, owned streams and endpoint transcription.
//!
//! All calls remain unavailable while the protocol number is unassigned.
//! The shared settings/secret owners and method table retain their authority.

// One-second input chunks define both accepted duration and segment headroom.
const MAX_CHUNK_SECONDS: usize = 1;

// Idle pooling must not retain more connections than the stream commit budget.
const MAX_IN_FLIGHT_COMMITS: usize = 2;

mod audio;
mod budget;
mod endpoint;
mod input;
mod jobs;
mod multipart;
mod provider_json;
mod roots;
mod text;
mod updates;

mod commands;
mod control;
mod stream;
mod stream_error;

mod prepare;
mod quota;
mod registry;
mod secret_read;

mod service;
mod wire;

/// Settings validation shares this service's existing bounded reader.
pub(crate) use secret_read::{ReadError, Reader};
/// Real connections share reservations independently of their lifetime guard.
pub(crate) use service::{Context, Resources, Service};

/// The assembled-node tests reuse the same real memory-RPC connection fixture.
#[cfg(test)]
pub(crate) use registry::test_fixture::Link as TestConnection;
