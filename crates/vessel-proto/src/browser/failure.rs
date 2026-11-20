//! Why a browser call was refused. The message is a fixed sentence: it
//! never repeats a cookie, a token, a page, or an evaluated result.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ErrorCode;

/// Which way `back` or `forward` had nothing to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HistoryWay {
    /// `back` on a tab that has not navigated.
    Back,
    /// `forward` when that stack is empty.
    Forward,
}

/// A browser refusal a client matches on, not on the sentence (ADR 0069).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserFailure {
    /// The setting is off, or the session started while it was off.
    Disabled,
    /// `list` or `new` with no workspace.
    NoWorkspace,
    /// A navigation that is not `http` or `https`.
    Scheme,
    /// `back` or `forward` when that stack is empty.
    NoHistory {
        /// Which stack was empty.
        way: HistoryWay,
    },
    /// The proxy refused the address. The engine slice answers this.
    Destination,
    /// The setting is on and this build has no real engine.
    NoEngine,
    /// No tab with that id in this workspace.
    UnknownTab,
    /// The tab was closed.
    Closed,
    /// The ref is not from the tab's latest snapshot.
    StaleRef,
    /// The deadline passed.
    TimedOut,
    /// This engine does not implement the command.
    Unsupported,
    /// The tab has not painted. A build with no engine does not say this.
    NoFrame,
    /// A bug. Not a bad argument.
    Internal,
    /// A bad id, URL, ref, size, wait or path.
    Invalid,
}

impl BrowserFailure {
    /// The existing code this refusal uses. No new code is added.
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::Disabled | Self::Unsupported => ErrorCode::Unsupported,
            Self::NoWorkspace | Self::Scheme | Self::NoHistory { .. } | Self::Destination => {
                ErrorCode::Forbidden
            }
            Self::NoEngine | Self::TimedOut | Self::NoFrame => ErrorCode::Unavailable,
            Self::UnknownTab => ErrorCode::NotFound,
            Self::Closed | Self::StaleRef => ErrorCode::Conflict,
            Self::Internal => ErrorCode::Internal,
            Self::Invalid => ErrorCode::InvalidRequest,
        }
    }

    /// The sentence a person reads. It does not contain the call's text.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Disabled => "browser tools are off",
            Self::NoWorkspace => "the session has no workspace",
            Self::Scheme => "Browser navigation only supports http and https URLs.",
            Self::NoHistory {
                way: HistoryWay::Back,
            } => "There is nothing to go back to.",
            Self::NoHistory {
                way: HistoryWay::Forward,
            } => "There is nothing to go forward to.",
            Self::Destination => "the proxy refused that address",
            Self::NoEngine => "this build has no browser engine",
            Self::UnknownTab => "no such browser tab in this workspace",
            Self::Closed => "that browser tab is closed",
            Self::StaleRef => "that element ref is not from the latest snapshot",
            Self::TimedOut => "the browser did not answer in time",
            Self::Unsupported => "this browser engine does not implement that command",
            Self::NoFrame => "the tab has not painted a frame",
            Self::Internal => "the browser hit a bug",
            Self::Invalid => "the browser arguments are not valid",
        }
    }
}

/// Which argument was rejected, so the sentence can name it without
/// quoting the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInput {
    /// Not a UUID version 4 and not a timestamp id.
    BrowserId,
    /// Not `@e` and a decimal.
    ElementRef,
    /// An empty URL.
    UrlEmpty,
    /// A URL containing whitespace.
    UrlSpace,
    /// A scheme other than `http` or `https`.
    UrlScheme,
    /// An `http` or `https` URL with no host.
    UrlHost,
    /// A viewport side outside 1..=8192.
    Viewport,
    /// `wait` was given both `text` and `url`, or neither.
    WaitBoth,
    /// `timeoutMs` was not a positive number of at most 30 seconds.
    WaitTimeout,
    /// `maxEntries` was not from 1 to 200.
    LogLimit,
    /// A scroll delta was not a finite number.
    Scroll,
    /// An upload path was empty.
    UploadEmpty,
    /// An upload path escaped the workspace.
    UploadOutside,
    /// No workspace root to resolve an upload against.
    NoRoot,
    /// A key of zero characters.
    KeyEmpty,
    /// A function of zero characters.
    FunctionEmpty,
    /// A screenshot above 8 MiB.
    ScreenshotSize,
    /// Snapshot or evaluate text above 80_000 bytes.
    TextSize,
    /// A required string was empty on the wire.
    EmptyText,
    /// A button, count, modifier or extent this build does not know.
    UnknownChoice,
}

impl InvalidInput {
    /// The sentence for this rejection. It does not quote the argument.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::BrowserId => "browser id is not a uuid version 4 or a timestamp id",
            Self::ElementRef => "element ref is not @e and a decimal",
            Self::UrlEmpty => "url is empty",
            Self::UrlSpace => "url contains whitespace",
            Self::UrlScheme => "url scheme is not http or https",
            Self::UrlHost => "url has no host",
            Self::Viewport => "viewport side must be from 1 to 8192",
            Self::WaitBoth => "wait needs exactly one of text or url",
            Self::WaitTimeout => "wait timeout must be from 1 to 30000 milliseconds",
            Self::LogLimit => "log limit must be from 1 to 200",
            Self::Scroll => "scroll delta is not a finite number",
            Self::UploadEmpty => "upload path is empty",
            Self::UploadOutside => "upload path is outside the workspace",
            Self::NoRoot => "upload has no workspace root",
            Self::KeyEmpty => "key is empty",
            Self::FunctionEmpty => "function is empty",
            Self::ScreenshotSize => "screenshot is above 8 MiB",
            Self::TextSize => "browser text is above 80000 bytes",
            Self::EmptyText => "browser text is empty",
            Self::UnknownChoice => "browser arguments use a choice this build does not know",
        }
    }
}

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
