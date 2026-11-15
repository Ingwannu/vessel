//! Errors as they appear on the wire.
//!
//! Each crate keeps its own typed error enum and maps it to a [`WireError`]
//! exactly once, at the RPC boundary. Clients branch on [`ErrorCode`], never
//! on the message text (Zed's string-only `bail!` errors all arrive as
//! "internal" and cannot be acted on; this is the fix).

use std::fmt;

use serde::{Deserialize, Serialize};

/// What kind of failure occurred. Clients branch on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request was malformed or its arguments were invalid.
    InvalidRequest,
    /// Missing or wrong credentials.
    Unauthorized,
    /// Authenticated, but the caller lacks the method's scope (protocol 4).
    Forbidden,
    /// The client and node share no protocol version.
    ProtocolMismatch,
    /// The referenced session, subscription or permission does not exist.
    NotFound,
    /// The target is in the wrong state (for example the session has ended).
    Conflict,
    /// The operation does not apply (for example terminal input sent to a
    /// structured session, or a harness without structured support).
    Unsupported,
    /// The harness binary (or its adapter) is not installed on the node.
    HarnessNotInstalled,
    /// The harness process could not be started.
    SpawnFailed,
    /// The node is shutting down or overloaded; retrying later may work.
    Unavailable,
    /// A bug or an unexpected I/O failure inside the node.
    Internal,
}

/// An error returned in place of a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WireError {
    /// Machine-readable kind.
    pub code: ErrorCode,
    /// Human-readable explanation. Never contains secrets.
    pub message: String,
    /// Optional machine-readable reason (protocol 13); older errors omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<ErrorDetail>,
}

/// Further typed context for a refusal. Unknown future reasons stay readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ErrorDetail {
    /// Content-free diagnostic collection admission.
    Diagnostics {
        /// Resource refusal, never a source error message.
        reason: crate::diagnostics::DiagnosticRefusal,
    },
    /// A schedule preview could not be computed.
    SchedulePreview {
        /// The stable preview reason.
        reason: crate::automation::SchedulePreviewError,
    },
    /// A fleet control could not complete.
    FleetControl {
        /// The stable fleet reason.
        reason: crate::FleetControlError,
    },
    /// The node cannot issue a pairing invitation.
    DeviceInvite {
        /// The stable pairing setup reason.
        reason: crate::DeviceInviteError,
    },
    /// A spoken-conversation refusal contains no provider diagnostic or audio.
    Voice {
        /// A closed public reason, never credential or transcript content.
        reason: crate::voice::VoiceFailure,
    },
    /// A reason introduced by a newer node.
    #[serde(other)]
    Unknown,
}

impl WireError {
    /// Build an error.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for WireError {}

#[cfg(test)]
mod detail_tests {
    use super::*;

    #[test]
    fn old_error_shape_and_future_reasons_stay_readable() {
        let old = r#"{"code":"invalid_request","message":"choose another time"}"#;
        let error: WireError = serde_json::from_str(old).unwrap();
        assert_eq!(error.detail, None);
        assert_eq!(serde_json::to_string(&error).unwrap(), old);
        let future = r#"{"code":"invalid_request","message":"update","detail":{"kind":"schedule_preview","reason":"future_reason"}}"#;
        let error: WireError = serde_json::from_str(future).unwrap();
        assert_eq!(
            error.detail,
            Some(ErrorDetail::SchedulePreview {
                reason: crate::automation::SchedulePreviewError::Unknown
            })
        );
        let future = r#"{"code":"invalid_request","message":"update","detail":{"kind":"future_kind","reason":"future_reason"}}"#;
        assert_eq!(
            serde_json::from_str::<WireError>(future).unwrap().detail,
            Some(ErrorDetail::Unknown)
        );
    }
}

#[cfg(test)]
mod invite_tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct OldWireError {
        code: ErrorCode,
        message: String,
    }

    #[test]
    fn pairing_setup_errors_are_typed_and_old_clients_keep_guidance() {
        let text = r#"{"code":"conflict","message":"configure a direct address or relay","detail":{"kind":"device_invite","reason":"no_routes"}}"#;
        let error: WireError = serde_json::from_str(text).unwrap();
        assert_eq!(
            error.detail,
            Some(ErrorDetail::DeviceInvite {
                reason: crate::DeviceInviteError::NoRoutes
            })
        );
        assert_eq!(serde_json::to_string(&error).unwrap(), text);
        assert_eq!(
            serde_json::from_str::<OldWireError>(text).unwrap(),
            OldWireError {
                code: ErrorCode::Conflict,
                message: "configure a direct address or relay".to_owned()
            }
        );
        let future = r#"{"code":"conflict","message":"new setup guidance","detail":{"kind":"device_invite","reason":"future_reason"}}"#;
        assert_eq!(
            serde_json::from_str::<WireError>(future).unwrap().detail,
            Some(ErrorDetail::DeviceInvite {
                reason: crate::DeviceInviteError::Unknown
            })
        );
    }
}
