//! Tracked standard-native resolution lifetime (ADR0104).
//!
//! A scope closes admission synchronously and retains actual blocking joins
//! after callers disappear. Only drain can acknowledge normal completion.
//! This module owns no transport, credentials, DNS cache or consumer policy.

mod failure;
mod scope;
mod values;
mod worker;

pub use failure::{Failure, FailureKind};
pub use scope::{NormalDrain, ResolutionBudget, ResolutionHandle, ResolutionScope, ScopeState};
pub use values::{Addresses, HostName, ResolutionPort, ResolutionQuota};

/// One lifecycle root's maximum simultaneous native work, across generations.
pub const MAX_NATIVE_RESOLUTIONS: usize = 64;
/// Application input bound, in UTF-8 bytes, before a worker copy is made.
pub const MAX_HOST_BYTES: usize = 255;
/// Raw resolver records examined; duplicates also count against this bound.
pub const MAX_RESOLVED_ADDRESSES: usize = 64;
