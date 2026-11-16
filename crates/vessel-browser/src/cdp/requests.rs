//! Typed CDP requests; this sealed vocabulary is not a caller-supplied method tunnel.

use serde::{Deserialize, Serialize};
use vessel_proto::browser::{
    BrowserUrl, InputText, PageText, RequiredInput, ScreenshotPng, ViewportSize,
};

use super::identity::{SessionId, TargetId};

mod sealed {
    /// Implemented only for this module's known requests.
    pub(crate) trait Sealed {}
}

/// Only this module can add a command and select its wire method/response type.
pub(crate) trait Request: sealed::Sealed + Serialize + Send {
    /// Known response shape decoded by the CDP edge.
    type Response: serde::de::DeserializeOwned;
    /// Fixed foreign method spelling, never derived from a tool argument.
    const METHOD: Method;
}

macro_rules! methods {
    ($($name:ident => $wire:literal),+ $(,)?) => {
        /// A closed internal debugging vocabulary, distinct from user-facing `BrowserTool`.
        #[derive(Debug, Clone, Copy, Serialize)]
        pub(crate) enum Method { $(#[doc=concat!("`",$wire,"`") ] #[serde(rename=$wire)] $name,)+ }
    }
}
methods! {
    Version => "Browser.getVersion",
    CloseBrowser => "Browser.close",
    CreateTarget => "Target.createTarget",
    CloseTarget => "Target.closeTarget",
    Attach => "Target.attachToTarget",
    EnablePage => "Page.enable",
    EnableRuntime => "Runtime.enable",
    EnableNetwork => "Network.enable",
    Navigate => "Page.navigate",
    Stop => "Page.stopLoading",
    Reload => "Page.reload",
    History => "Page.getNavigationHistory",
    NavigateHistory => "Page.navigateToHistoryEntry",
    Evaluate => "Runtime.evaluate",
    Capture => "Page.captureScreenshot",
    LayoutMetrics => "Page.getLayoutMetrics",
    SetViewport => "Emulation.setDeviceMetricsOverride",
    ClearViewport => "Emulation.clearDeviceMetricsOverride",
    InsertText => "Input.insertText",
    MouseEvent => "Input.dispatchMouseEvent",
    KeyEvent => "Input.dispatchKeyEvent",
    FrameTree => "Page.getFrameTree",
    IsolatedWorld => "Page.createIsolatedWorld",
    CallFunction => "Runtime.callFunctionOn",
}

macro_rules! request {
    ($name:ty,$response:ty,$method:ident) => {
        impl sealed::Sealed for $name {}
        impl Request for $name {
            type Response = $response;
            const METHOD: Method = Method::$method;
        }
    };
}
mod keyboard;
mod modifiers;
mod pointer;
pub(crate) use keyboard::{KeyboardError, KeyboardState};
mod capture;
pub(crate) use capture::{CapturePage, LayoutMetrics};
mod condition;
mod snapshot;
pub(crate) use condition::ConditionProbe;
pub(crate) use pointer::{MouseEvent, PointerState};
pub(crate) use snapshot::{FrameTree, IsolatedWorld, SnapshotTraversal};

macro_rules! empty_requests {
    ($($name:ident => $response:ty,$method:ident,$doc:literal;)+) => {$(
        #[doc=$doc]
        #[derive(Debug, Clone, Copy, Serialize)]
        pub(crate) struct $name {}
        request!($name,$response,$method);
    )+};
}

/// An empty acknowledgement is not a fabricated page result.
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct Acknowledged {}
empty_requests! {
    Version => BrowserVersion,Version,"Inspect the connected runtime's reported version, not its path alone.";
    CloseBrowser => Acknowledged,CloseBrowser,"Request owned-browser shutdown; process-group cleanup is still required.";
    EnablePage => Acknowledged,EnablePage,"Observe the page's navigation and dialog lifecycle.";
    EnableRuntime => Acknowledged,EnableRuntime,"Observe console/exception events without a host bridge.";
    EnableNetwork => Acknowledged,EnableNetwork,"Observe bounded metadata; never collect request headers/bodies.";
    Stop => Acknowledged,Stop,"Stop loading without implying a successful reload.";
    Reload => Acknowledged,Reload,"Reload the current target, invalidating refs first.";
    Metrics => LayoutMetrics,LayoutMetrics,"Read explicit CSS content dimensions before a full-page surface is admitted.";
    History => NavigationHistory,History,"Read actual browser history before selecting a history entry.";
    Capture => Captured,Capture,"Capture a viewport PNG from a real engine target.";
    ClearViewport => Acknowledged,ClearViewport,"Clear a metrics override before responsive sizing resumes.";
}

/// Version strings are content facts, not runtime qualification by themselves.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserVersion {
    /// Reported protocol version.
    pub(crate) protocol_version: PageText,
    /// Reported product/version.
    pub(crate) product: PageText,
    /// Build revision.
    pub(crate) revision: PageText,
    /// The actual user-agent string is never injected into a host diagnostic.
    pub(crate) user_agent: PageText,
    /// Engine JavaScript version.
    pub(crate) js_version: PageText,
}

/// General creation accepts an HTTP(S) URL, never a file/debugging URL.
#[derive(Debug, Serialize)]
pub(crate) struct CreateTarget {
    /// Initial URL still needs network/admission checks in the controller.
    pub(crate) url: BrowserUrl,
    /// Background creation cannot steal native pane focus.
    pub(crate) background: bool,
}
/// Only the node actor binds this private handle to a public `BrowserId`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Created {
    /// Private target handle.
    pub(crate) target_id: TargetId,
}
request!(CreateTarget, Created, CreateTarget);

/// Closing a CDP target never exposes its private id through a tool response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseTarget {
    /// The target the actor owns.
    pub(crate) target_id: TargetId,
}
/// A backend refusal to close must not appear as completed close.
#[derive(Debug, Deserialize)]
pub(crate) struct Closed {
    /// The browser acknowledged closing the target.
    pub(crate) success: bool,
}
request!(CloseTarget, Closed, CloseTarget);

/// Attach uses flattened sessions on the one private browser connection.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Attach {
    target_id: TargetId,
    flatten: bool,
}
impl Attach {
    /// Flattening is policy, not a caller-controlled debugging mode.
    #[must_use]
    pub(crate) fn new(target_id: TargetId) -> Self {
        Self {
            target_id,
            flatten: true,
        }
    }
}
/// This is unrelated to a Vessel session credential.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Attached {
    /// Flattened CDP session.
    pub(crate) session_id: SessionId,
}
request!(Attach, Attached, Attach);

/// URL preparation is already typed; it does not replace proxy destination checks.
#[derive(Debug, Serialize)]
pub(crate) struct Navigate {
    /// The requested HTTP(S) page.
    pub(crate) url: BrowserUrl,
}
/// Page errors are retained as bounded redacted content, not RPC messages.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Navigated {
    /// Missing means no immediate navigation error was reported.
    pub(crate) error_text: Option<PageText>,
}
request!(Navigate, Navigated, Navigate);

/// A history id is meaningful only in the target's current history list.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct HistoryEntryId(i64);
/// History URLs may be a browser's internal initial blank entry.
#[derive(Debug, Deserialize)]
pub(crate) struct HistoryEntry {
    /// Use only after reading this target's current list.
    pub(crate) id: HistoryEntryId,
    /// Opaque location fact; the controller refuses unsupported destinations.
    pub(crate) url: PageText,
}
/// The actor bounds retained history before presenting eligibility facts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationHistory {
    /// Position is a checked foreign index at the adapter, not a session id.
    pub(crate) current_index: HistoryIndex,
    /// CDP transport size is bounded before decoding this list.
    pub(crate) entries: Vec<HistoryEntry>,
}
/// A foreign signed index must be checked before selecting any entry.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
pub(crate) struct HistoryIndex(i64);
impl HistoryIndex {
    /// Negative/overflowing indices cannot select an element.
    #[must_use]
    pub(crate) fn position(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}
/// The actor selects a real id after checking back/forward eligibility.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigateHistory {
    /// No synthetic relative index reaches CDP.
    pub(crate) entry_id: HistoryEntryId,
}
request!(NavigateHistory, Acknowledged, NavigateHistory);

/// Page expression text is redacted just like agent functions and human input.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Evaluate {
    expression: RequiredInput,
    return_by_value: bool,
    await_promise: bool,
    user_gesture: bool,
}
impl Evaluate {
    /// Return value-only page data and await a result under the finite command deadline.
    #[must_use]
    pub(crate) fn new(expression: RequiredInput) -> Self {
        Self {
            expression,
            return_by_value: true,
            await_promise: true,
            // Evaluation must not mint a trusted browser user gesture.
            user_gesture: false,
        }
    }
}
/// Runtime result interpretation remains inside the CDP edge.
#[derive(Debug, Deserialize)]
pub(crate) struct Evaluated {
    /// The wrapper has no raw Debug rendering.
    result: super::wire::Payload,
    /// Exceptions must not become successful evaluate echoes.
    #[serde(rename = "exceptionDetails")]
    exception: Option<super::wire::Payload>,
}
impl Evaluated {
    /// Presence is a typed failure fact; exception text is never an RPC diagnostic.
    #[must_use]
    pub(crate) fn threw(&self) -> bool {
        self.exception.is_some()
    }
    /// Decode the known `Runtime.RemoteObject` shape at this foreign-protocol boundary.
    /// # Errors
    /// Page exceptions refuse before decoding, and malformed remote objects retain the JSON cause.
    pub(crate) fn remote_object<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T, EvaluationError> {
        if self.threw() {
            return Err(EvaluationError::Exception);
        }
        self.result.decode().map_err(EvaluationError::Decode)
    }
}
request!(Evaluate, Evaluated, Evaluate);

/// The actual base64 PNG is checked before retaining compressed bytes or raster geometry.
#[derive(Debug, Deserialize)]
pub(crate) struct Captured {
    /// No 1x1 fixture is substituted for a missing frame.
    pub(crate) data: ScreenshotPng,
}

/// Device scale is fixed to one in the initial remotely rendered engine.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetViewport {
    width: vessel_proto::browser::ViewportSide,
    height: vessel_proto::browser::ViewportSide,
    device_scale_factor: DeviceScale,
    mobile: bool,
}
#[derive(Debug, Serialize)]
#[serde(transparent)]
struct DeviceScale(u8);
impl SetViewport {
    /// CSS dimensions remain distinct from actual screenshot pixel geometry.
    #[must_use]
    pub(crate) const fn new(size: ViewportSize) -> Self {
        Self {
            width: size.width,
            height: size.height,
            device_scale_factor: DeviceScale(1),
            mobile: false,
        }
    }
}
request!(SetViewport, Acknowledged, SetViewport);

/// Trusted text insertion is separate from a host keyboard or page-evaluation string.
#[derive(Debug, Serialize)]
pub(crate) struct InsertText {
    /// Raw tool input remains in its harness history; human input does not.
    pub(crate) text: InputText,
}
request!(InsertText, Acknowledged, InsertText);

/// Evaluation exceptions and remote-object decode failures are separate typed outcomes.
#[derive(thiserror::Error)]
pub(crate) enum EvaluationError {
    /// Page code threw; its content is not an RPC diagnostic.
    #[error("page evaluation raised an exception")]
    Exception,
    /// The foreign object shape differed from the known adapter response.
    #[error("page evaluation result could not be decoded")]
    Decode(#[source] serde_json::Error),
}
impl std::fmt::Debug for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CdpEvaluationError({self})")
    }
}
