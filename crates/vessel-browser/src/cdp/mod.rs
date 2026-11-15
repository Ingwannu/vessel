//! Trusted browser-debugger transport and its sealed typed protocol vocabulary.
//!
//! This module does not authorize a profile, launch a runtime or offer tools.
//! The future engine controller must obtain this endpoint from its verified
//! child, enforce workspace/service policy and map only browser-domain effects.
//! Never forward this endpoint, CDP target ids or raw events to an app/MCP client.

mod client;
mod endpoint;
mod identity;
pub mod requests;
#[cfg(test)]
mod tests;
mod wire;

pub use client::{Client, Error, Events};
pub use endpoint::{Endpoint, InvalidEndpoint};
pub use identity::{InvalidCdpIdentity, RemoteObjectId, SessionId, TargetId};
pub use wire::{DecodeError, Event, EventKind, RemoteError};
