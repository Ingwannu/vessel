//! Standalone canonical tool bodies for borrowed display; no event/RPC attachment.
//! Provider parsing, execution, authority and session ownership stay outside proto.
mod detail;
mod execution;
mod identity;
mod measure;

pub use detail::*;
pub use execution::{ExecutionInvalidation, FinishGrantCeiling, ParentExecutionStamp};
pub use identity::*;
pub use measure::{DetailCount, DetailIndex, DetailOffset, ToolByteCount, ToolExitCode};

/// The single timeline feature remains unassigned; these values activate no peer behavior.
pub const TIMELINE_PARITY: Option<crate::ProtocolVersion> = None;
