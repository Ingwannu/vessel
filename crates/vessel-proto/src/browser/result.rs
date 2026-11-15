//! Results describe real engine effects; no variant manufactures a fixture page.

use serde::{Deserialize, Serialize};

use super::state::{BrowserRevision, BrowserTab, SnapshotStats};
use super::{BrowserId, CappedText, PageText, ScreenshotPng};

/// Screenshot/snapshot/evaluation data remain explicit content channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserResult {
    /// A workspace-filtered inventory from the owning node.
    Tabs {
        /// Never includes another workspace/session's private Preview target.
        tabs: super::BrowserTabs,
    },
    /// Creation/navigation/resize facts for the same real target.
    Tab {
        /// Current descriptor is derived from the engine, not an echoed request.
        tab: BrowserTab,
    },
    /// An input action was acknowledged at its actual execution boundary.
    Applied {
        /// The node verified ownership before sending input.
        browser_id: BrowserId,
    },
    /// A bounded accessibility snapshot with current refs.
    Snapshot {
        /// The refs belong to this target only.
        browser_id: BrowserId,
        /// A client cannot carry a ref across a later snapshot/navigation.
        revision: BrowserRevision,
        /// Display-only document facts captured by that same traversal.
        document: SnapshotDocument,
        /// Includes the truncation fact, not just a partial text string.
        content: CappedText,
        /// Measured traversal stats do not claim the entire DOM fit.
        stats: SnapshotStats,
    },
    /// A real painted PNG; absence/first-paint failures are errors instead.
    Screenshot {
        /// The target is retained even when its pane is hidden.
        browser_id: BrowserId,
        /// Image channel without a duplicated base64 metadata field.
        image: ScreenshotPng,
    },
    /// Page evaluation's JSON string is opaque to callers outside its decoding edge.
    Evaluation {
        /// The page/profile where the function actually executed.
        browser_id: BrowserId,
        /// Bounded serialized result, potentially containing signed-in data.
        json: CappedText,
    },
    /// Tail entries from bounded console and network rings.
    Logs {
        /// Only the caller's authorized target.
        browser_id: BrowserId,
        /// Typed console content, redacted from Debug through `PageText`.
        console: super::ConsoleEntries,
        /// Network metadata obtained from the same browser context.
        network: super::NetworkEntries,
    },
    /// A wait predicate matched; the command did not create a new navigation.
    Matched {
        /// The target whose text/location satisfied the predicate.
        browser_id: BrowserId,
    },
    /// Closing invalidated the target and its watches/refs.
    Closed {
        /// A later call cannot reuse this resource id.
        browser_id: BrowserId,
    },
}

/// Observed snapshot content, never navigation authority or a later tab lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SnapshotDocument {
    /// Preserve the exact observed address; do not fetch it or expose it in Debug.
    pub address: PageText,
    /// The title can be shortened only with the snapshot's truncation fact.
    pub title: PageText,
}

/// Console levels cannot become free-form tracing categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLevel {
    /// General console output.
    Log,
    /// Informational console output.
    Info,
    /// Warning console output.
    Warning,
    /// Page error console output.
    Error,
    /// Debug output still follows the same retention/content limits.
    Debug,
}

/// A console entry is page content, never an automatic host log record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConsoleEntry {
    /// Preserve the browser-provided severity as a typed value.
    pub level: ConsoleLevel,
    /// Never format this into diagnostics or persisted layout.
    pub text: PageText,
}

/// Network records do not carry cookies, request bodies or authorization headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NetworkEntry {
    /// URL itself can contain secrets; its Debug representation is redacted.
    pub url: super::BrowserUrl,
    /// HTTP method is decoded into this closed set at the CDP boundary.
    pub method: NetworkMethod,
    /// A response status when one exists; a failure is distinct from missing data.
    pub outcome: NetworkOutcome,
}

/// Common page request methods; extension methods remain explicit rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "UPPERCASE")]
pub enum NetworkMethod {
    /// Fetch a representation.
    Get,
    /// Inspect response metadata.
    Head,
    /// Submit page data.
    Post,
    /// Replace resource content.
    Put,
    /// Modify part of a resource.
    Patch,
    /// Remove a resource.
    Delete,
    /// Discover request options.
    Options,
    /// A method this client cannot classify remains a visible fact.
    #[serde(other)]
    Unknown,
}

/// A failed/pending request must not appear as status zero or an empty success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum NetworkOutcome {
    /// The request has not received a response.
    Pending,
    /// A checked HTTP response code.
    Response(HttpStatus),
    /// No arbitrary page/body detail is exposed as a host error string.
    Failed(super::BrowserFailure),
}

/// A response status is not an untyped byte count or a false success sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct HttpStatus(u16);
const MIN_HTTP_STATUS: u16 = 100;
const MAX_HTTP_STATUS: u16 = 599;

/// Refused status values are data errors, without any response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("HTTP status must be {MIN_HTTP_STATUS}..={MAX_HTTP_STATUS}")]
pub struct InvalidHttpStatus;
impl TryFrom<u16> for HttpStatus {
    type Error = InvalidHttpStatus;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if (MIN_HTTP_STATUS..=MAX_HTTP_STATUS).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidHttpStatus)
        }
    }
}
impl From<HttpStatus> for u16 {
    fn from(value: HttpStatus) -> Self {
        value.0
    }
}
impl HttpStatus {
    /// Explicit formatting edge for a checked HTTP code.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}
