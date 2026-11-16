//! Browser refusals never reflect a credential, URL, page or evaluated result.
//!
//! The engine retains its process/network/parser source internally and projects
//! one of these reasons at the RPC edge. A timeout after delivery is uncertain,
//! so a client cannot mistake it for permission to replay a mutation.

use serde::{Deserialize, Serialize};

use crate::ErrorCode;

/// History direction is explicit even when there is no entry to navigate to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HistoryDirection {
    /// A preceding entry.
    Back,
    /// A later entry.
    Forward,
}

/// Delivery to the page matters more than a lost response after the deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserEffect {
    /// The command never crossed the browser execution boundary.
    NotDelivered,
    /// It may have changed the page; a client must not automatically replay it.
    Uncertain,
}

/// Typed failure projection, independent of backend/library diagnostic text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BrowserFailure {
    /// A disabled or never-offered session cannot gain tools by guessing a call.
    #[error("browser tools are off for this session")]
    Disabled,
    /// A session/workspace context is required even for listing tabs.
    #[error("this browser command requires a workspace")]
    NoWorkspace,
    /// No external handlers, file navigation or arbitrary app scheme.
    #[error("browser navigation supports only HTTP and HTTPS")]
    Scheme,
    /// No silent no-op when the requested history stack is empty.
    #[error("there is no {direction:?} history entry")]
    NoHistory {
        /// The stack that was empty.
        direction: HistoryDirection,
    },
    /// The complete request may not reach that destination.
    #[error("the browser destination policy refused this address")]
    Destination,
    /// Runtime absence, failed qualification and missing runtime prerequisites.
    #[error("this node has no qualified browser engine")]
    NoEngine,
    /// Do not disclose a tab that exists under a different workspace/session.
    #[error("no such browser tab is available in this context")]
    UnknownTab,
    /// Closing a target settles its pending actions rather than reusing its id.
    #[error("this browser tab has closed")]
    Closed,
    /// A ref from another navigation/snapshot cannot select a new element.
    #[error("this element reference is no longer current")]
    StaleRef,
    /// The actor preserves whether a page side effect may already have occurred.
    #[error("browser command timed out; delivery state: {effect:?}")]
    TimedOut {
        /// Controls replay guidance; never infer it from a transport error alone.
        effect: BrowserEffect,
    },
    /// The actual engine/runtime must support the requested operation.
    #[error("this browser engine does not support the operation")]
    Unsupported,
    /// A screenshot cannot succeed with a blank fixture before first paint.
    #[error("the browser tab has no painted frame yet")]
    NoFrame,
    /// Admission is bounded before a large payload is queued.
    #[error("the browser command queue is full")]
    Busy,
    /// OS/process details remain an internal source, not an unfiltered wire message.
    #[error("the browser process could not be started")]
    Launch,
    /// An unsafe profile never becomes writable through a blanket permission repair.
    #[error("the browser profile does not have safe private permissions")]
    ProfilePermissions,
    /// A private preview loses authority when the owner/lifetime cannot be proved.
    #[error("the workspace service preview capability is no longer valid")]
    PreviewRevoked,
    /// Parse failures are not an internal engine bug.
    #[error("the browser command arguments are invalid")]
    Invalid,
    /// Unexpected engine failure has no page-content exception string on the wire.
    #[error("the browser operation failed internally")]
    Internal,
    /// A later reason remains a refusal for an older decoder.
    #[serde(other)]
    #[error("this browser failure is not understood by this client")]
    Unknown,
}

impl BrowserFailure {
    /// Preserve the existing RPC error classes instead of introducing a second retry flag.
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::Disabled | Self::Unsupported | Self::Unknown => ErrorCode::Unsupported,
            Self::NoWorkspace
            | Self::Scheme
            | Self::NoHistory { .. }
            | Self::Destination
            | Self::ProfilePermissions
            | Self::PreviewRevoked => ErrorCode::Forbidden,
            Self::NoEngine | Self::TimedOut { .. } | Self::NoFrame | Self::Busy | Self::Launch => {
                ErrorCode::Unavailable
            }
            Self::UnknownTab => ErrorCode::NotFound,
            Self::Closed | Self::StaleRef => ErrorCode::Conflict,
            Self::Invalid => ErrorCode::InvalidRequest,
            Self::Internal => ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertain_delivery_survives_round_trip_and_is_not_a_success() {
        let failure = BrowserFailure::TimedOut {
            effect: BrowserEffect::Uncertain,
        };
        assert_eq!(
            serde_json::from_str::<BrowserFailure>(&serde_json::to_string(&failure).unwrap())
                .unwrap(),
            failure
        );
        assert_eq!(failure.code(), ErrorCode::Unavailable);
        assert_ne!(
            failure,
            BrowserFailure::TimedOut {
                effect: BrowserEffect::NotDelivered
            }
        );
    }

    #[test]
    fn unknown_reasons_preserve_failure_instead_of_defaulting_to_ready() {
        let failure: BrowserFailure =
            serde_json::from_str(r#"{"reason":"future_failure"}"#).unwrap();
        assert_eq!(failure, BrowserFailure::Unknown);
        assert_eq!(failure.code(), ErrorCode::Unsupported);
    }
}
