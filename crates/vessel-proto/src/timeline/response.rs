//! Rich permission replies preserve offered option identity and explicit effects.

use crate::{EventSeq, PermissionId, PermissionOutcome, RawJson, SessionId};
use serde::{Deserialize, Serialize};

/// Interrupt is an explicit requested effect, never inferred from a button label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionInterrupt {
    /// Settle only the permission.
    #[default]
    Continue,
    /// Also stop the request's exact turn, if supported by its provider edge.
    Interrupt,
}

/// Extra response values may only be applied by the provider that understands them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionResponse {
    /// The actual offered option identity, or explicit cancellation.
    pub outcome: PermissionOutcome,
    /// Opaque only until the owning provider validates its known input schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_input: Option<RawJson>,
    /// Human-authored denial explanation, if that provider can deliver one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Unsupported interrupt intent refuses the entire response before any effect.
    #[serde(default)]
    pub interrupt: PermissionInterrupt,
}

/// One exact permission response; all-request CLI operations enumerate separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RespondPermission {
    /// Owning session, protected by the existing Operate/ownership rules.
    pub session: SessionId,
    /// Request identity, never the related tool identity.
    pub permission: PermissionId,
    /// Complete intended decision, validated before mutation.
    pub response: PermissionResponse,
}

/// A provider never silently drops a rich response field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionResponseRefusal {
    /// This edge cannot safely modify the pending operation's input.
    ModifiedInputUnsupported,
    /// This edge cannot convey an explanation with the denial.
    MessageUnsupported,
    /// This edge cannot bind an interrupt to the permission's exact turn.
    InterruptUnsupported,
    /// The identity was not an option of this request.
    UnknownOption,
    /// A native plan transition refused before effects; the proposal remains Pending.
    PlanDecision {
        /// Exact provider-owned refusal; stale keys are withdrawn separately.
        reason: super::PlanDecisionRefusal,
    },
}

/// Acknowledgement describes the real resolution, including uncertain delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RespondPermissionReply {
    /// Provider accepted and this exact resolution reached the committed event log.
    Settled {
        /// Resolved request.
        permission: PermissionId,
        /// Accepted decision, including its offered option identity.
        outcome: PermissionOutcome,
        /// Durable resolution record, for reconciliation.
        seq: EventSeq,
    },
    /// No such request remains available on the current provider.
    Vanished {
        /// Request that disappeared.
        permission: PermissionId,
    },
    /// A competing response or withdrawal won.
    Conflict {
        /// Request already resolved elsewhere.
        permission: PermissionId,
    },
    /// No field was applied and the request may still be answered differently.
    Refused {
        /// Request left unchanged.
        permission: PermissionId,
        /// Exact unsupported/invalid value category.
        reason: PermissionResponseRefusal,
    },
    /// An effect may have happened; no accepted resolution is fabricated.
    OutcomeUnknown {
        /// Request requiring reconciliation.
        permission: PermissionId,
    },
}
