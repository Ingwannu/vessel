//! Ordinary independent sessions sharing one submitted input (ADR 0046).
//!
//! The sole job owns continuations and admitted mutations. Snapshots are
//! passive reports: neither a lost reply nor navigation can restart one.
mod copy;
mod effect;
mod endpoint;
mod input;
mod job;
mod observe;
mod prepare;
mod state;
mod validate;
mod worker;

pub use effect::MutationError;
pub use input::{ComparisonInput, ComparisonPlan, InputError, Selection, SourceAttachments};
pub use job::{ComparisonJob, JobError};
pub use state::{
    CopyProgress, Failure, FailureKind, JobStatus, Phase, ResourceTrail, SelectionIndex, Snapshot,
    TargetReport, TargetStatus,
};
