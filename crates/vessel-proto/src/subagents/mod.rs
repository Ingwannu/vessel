//! Managed delegation and provider-owned tasks are different identities (ADR 0090).
//! Creator is immutable audit data; current parentage applies a durable detach.

mod parentage;
mod values;

pub use parentage::{
    DetachDisposition, DetachReceipt, DetachSession, DetachedSession, InvalidParentage, Parentage,
};
pub use values::{ProviderChildId, ProviderText, SubagentValueError};

/// The shared gate for detach, provider-child observations, list and timeline.
/// Integration assigns it only after native, CLI and delegated callers consume
/// current parentage without treating a provider task as a managed session.
pub const SUBAGENT_RELATIONS: Option<crate::ProtocolVersion> = None;
