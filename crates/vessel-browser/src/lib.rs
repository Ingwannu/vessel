//! Browser execution primitives, below node principal/workspace admission.
//!
//! No engine is qualified or launched by this staged library. Networking
//! validates the whole resolved answer and dials only a checked socket;
//! references cannot alias elements across snapshot/navigation replacement.
//! The node remains responsible for authenticated owner/service capabilities.

pub mod network;
mod references;

pub use references::{ReferenceError, References, SnapshotGeneration};
