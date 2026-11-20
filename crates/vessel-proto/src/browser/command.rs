//! One browser command. Twenty-two tools, one method (ADR 0069).

use serde::{Deserialize, Serialize};

use crate::{ProjectId, WorktreeId};

use super::id::{BrowserId, ElementRef};
use super::limits::{CssDelta, LogLimit, ViewportSize, WaitMs};
use super::text::{BrowserText, RequiredText};
use super::url::BrowserUrl;
use super::InvalidInput;

/// The workspace a tab belongs to: a project, and a worktree when the
/// session runs in one. Tabs are not global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserSpace {
    /// The project.
    pub project: ProjectId,
    /// The worktree, when the session has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorktreeId>,
}

/// `browser.call`: the command, and the workspace when the caller is not
/// an agent of a session that already has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserCall {
    /// The workspace. An agent omits it and is held to its session's.
    /// A device or the node token names one; without it, `list` and `new`
    /// are [`super::BrowserFailure::NoWorkspace`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<BrowserSpace>,
    /// What to do.
    pub command: BrowserCommand,
}

/// Which mouse button. Not a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClickButton {
    /// The left button. The default.
    #[default]
    Left,
    /// The right button.
    Right,
    /// The middle button.
    Middle,
    /// A button a newer peer names.
    #[serde(other)]
    Unknown,
}

/// One click or two. Not a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClickCount {
    /// A single click. The default.
    #[default]
    Single,
    /// A double click.
    Double,
    /// A count a newer peer names.
    #[serde(other)]
    Unknown,
}

/// A modifier held during a click or a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum KeyModifier {
    /// Alt.
    #[serde(rename = "Alt")]
    Alt,
    /// Control.
    #[serde(rename = "Control")]
    Control,
    /// Meta.
    #[serde(rename = "Meta")]
    Meta,
    /// Shift.
    #[serde(rename = "Shift")]
    Shift,
    /// A modifier a newer peer names.
    #[serde(other)]
    Unknown,
}

/// How much of the page a screenshot takes. Not a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ShotExtent {
    /// The viewport. The default.
    #[default]
    Viewport,
    /// The whole page.
    Page,
    /// An extent a newer peer names.
    #[serde(other)]
    Unknown,
}

/// What `browser_wait` waits for. Exactly one of the two.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum WaitExpect {
    /// Text that must appear.
    Text(RequiredText),
    /// A URL substring that must appear.
    Url(RequiredText),
}

/// `text` and `url` into one [`WaitExpect`]. Both, or neither, is an error.
///
/// # Errors
/// [`InvalidInput::WaitBoth`].
pub fn wait_expect(text: Option<&str>, url: Option<&str>) -> Result<WaitExpect, InvalidInput> {
    let text = text.filter(|text| !text.is_empty());
    let url = url.filter(|url| !url.is_empty());
    match (text, url) {
        (Some(text), None) => Ok(WaitExpect::Text(RequiredText::non_empty(
            text,
            InvalidInput::WaitBoth,
        )?)),
        (None, Some(url)) => Ok(WaitExpect::Url(RequiredText::non_empty(
            url,
            InvalidInput::WaitBoth,
        )?)),
        (Some(_), Some(_)) | (None, None) => Err(InvalidInput::WaitBoth),
    }
}

/// The closed set of browser commands. Match it without a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum BrowserCommand {
    /// The workspace's open tabs.
    ListTabs,
    /// A background tab. An omitted URL is the double's `https://example.com`.
    NewTab {
        /// The URL, when the caller set one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<BrowserUrl>,
    },
    /// Close a tab.
    CloseTab {
        /// The tab.
        browser_id: BrowserId,
    },
    /// Navigate a tab. The URL is already `http` or `https`.
    Navigate {
        /// The tab.
        browser_id: BrowserId,
        /// The URL.
        url: BrowserUrl,
    },
    /// Back.
    Back {
        /// The tab.
        browser_id: BrowserId,
    },
    /// Forward.
    Forward {
        /// The tab.
        browser_id: BrowserId,
    },
    /// Reload.
    Reload {
        /// The tab.
        browser_id: BrowserId,
    },
    /// The accessibility snapshot.
    Snapshot {
        /// The tab.
        browser_id: BrowserId,
    },
    /// A PNG of the viewport or the page.
    Screenshot {
        /// The tab.
        browser_id: BrowserId,
        /// Viewport or page.
        #[serde(default)]
        extent: ShotExtent,
    },
    /// Click an element from the latest snapshot.
    Click {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// Which button.
        #[serde(default)]
        button: ClickButton,
        /// Single or double.
        #[serde(default)]
        count: ClickCount,
        /// Modifiers held down.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        modifiers: Vec<KeyModifier>,
    },
    /// Hover an element.
    Hover {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
    },
    /// Drag one element onto another.
    Drag {
        /// The tab.
        browser_id: BrowserId,
        /// The element that moves.
        source: ElementRef,
        /// The element it is dropped on.
        target: ElementRef,
    },
    /// Replace an element's value. Empty is a clear. The text stays in
    /// the session log as this call's input.
    Fill {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// The value, including empty.
        value: BrowserText,
    },
    /// Choose an option.
    Select {
        /// The tab.
        browser_id: BrowserId,
        /// The element.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// The option.
        value: BrowserText,
    },
    /// Type text. No element means the focused one. The text stays in
    /// the session log as this call's input.
    Type {
        /// The tab.
        browser_id: BrowserId,
        /// The element, or the focused one when absent.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
        /// The text.
        text: BrowserText,
    },
    /// Press a key. The key stays in the session log as this call's input.
    Keypress {
        /// The tab.
        browser_id: BrowserId,
        /// The element, or the focused one when absent.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
        /// The key, at least one character.
        key: RequiredText,
    },
    /// Set files on an input. The paths are checked, not read.
    Upload {
        /// The tab.
        browser_id: BrowserId,
        /// The file input.
        #[serde(rename = "ref")]
        element: ElementRef,
        /// At least one path.
        paths: Vec<RequiredText>,
    },
    /// Wait until text or a URL substring is present.
    Wait {
        /// The tab.
        browser_id: BrowserId,
        /// Text or URL, not both.
        expect: WaitExpect,
        /// When set, the broker deadline is this plus one second.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<WaitMs>,
    },
    /// Scroll, by CSS pixels, at an element or the page.
    Scroll {
        /// The tab.
        browser_id: BrowserId,
        /// The element, or the page when absent.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
        /// The CSS pixel delta.
        delta: CssDelta,
    },
    /// Resize the viewport.
    Resize {
        /// The tab.
        browser_id: BrowserId,
        /// The new size.
        viewport: ViewportSize,
    },
    /// Console and network entries.
    Logs {
        /// The tab.
        browser_id: BrowserId,
        /// How many of each. Absent is 50.
        #[serde(default)]
        limit: LogLimit,
    },
    /// A function the real engine would run. The result is a string.
    /// The text of the function stays in the session log as this call's
    /// input. The test double does not run it.
    Evaluate {
        /// The tab.
        browser_id: BrowserId,
        /// The element, passed as the function's first argument.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        element: Option<ElementRef>,
        /// The function source, at least one character.
        function: RequiredText,
    },
}

impl BrowserCommand {
    /// Refuse a button, count, modifier or extent this build does not know.
    ///
    /// # Errors
    /// [`InvalidInput::UnknownChoice`].
    pub fn known(&self) -> Result<(), InvalidInput> {
        match self {
            Self::ListTabs
            | Self::NewTab { .. }
            | Self::CloseTab { .. }
            | Self::Navigate { .. }
            | Self::Back { .. }
            | Self::Forward { .. }
            | Self::Reload { .. }
            | Self::Snapshot { .. }
            | Self::Hover { .. }
            | Self::Drag { .. }
            | Self::Fill { .. }
            | Self::Select { .. }
            | Self::Type { .. }
            | Self::Keypress { .. }
            | Self::Upload { .. }
            | Self::Wait { .. }
            | Self::Scroll { .. }
            | Self::Resize { .. }
            | Self::Logs { .. }
            | Self::Evaluate { .. } => Ok(()),
            Self::Screenshot { extent, .. } => match extent {
                ShotExtent::Viewport | ShotExtent::Page => Ok(()),
                ShotExtent::Unknown => Err(InvalidInput::UnknownChoice),
            },
            Self::Click {
                button,
                count,
                modifiers,
                ..
            } => {
                let button_known = matches!(
                    button,
                    ClickButton::Left | ClickButton::Right | ClickButton::Middle
                );
                let count_known = matches!(count, ClickCount::Single | ClickCount::Double);
                let modifiers_known = modifiers.iter().all(|modifier| {
                    matches!(
                        modifier,
                        KeyModifier::Alt
                            | KeyModifier::Control
                            | KeyModifier::Meta
                            | KeyModifier::Shift
                    )
                });
                if button_known && count_known && modifiers_known {
                    Ok(())
                } else {
                    Err(InvalidInput::UnknownChoice)
                }
            }
        }
    }
}
