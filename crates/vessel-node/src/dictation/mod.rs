//! Staged dictation mechanisms, compiled only for tests until RPC wiring.
//!
//! The wiring commit after shared protocol-accessor and call-table support
//! removes the outer test gate. These helpers do not start a live service;
//! tests exercise the real audio/identity code without widening its API.

mod audio;
mod multipart;
mod owner;
mod roots;
