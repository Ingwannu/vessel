//! Staged spoken-conversation mechanisms (ADR 0080), not a product provider.
//!
//! A10: the outer test-only declaration is removed with node service wiring
//! after the shared method-table head is announced. Tests drive these real
//! boundaries with deterministic detector/provider inputs, never a microphone.

mod boundary;
mod detector_input;
mod finalize;
#[cfg(test)]
mod fixture;
mod http;
mod input_buffer;

mod prompt;
mod receipts;
mod registry;
mod segments;
mod spoken_input;
mod stt_endpoint;
mod stt_form;
mod synthesis;
mod transcript;
mod tts_endpoint;
mod updates;
