//! The worker retains its task cause privately; panic payloads are not diagnostics.
use std::{error::Error, fmt};

pub(crate) struct SkillTaskFailure {
    cause: tokio::task::JoinError,
}
#[derive(Debug)]
enum Kind {
    Cancelled,
    Panicked,
}
impl SkillTaskFailure {
    fn kind(&self) -> Kind {
        if self.cause.is_cancelled() {
            Kind::Cancelled
        } else {
            Kind::Panicked
        }
    }
}
impl From<tokio::task::JoinError> for SkillTaskFailure {
    fn from(cause: tokio::task::JoinError) -> Self {
        Self { cause }
    }
}
impl fmt::Debug for SkillTaskFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_tuple("SkillTaskFailure")
            .field(&self.kind())
            .finish()
    }
}
impl fmt::Display for SkillTaskFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            Kind::Cancelled => out.write_str("skill worker task was cancelled"),
            Kind::Panicked => out.write_str("skill worker task panicked"),
        }
    }
}
impl Error for SkillTaskFailure {}
