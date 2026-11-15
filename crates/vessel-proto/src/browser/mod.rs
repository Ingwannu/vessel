//! Browser identities and commands, staged before the engine (ADR 0078).
//!
//! No method is registered and no product engine is represented by a fixture.
//! The protocol constant remains unassigned until the common method-table
//! integration. Page execution and network policy do not belong in this crate.

mod arguments;
mod bounds;
mod identity;
mod input;
mod tools;
mod upload;
mod url;

pub use arguments::{
    ClickArgs, ClickCount, CssDelta, DragArgs, ElementArgs, EvaluateArgs, KeyModifier,
    KeypressArgs, LogsArgs, MouseButton, NavigateArgs, NewTabArgs, NoBrowserArgs, ResizeArgs,
    ScreenshotArgs, ScreenshotExtent, ScrollArgs, TabArgs, TypeArgs, ValueArgs, WaitArgs,
};
pub use bounds::{
    DEFAULT_LOG_ENTRIES, DEFAULT_WAIT_MS, InvalidBrowserBound, LogCount, MAX_LOG_ENTRIES,
    MAX_VIEWPORT_SIDE, MAX_WAIT_MS, ViewportSide, ViewportSize, WaitTimeout,
};
pub use identity::{BrowserId, ElementRef, InvalidBrowserIdentity};
pub use input::{EmptyBrowserInput, InputText, RequiredInput, WaitFor};
pub use tools::{BROWSER_TOOLS, BrowserTool};
pub use upload::{InvalidUpload, UploadArgs, UploadFiles, UploadPath};
pub use url::{BrowserUrl, InvalidBrowserUrl};

use crate::ProtocolVersion;

/// A pending browser method cannot accidentally reach a current peer.
pub const BROWSER_PROTOCOL: Option<ProtocolVersion> = None;
