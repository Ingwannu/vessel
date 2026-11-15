//! Staged dictation mechanisms, compiled only for tests until RPC wiring.
//!
//! The wiring commit after shared protocol-accessor and call-table support
//! removes the outer test gate. These helpers do not start a live service;
//! tests exercise the real audio/identity code without widening its API.

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
mod owner;
mod provider_json;
mod roots;
mod text;
mod updates;

mod commands;
mod control;
mod stream;
mod stream_error;

mod quota;
mod secret_read;
