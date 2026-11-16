//! Canonical display data shared by every timeline consumer (ADR 0084).
//! Provider decoding belongs to vessel-agent; these values never execute tools.

pub mod admission;
mod detail;
mod identity;
mod item;
mod measure;
mod notice;
mod permission;
mod question;
mod receipt;

pub use detail::*;
pub use identity::*;
pub use item::*;
pub use measure::{DetailCount, DetailIndex, DetailOffset, ToolByteCount, ToolExitCode};
pub use notice::{NoticeCheckpoint, NoticeKey, NoticeRequest};
pub use permission::{
    PermissionAction, PermissionActionIntent, PermissionActionVariant, PermissionBehavior,
    PermissionKind, PermissionPresentation,
};

pub use question::{
    EmptyAnswerPolicy, QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime,
};
pub use receipt::{PromptReceipt, SubmitPrompt, SubmitPromptReply};

use crate::ProtocolVersion;

/// Activation waits for native, web, CLI, inbox and forwarded-reply consumers
/// to understand canonical details, identified prompts and durable questions.
pub const TIMELINE_PARITY: Option<ProtocolVersion> = None;
