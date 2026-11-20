//! What a browser command answers. A screenshot is capped at 8 MiB.
//! Evaluate's JSON is a string and is not parsed.

use std::fmt;

use bytes::Bytes;
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::command::BrowserSpace;
use super::id::{BrowserId, ElementRef};
use super::limits::{CssPoint, TextCap, ViewportSide, MAX_SCREENSHOT_BYTES};
use super::text::{BrowserText, EvaluateJson, RequiredText, SnapshotText};
use super::url::BrowserUrl;
use super::InvalidInput;

/// A tab in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserTab {
    /// The tab.
    pub browser_id: BrowserId,
    /// The workspace it belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<BrowserSpace>,
    /// The current URL.
    pub url: BrowserUrl,
    /// The title.
    pub title: BrowserText,
    /// Whether it is the active tab.
    pub focus: TabFocus,
    /// Whether it is still loading.
    pub load: TabLoad,
    /// Whether `back` has a page.
    pub back: HistoryMove,
    /// Whether `forward` has a page.
    pub forward: HistoryMove,
}

/// Counts a snapshot reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SnapshotStats {
    /// Nodes in the snapshot.
    pub node_count: u32,
    /// Element refs in the snapshot.
    pub ref_count: u32,
    /// Bytes of snapshot text.
    pub text_length: u32,
    /// Iframes, when the engine counted them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iframe_count: Option<u32>,
    /// The deepest node, when the engine counted it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// One console line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConsoleLine {
    /// The level.
    pub level: ConsoleLevel,
    /// The message.
    pub message: BrowserText,
}

/// A console level. Unknown is a level a newer engine names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLevel {
    /// A debug line.
    Debug,
    /// An info line.
    Info,
    /// A warning.
    Warn,
    /// An error.
    Error,
    /// A level this build does not name.
    #[serde(other)]
    Unknown,
}

/// One network entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NetworkEntry {
    /// The request URL.
    pub url: BrowserUrl,
    /// The method, when the engine reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<BrowserText>,
    /// The status, when the response has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

/// PNG bytes, at most 8 MiB decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ScreenshotPng(Bytes);

impl ScreenshotPng {
    /// `bytes` when they fit the cap.
    ///
    /// # Errors
    /// [`InvalidInput::ScreenshotSize`].
    pub fn new(bytes: Bytes) -> Result<Self, InvalidInput> {
        if bytes.len() > MAX_SCREENSHOT_BYTES {
            Err(InvalidInput::ScreenshotSize)
        } else {
            Ok(Self(bytes))
        }
    }

    /// The decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Standard base64, for an MCP image block.
    #[must_use]
    pub fn base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.0)
    }
}

impl Serialize for ScreenshotPng {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::base64_bytes::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for ScreenshotPng {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = crate::base64_bytes::deserialize(deserializer)?;
        Self::new(bytes).map_err(de::Error::custom)
    }
}

/// What `browser.call` answers. Match it without a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum BrowserResult {
    /// The open tabs.
    Tabs {
        /// The tabs.
        tabs: Vec<BrowserTab>,
    },
    /// A tab that was opened.
    Opened {
        /// The tab.
        browser_id: BrowserId,
        /// Its URL.
        url: BrowserUrl,
    },
    /// A tab that was closed.
    Closed {
        /// The tab.
        browser_id: BrowserId,
    },
    /// The tab's URL after a navigation, back, forward or reload.
    At {
        /// The tab.
        browser_id: BrowserId,
        /// The URL now.
        url: BrowserUrl,
    },
    /// A snapshot.
    Snapshot {
        /// The tab.
        browser_id: BrowserId,
        /// The URL.
        url: BrowserUrl,
        /// The title.
        title: BrowserText,
        /// Always `aria-yaml` for this slice's double.
        format: SnapshotFormat,
        /// The text, capped.
        snapshot: SnapshotText,
        /// Whether the cap bound.
        truncated: TextCap,
        /// The counts.
        stats: SnapshotStats,
    },
    /// A PNG.
    Screenshot {
        /// `image/png`.
        mime: ImageMime,
        /// The bytes.
        #[serde(rename = "data_base64")]
        data: ScreenshotPng,
        /// The width.
        width: ViewportSide,
        /// The height.
        height: ViewportSide,
    },
    /// A click.
    Clicked {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// Where, when the engine reports it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<CssPoint>,
    },
    /// A hover.
    Hovered {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// Where, when the engine reports it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<CssPoint>,
    },
    /// A drag.
    Dragged {
        /// The tab.
        browser_id: BrowserId,
        /// The source.
        source: ElementRef,
        /// The target.
        target: ElementRef,
    },
    /// A fill.
    Filled {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
    },
    /// A select.
    Selected {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// The option.
        value: BrowserText,
    },
    /// Typed text.
    Typed {
        /// The tab.
        browser_id: BrowserId,
        /// The element, when one was named.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
        /// Where, when the engine reports it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<CssPoint>,
    },
    /// A key.
    Pressed {
        /// The tab.
        browser_id: BrowserId,
        /// The key.
        key: RequiredText,
        /// The element, when one was named.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
    },
    /// Files set on an input. The paths were not read.
    Uploaded {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// The paths.
        paths: Vec<RequiredText>,
    },
    /// A wait that matched.
    Waited {
        /// The tab.
        browser_id: BrowserId,
        /// Which condition matched.
        matched: Matched,
    },
    /// A scroll.
    Scrolled {
        /// The tab.
        browser_id: BrowserId,
        /// The element, when one was named.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
    },
    /// The viewport after a resize.
    Resized {
        /// The tab.
        browser_id: BrowserId,
        /// The width.
        width: ViewportSide,
        /// The height.
        height: ViewportSide,
    },
    /// Console and network rings.
    Logs {
        /// Console lines, at most the requested limit.
        console: Vec<ConsoleLine>,
        /// Network entries, at most the requested limit.
        network: Vec<NetworkEntry>,
    },
    /// An evaluation. `result_json` is not parsed.
    Evaluated {
        /// The tab.
        browser_id: BrowserId,
        /// The JSON text.
        result_json: EvaluateJson,
        /// Whether the cap bound.
        truncated: TextCap,
    },
}

/// The snapshot format. This slice's double always answers `aria-yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotFormat {
    /// YAML from the accessibility tree.
    AriaYaml,
    /// A format a newer engine names.
    #[serde(other)]
    Unknown,
}

/// The only image type this slice returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ImageMime {
    /// `image/png`.
    ImagePng,
    /// A type a newer engine names.
    #[serde(other)]
    Unknown,
}

/// Which wait condition matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Matched {
    /// The text was present.
    Text,
    /// The URL substring was present.
    Url,
}

/// Whether a tab is the one the page calls active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TabFocus {
    /// Not the active tab. A new tab is this: Paseo opens it in the
    /// background.
    #[default]
    Background,
    /// The active tab.
    Active,
}

/// Whether a tab is loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TabLoad {
    /// The load has settled.
    #[default]
    Settled,
    /// Still loading.
    Loading,
}

/// Whether a history step exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HistoryMove {
    /// The stack has a page.
    Open,
    /// The stack is empty.
    Empty,
}

impl fmt::Display for ImageMime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImagePng => f.write_str("image/png"),
            Self::Unknown => f.write_str("application/octet-stream"),
        }
    }
}
