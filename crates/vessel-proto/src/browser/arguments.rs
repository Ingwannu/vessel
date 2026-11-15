//! Typed browser arguments cannot become an arbitrary CDP request.
//!
//! MCP booleans/names are converted at its decoding edge. This contract carries
//! explicit meanings and validated quantities, not raw JSON or global host paths.

use serde::{Deserialize, Serialize};

use super::bounds::{LogCount, ViewportSize, WaitBudget};
use super::identity::{BrowserId, ElementRef};
use super::input::{InputText, RequiredInput, WaitFor};
use super::url::BrowserUrl;

/// Commands without arguments still have one explicit typed payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NoBrowserArgs {}

/// A new tab uses the documented default URL when no URL is supplied.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NewTabArgs {
    /// General/public or an explicitly admitted workspace Preview origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<BrowserUrl>,
}

/// Target ownership is checked independently of the operation's arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TabArgs {
    /// This is a node/browser id, never a CDP target id.
    pub browser_id: BrowserId,
}

/// Element actions must resolve a reference from this target's current snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ElementArgs {
    /// The tab whose reference map must contain the element.
    pub browser_id: BrowserId,
    /// The snapshot reference, not an arbitrary JavaScript expression.
    pub element: ElementRef,
}

/// Distinguish a mouse button from key modifiers or a click count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    /// Reference default for a click.
    #[default]
    Left,
    /// Context-menu intent remains a trusted input event.
    Right,
    /// Middle click can initiate a popup governed by the target's policy.
    Middle,
}

/// A boolean at the MCP edge becomes an explicit click-count meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClickCount {
    /// The reference's default.
    #[default]
    Single,
    /// Trusted double-click sequence.
    Double,
}

/// Modifier names are decoded once; unknown strings cannot reach input dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum KeyModifier {
    /// Alternate-key modifier.
    Alt,
    /// Control-key modifier.
    Control,
    /// Platform meta-key modifier.
    Meta,
    /// Shift-key modifier.
    Shift,
}

/// A click cannot quietly discard the requested button/count/modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClickArgs {
    /// Resolve both ownership and the reference in this tab.
    pub target: ElementArgs,
    /// Defaults to the left button at both decoding edges.
    #[serde(default)]
    pub button: MouseButton,
    /// Defaults to one click; never an unexplained boolean on the wire.
    #[serde(default)]
    pub count: ClickCount,
    /// Hold only these known modifiers during trusted input.
    #[serde(default)]
    pub modifiers: Vec<KeyModifier>,
}

/// Fill/select may legitimately set an empty value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ValueArgs {
    /// The current element reference identifies the form control.
    pub target: ElementArgs,
    /// Intentionally serialized page input, redacted from Debug.
    pub value: InputText,
}

/// Both wait conditions cannot be supplied simultaneously in a typed request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WaitArgs {
    /// Ownership is still required for a read-like operation on signed-in data.
    pub browser_id: BrowserId,
    /// Text or a URL substring, not an arbitrary wait script.
    pub condition: WaitFor,
    /// The broker's extra response allowance is applied outside this duration.
    #[serde(default)]
    pub budget: WaitBudget,
}

/// An absent reference means current focus, rather than a magic empty ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TypeArgs {
    /// The focus belongs to this tab.
    pub browser_id: BrowserId,
    /// Optionally establish a validated element focus before typing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
    /// Text is not a host key/chord or a JavaScript function.
    pub text: InputText,
}

/// Keys remain separate from text so Enter does not become literal page text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KeypressArgs {
    /// The tab receives only a supported parsed chord.
    pub browser_id: BrowserId,
    /// Optional focus follows the same reference-generation check as typing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
    /// A chord is parsed by the engine input boundary before any key goes down.
    pub key: RequiredInput,
}

/// Navigation cannot smuggle an external-handler or credential-bearing URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NavigateArgs {
    /// Navigating a General target does not mint a Preview capability.
    pub browser_id: BrowserId,
    /// The policy proxy must also admit the resolved destination.
    pub url: BrowserUrl,
}

/// Screenshot scope is an explicit intent, not a boolean with hidden meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotExtent {
    /// Reference default: the current viewport.
    #[default]
    Viewport,
    /// Full content remains subject to the encoded/decoded image bound.
    Page,
}

/// A screenshot needs a real target/paint even when the panel is hidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScreenshotArgs {
    /// The target to capture, in the caller's workspace.
    pub browser_id: BrowserId,
    /// Full-page is a bounded explicit request.
    #[serde(default)]
    pub extent: ScreenshotExtent,
}

/// Both drag endpoints are validated before any pointer press is delivered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DragArgs {
    /// Both refs must belong to this same target's current generation.
    pub browser_id: BrowserId,
    /// The press begins at this element.
    pub source: ElementRef,
    /// The release ends at this element.
    pub destination: ElementRef,
}

/// Page evaluation is not a host process or an arbitrary CDP method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EvaluateArgs {
    /// Browser policy and signed-in-data authority apply to this tab.
    pub browser_id: BrowserId,
    /// The function's source is redacted from diagnostics.
    pub function: RequiredInput,
    /// Optional element passed to that function, checked before execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
}

/// Finite scroll units cannot become `NaN`/Infinity inside input dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct CssDelta(pub crate::conversation::FiniteNumber);

/// Scroll acts on the current tab or a snapshot element, never a global surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScrollArgs {
    /// The target owns its current focus and reference generation.
    pub browser_id: BrowserId,
    /// A missing element means the viewport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
    /// Horizontal CSS-pixel wheel delta.
    pub delta_x: CssDelta,
    /// Vertical CSS-pixel wheel delta.
    pub delta_y: CssDelta,
}

/// Dimensions are checked together before either override or retained state changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResizeArgs {
    /// The same target supplies the app's frames and the agent's tools.
    pub browser_id: BrowserId,
    /// The complete new CSS viewport.
    pub size: ViewportSize,
}

/// Logs are a bounded page-content result, not a side effect on tracing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LogsArgs {
    /// Only this authorized target's retained ring is visible.
    pub browser_id: BrowserId,
    /// Apply the reference's default when omitted.
    #[serde(default)]
    pub count: LogCount,
}
