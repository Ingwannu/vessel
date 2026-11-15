//! Trusted browser-debugger transport and its sealed typed protocol vocabulary.
//!
//! This module does not authorize a profile, launch a runtime or offer tools.
//! The future engine controller must obtain this endpoint from its verified
//! child, enforce workspace/service policy and map only browser-domain effects.
//! Never forward this endpoint, CDP target ids or raw events to an app/MCP client.

mod client;
mod endpoint;
mod identity;
pub(crate) mod requests;
#[cfg(test)]
mod tests;
mod wire;

pub(crate) use client::{CdpError, Client};
pub(crate) use endpoint::Endpoint;
pub(crate) use identity::{SessionId, TargetId};
