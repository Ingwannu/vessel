//! Standalone canonical display and private correlation values; no event/RPC attachment.
//! Provider parsing, execution, authority and session ownership stay outside proto.
pub mod admission;
mod detail;
mod execution;
mod identity;
mod measure;
mod notice;

pub use detail::*;
pub use execution::{ExecutionInvalidation, FinishGrantCeiling, ParentExecutionStamp};
pub use identity::*;
pub use measure::{DetailCount, DetailIndex, DetailOffset, ToolByteCount, ToolExitCode};
pub use notice::{NoticeCheckpoint, NoticeKey, NoticeRequest};

/// The single timeline feature remains unassigned; these values activate no peer behavior.
pub const TIMELINE_PARITY: Option<crate::ProtocolVersion> = None;
