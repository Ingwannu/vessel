//! The browser tool surface (ADR 0069): one call, `browser.call`, and the
//! types it carries. This crate does no I/O. A build with no engine mints
//! no tab id and paints nothing; the node answers `NoEngine`.
//!
//! [`BrowserUrl`] is not [`crate::HttpUrl`]. The fleet type refuses
//! userinfo. A page URL may have it, and `Debug` redacts it.

mod command;
mod failure;
mod id;
mod limits;
mod result;
mod text;
mod url;

pub use command::{
    BrowserCall, BrowserCommand, BrowserSpace, ClickButton, ClickCount, KeyModifier, ShotExtent,
    WaitExpect, wait_expect,
};
pub use failure::{BrowserFailure, HistoryWay, InvalidInput};
pub use id::{BrowserId, ElementRef};
pub use limits::{
    CssDelta, CssPoint, LogLimit, TextCap, ViewportSide, ViewportSize, WaitMs, DEFAULT_WAIT_MS,
    MAX_SCREENSHOT_BYTES, MAX_TEXT_BYTES, MAX_VIEWPORT_SIDE, MAX_WAIT_MS,
};
pub use result::{
    BrowserResult, BrowserTab, ConsoleLevel, ConsoleLine, HistoryMove, ImageMime, Matched,
    NetworkEntry, ScreenshotPng, SnapshotFormat, SnapshotStats, TabFocus, TabLoad,
};
pub use text::{BrowserText, EvaluateJson, RequiredText, SnapshotText};
pub use url::BrowserUrl;

use crate::{ErrorCode, ErrorDetail, WireError};

/// The MCP names, in the plan's order. The product build does not
/// advertise them.
pub const TOOL_NAMES: &[&str] = &[
    "browser_list_tabs",
    "browser_new_tab",
    "browser_close_tab",
    "browser_navigate",
    "browser_back",
    "browser_forward",
    "browser_reload",
    "browser_snapshot",
    "browser_screenshot",
    "browser_click",
    "browser_hover",
    "browser_drag",
    "browser_fill",
    "browser_select",
    "browser_type",
    "browser_keypress",
    "browser_upload",
    "browser_wait",
    "browser_scroll",
    "browser_resize",
    "browser_logs",
    "browser_evaluate",
];

/// A browser refusal as a wire error. The detail is the matchable reason.
#[must_use]
pub fn refusal(failure: BrowserFailure) -> WireError {
    WireError {
        code: failure.code(),
        message: failure.message().to_owned(),
        detail: Some(ErrorDetail::Browser { reason: failure }),
    }
}

/// A bad argument. The sentence names the kind and never quotes the text.
#[must_use]
pub fn invalid(reason: InvalidInput) -> WireError {
    WireError {
        code: ErrorCode::InvalidRequest,
        message: reason.message().to_owned(),
        detail: Some(ErrorDetail::Browser {
            reason: BrowserFailure::Invalid,
        }),
    }
}

#[cfg(test)]
mod tests;
