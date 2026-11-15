//! Browser identities and commands, staged before the engine (ADR 0078).
//!
//! No method is registered and no product engine is represented by a fixture.
//! The protocol constant remains unassigned until the common method-table
//! integration. Page execution and network policy do not belong in this crate.

mod identity;
mod tools;

pub use identity::{BrowserId, ElementRef, InvalidBrowserIdentity};
pub use tools::{BROWSER_TOOLS, BrowserTool};

use crate::ProtocolVersion;

/// A pending browser method cannot accidentally reach a current peer.
pub const BROWSER_PROTOCOL: Option<ProtocolVersion> = None;
