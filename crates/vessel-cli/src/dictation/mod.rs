//! File dictation client, staged until the pending RPC methods are registered.
//!
//! The command wiring removes the outer test gate. This edge decodes the narrow
//! WAV subset before connecting; capture, codecs and recognition stay on the node.

mod file;
