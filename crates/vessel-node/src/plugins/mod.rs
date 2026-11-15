//! Node-owned plugin package admission and lifecycle policy, not helper mechanics.
//!
//! A14 staging: private manifest tests only. No install/configuration/RPC caller
//! can execute or activate a package through this module yet.
mod manifest;

mod source;

// Pure bounded digest facts only; no filesystem observation or authority.
mod artifact;
