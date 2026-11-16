//! Node-owned plugin package admission and lifecycle policy, not helper mechanics.
//!
//! Private staging: manifest/source/artifact fixtures only. No install/configuration/RPC caller
//! can execute or activate a package through this module yet.
mod manifest;

mod source;

// Staged filesystem observations and tracked work; no installation authority.
mod artifact;
