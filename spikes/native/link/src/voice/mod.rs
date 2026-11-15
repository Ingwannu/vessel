//! Staged voice adapter/controller (ADR 0080); no RPC or audio device is advertised.
//!
//! The outer test-only declaration is removed after the actual node adapter
//! and shared audio engine exist. This generic boundary is the explicitly
//! staged node/app seam; no invented wire call is serialized here.

mod upload;
