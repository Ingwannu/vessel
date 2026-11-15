//! Canonical display data shared by every timeline consumer (ADR 0084).
//! Provider decoding belongs to vessel-agent; these values never execute tools.

pub mod admission;
pub(crate) mod compat;
mod detail;
mod identity;
mod item;
mod measure;
mod permission;
mod plan;
mod question;
mod question_reply;
mod receipt;
mod reference;
mod response;
mod scope;
mod submission;

pub use detail::*;
pub use identity::{
    ClientMessageId, ExternalAgentId, ExternalHostId, ExternalThreadId, ExternalTurnId,
    MAX_TIMELINE_IDENTITY_BYTES, PlanProposalId, PluginItemId, PluginKind, TimelineIdentityError,
    TimelineName, TimelinePluginId, TimelineTaskId,
};
pub use item::*;
pub use measure::{DetailCount, DetailIndex, DetailOffset, ToolByteCount, ToolExitCode};
pub use permission::{
    PermissionAction, PermissionActionIntent, PermissionActionVariant, PermissionBehavior,
    PermissionDeliveryState, PermissionKind, PermissionPresentation, PermissionReference,
};

pub use question::{
    EmptyAnswerPolicy, QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime,
    QuestionReference,
};
pub use question_reply::{QuestionResponseRefusal, RespondQuestion, RespondQuestionReply};
pub use receipt::{PromptReceipt, SubmitPrompt, SubmitPromptReply};
pub use response::{
    PermissionInterrupt, PermissionResponse, PermissionResponseRefusal, RespondPermission,
    RespondPermissionReply,
};

use crate::ProtocolVersion;

/// Activation waits for native, web, CLI, inbox and forwarded-reply consumers
/// to understand canonical details, identified prompts and durable questions.
pub const TIMELINE_PARITY: Option<ProtocolVersion> = None;

pub use scope::ToolCallScope;

pub use plan::*;

pub use reference::TimelineReferenceOrigin;

pub(crate) use reference::ReferenceContext;

#[cfg(test)]
mod reference_tests;

pub use submission::{
    NativeCommandRefusal, PromptDisposition, SubmissionAcceptedKind, SubmissionDelivery,
    SubmissionRefusal,
};
