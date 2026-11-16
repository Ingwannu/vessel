//! Browser identities and commands, staged before the engine (ADR 0078).
//!
//! The four method rows remain pending; no product engine is represented by a fixture.
//! The protocol constant remains unassigned until the common method-table
//! integration. Page execution and network policy do not belong in this crate.

mod action_result;
#[cfg(test)]
mod admission_tests;
mod arguments;
mod bounds;
mod content;
mod failure;
mod human;
mod identity;
mod image;
mod input;
mod lists;
mod modifiers;
mod policy;
mod requests;
mod result;
mod state;
mod tools;
mod upload;
mod url;
mod watch;

pub use action_result::{ActionOutcome, WaitMatch};
pub use arguments::{
    ClickArgs, ClickCount, CssDelta, DragArgs, ElementArgs, EvaluateArgs, KeyModifier,
    KeypressArgs, LogsArgs, MouseButton, NavigateArgs, NewTabArgs, NoBrowserArgs, ResizeArgs,
    ScreenshotArgs, ScreenshotExtent, ScrollArgs, TabArgs, TypeArgs, ValueArgs, WaitArgs,
};
pub use bounds::{
    COMMAND_TIMEOUT, DEFAULT_LOG_ENTRIES, DEFAULT_WAIT_MS, InvalidBrowserBound, LogCount,
    MAX_LOG_ENTRIES, MAX_VIEWPORT_SIDE, MAX_WAIT_MS, ViewportSide, ViewportSize, WaitBudget,
    WaitTimeout,
};
pub use content::{
    CappedText, InvalidBrowserContent, MAX_PAGE_TEXT_BYTES, PageText, TextCompleteness,
};
pub use failure::{BrowserEffect, BrowserFailure, HistoryDirection};
pub use human::{
    CssCoordinate, CssPoint, HumanInput, InvalidCssCoordinate, KeyAction, PointerAction,
};
pub use identity::{BrowserId, ElementRef, InvalidBrowserIdentity};
pub use image::{
    ImagePixels, ImageSize, InvalidBrowserImage, MAX_RGBA_BYTES, MAX_SCREENSHOT_BYTES,
    ScreenshotPng,
};
pub use input::{InputText, InvalidBrowserInput, MAX_BROWSER_INPUT_BYTES, RequiredInput, WaitFor};
pub use lists::{
    BrowserTabs, ConsoleEntries, InvalidBrowserList, MAX_BROWSER_LOGS, MAX_BROWSER_TABS,
    NetworkEntries,
};
pub use modifiers::{KeyModifiers, ModifierState};
pub use policy::BrowserTools;
pub use requests::{
    BrowserCall, BrowserControl, BrowserControlCommand, BrowserWatched, ClearBrowserProfile,
    PreviewServiceHint, ServiceScheme, WatchBrowser,
};
pub use result::{
    BrowserResult, ConsoleEntry, ConsoleLevel, HttpStatus, InvalidHttpStatus, NetworkEntry,
    NetworkMethod, NetworkOutcome, SnapshotDocument,
};
pub use state::{
    BrowserHistory, BrowserIcon, BrowserLoad, BrowserProfile, BrowserRevision, BrowserTab,
    BrowserViewport, SnapshotMetric, SnapshotStats, TextByteCount,
};
pub use tools::{BROWSER_TOOLS, BrowserCommand, BrowserTool};
pub use upload::{InvalidUpload, MAX_UPLOAD_FILES, UploadArgs, UploadFiles, UploadPath};
pub use url::{BrowserUrl, InvalidBrowserUrl};

use crate::ProtocolVersion;

/// A pending browser method cannot accidentally reach a current peer.
pub const BROWSER_PROTOCOL: Option<ProtocolVersion> = None;

pub use watch::{BrowserChange, BrowserFrame, BrowserUpdate, BrowserUpdateSeq, BrowserWatchEnd};
