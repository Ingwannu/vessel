//! The CDP JSON edge; raw payloads remain opaque until a known typed adapter decodes them.
//!
//! `RawJson`'s ordinary Debug includes its text. This wrapper must therefore
//! redact every request/result/event/error-data channel before it leaves the edge.

use std::fmt;

use serde::{Deserialize, Serialize};
use vessel_proto::RawJson;
use vessel_proto::browser::PageText;

use super::identity::{CommandId, SessionId};

/// Bound transport allocation independently of screenshot base64/PNG budgets.
pub(super) const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
/// Essential state events are bounded before entering the actor's event queue.
const MAX_EVENT_BYTES: usize = 256 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct Payload(RawJson);
impl fmt::Debug for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CdpPayload([redacted])")
    }
}
impl Payload {
    pub(super) fn encode<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        RawJson::from_serialize(value).map(Self)
    }
    pub(super) fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(self.0.as_text())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Outgoing {
    pub(super) id: CommandId,
    pub(super) method: super::requests::Method,
    pub(super) params: Payload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) session_id: Option<SessionId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Packet {
    id: Option<CommandId>,
    method: Option<EventKind>,
    params: Option<Payload>,
    result: Option<Payload>,
    error: Option<RemoteError>,
    session_id: Option<SessionId>,
}

/// Remote diagnostics are retained as page content, never reflected in a node error.
#[derive(Deserialize)]
pub(crate) struct RemoteError {
    // This numeric value belongs to the foreign CDP error format only.
    code: RemoteCode,
    message: PageText,
    #[serde(default)]
    data: Option<Payload>,
}
impl fmt::Debug for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteError")
            .field("code", &self.code.0)
            .field("message", &self.message)
            .field("data", &self.data)
            .finish()
    }
}
impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CDP operation was refused")
    }
}
impl std::error::Error for RemoteError {}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
struct RemoteCode(i64);

#[derive(Debug)]
pub(super) enum Incoming {
    Response {
        id: CommandId,
        session: Option<SessionId>,
        result: Result<Payload, RemoteError>,
    },
    Event(Event),
    UnknownEvent,
}
/// A bounded essential signal; raw payload interpretation stays in this CDP module.
pub(crate) struct Event {
    pub(super) kind: EventKind,
    pub(super) session: Option<SessionId>,
    pub(super) params: Payload,
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CdpEvent")
            .field("kind", &self.kind)
            .field("session", &self.session)
            .field("params", &self.params)
            .finish()
    }
}
impl Event {
    /// Branch on a closed signal, never the foreign method's string.
    #[must_use]
    pub(crate) const fn kind(&self) -> EventKind {
        self.kind
    }
    /// Private flattened identity remains distinct from a Vessel session id.
    #[must_use]
    pub(crate) fn session(&self) -> Option<&SessionId> {
        self.session.as_ref()
    }
}

/// Essential signals consumed by the engine controller; new unrelated events stay unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum EventKind {
    #[doc = "`Target.attachedToTarget`"]
    #[serde(rename = "Target.attachedToTarget")]
    Attached,
    #[doc = "`Target.detachedFromTarget`"]
    #[serde(rename = "Target.detachedFromTarget")]
    Detached,
    #[doc = "`Target.targetCrashed`"]
    #[serde(rename = "Target.targetCrashed")]
    Crashed,
    #[doc = "`Page.frameNavigated`"]
    #[serde(rename = "Page.frameNavigated")]
    Navigated,
    #[doc = "`Page.documentOpened`"]
    #[serde(rename = "Page.documentOpened")]
    DocumentOpened,
    #[doc = "`Page.navigatedWithinDocument`"]
    #[serde(rename = "Page.navigatedWithinDocument")]
    WithinDocument,
    #[doc = "`Page.frameStartedLoading`"]
    #[serde(rename = "Page.frameStartedLoading")]
    Loading,
    #[doc = "`Page.frameStoppedLoading`"]
    #[serde(rename = "Page.frameStoppedLoading")]
    Stopped,
    #[doc = "`Page.loadEventFired`"]
    #[serde(rename = "Page.loadEventFired")]
    Loaded,
    #[doc = "`Page.javascriptDialogOpening`"]
    #[serde(rename = "Page.javascriptDialogOpening")]
    Dialog,
    #[doc = "`Runtime.consoleAPICalled`"]
    #[serde(rename = "Runtime.consoleAPICalled")]
    Console,
    #[doc = "`Runtime.exceptionThrown`"]
    #[serde(rename = "Runtime.exceptionThrown")]
    Exception,
    #[doc = "`Network.requestWillBeSent`"]
    #[serde(rename = "Network.requestWillBeSent")]
    Request,
    #[doc = "`Network.responseReceived`"]
    #[serde(rename = "Network.responseReceived")]
    Response,
    #[doc = "`Network.loadingFailed`"]
    #[serde(rename = "Network.loadingFailed")]
    RequestFailed,
    #[doc = "`Fetch.requestPaused`"]
    #[serde(rename = "Fetch.requestPaused")]
    RequestPaused,
    #[doc = "`Browser.downloadWillBegin`"]
    #[serde(rename = "Browser.downloadWillBegin")]
    Download,
    /// Ignore unrelated future signals without inventing state.
    #[serde(other)]
    Unknown,
}

/// Foreign protocol refusal, without a page-selected diagnostic string.
#[derive(thiserror::Error)]
pub(crate) enum DecodeError {
    /// Apply before decoding/queuing message content.
    #[error("CDP message exceeds its transport/event budget")]
    Size,
    /// Retain JSON cause for internal inspection only.
    #[error("CDP message could not be decoded")]
    Json(#[source] serde_json::Error),
    /// A message cannot be a response and an event together.
    #[error("CDP message mixes response/event fields or lacks an outcome")]
    Shape,
}

pub(super) fn incoming(text: &str) -> Result<Incoming, DecodeError> {
    if text.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::Size);
    }
    let packet: Packet = serde_json::from_str(text).map_err(DecodeError::Json)?;
    if let Some(id) = packet.id {
        if packet.method.is_some() || packet.params.is_some() {
            return Err(DecodeError::Shape);
        }
        let result = match (packet.result, packet.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(error),
            (Some(_), Some(_)) | (None, None) => return Err(DecodeError::Shape),
        };
        return Ok(Incoming::Response {
            id,
            session: packet.session_id,
            result,
        });
    }
    if packet.result.is_some() || packet.error.is_some() {
        return Err(DecodeError::Shape);
    }
    let kind = packet.method.ok_or(DecodeError::Shape)?;
    if kind == EventKind::Unknown {
        return Ok(Incoming::UnknownEvent);
    }
    if text.len() > MAX_EVENT_BYTES {
        return Err(DecodeError::Size);
    }
    let params = packet.params.ok_or(DecodeError::Shape)?;
    Ok(Incoming::Event(Event {
        kind,
        session: packet.session_id,
        params,
    }))
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CdpDecodeError({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_does_not_enter_debug_and_mixed_messages_cannot_satisfy_a_waiter() {
        let secret = "synthetic-typing-canary";
        let response = incoming(&format!(r#"{{"id":1,"result":{{"text":"{secret}"}}}}"#)).unwrap();
        assert!(!format!("{response:?}").contains(secret));
        for text in [
            r#"{"id":1}"#,
            r#"{"id":1,"result":{},"error":{"code":1,"message":"bad"}}"#,
            r#"{"id":1,"result":{},"method":"Page.loadEventFired","params":{}}"#,
            r#"{"id":0,"result":{}}"#,
        ] {
            assert!(incoming(text).is_err());
        }
        assert!(matches!(
            incoming(r#"{"method":"Future.event","params":{"anything":true}}"#).unwrap(),
            Incoming::UnknownEvent
        ));
    }
}
