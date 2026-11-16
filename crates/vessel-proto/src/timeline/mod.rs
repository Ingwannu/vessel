//! Standalone canonical display and private correlation values; no event/RPC attachment.
//! Provider parsing, execution, authority and session ownership stay outside proto.
pub mod admission;
mod detail;
mod execution;
mod identity;
mod measure;
mod notice;
mod permission;
mod question;
mod receipt;
mod recovery;
mod submission;

pub use detail::*;
pub use identity::*;
pub use measure::{DetailCount, DetailIndex, DetailOffset, ToolByteCount, ToolExitCode};
pub use notice::{NoticeCheckpoint, NoticeKey, NoticeRequest};

/// The single timeline feature remains unassigned; these values activate no peer behavior.
pub const TIMELINE_PARITY: Option<crate::ProtocolVersion> = None;

// Standalone values only: no request-field, event or method attachment.
pub use execution::{ExecutionInvalidation, FinishGrantCeiling, ParentExecutionStamp};
pub use permission::{
    PermissionAction, PermissionActionIntent, PermissionActionVariant, PermissionBehavior,
    PermissionDeliveryState, PermissionKind, PermissionPresentation,
};
pub use question::{
    EmptyAnswerPolicy, QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime,
};
pub use receipt::{PromptReceipt, SubmitPrompt, SubmitPromptReply};
pub use recovery::{ObservePromptReceipt, ObservePromptReceiptReply, PromptReceiptBinding};
pub use submission::{
    NativeCommandRefusal, PromptDisposition, SubmissionAcceptedKind, SubmissionDelivery,
    SubmissionRefusal,
};
