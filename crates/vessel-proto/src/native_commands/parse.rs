//! Reference codex-app-server-agent.ts:226-242,4125-4141,5166-5189.
//! An advertised typed kind is required; parsing alone grants no capability.

use super::{
    GoalIntent, GoalObjective, GoalObjectiveError, NativeCommandIntent, NativeCommandKind,
};
use crate::text::ecmascript_space as space;

/// Invalid or mismatched intent must not fall back to sending a model prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NativeCommandParseError {
    /// The original text does not name the advertised root command.
    #[error("native command does not match its original text")]
    Mismatch,
    /// Invalid goal text is not silently truncated or reinterpreted.
    #[error(transparent)]
    Objective(#[from] GoalObjectiveError),
}

impl NativeCommandIntent {
    /// Validate original text against an explicitly advertised native operation.
    /// The caller still checks exact plain-text content, authority and capability.
    ///
    /// # Errors
    /// Refuses a mismatched command or an invalid/oversized objective.
    pub fn parse(kind: NativeCommandKind, text: &str) -> Result<Self, NativeCommandParseError> {
        let command = trim(text)
            .strip_prefix('/')
            .ok_or(NativeCommandParseError::Mismatch)?;
        let (name, args) = command.find(space).map_or((command, ""), |at| {
            (
                command.get(..at).unwrap_or_default(),
                trim(command.get(at..).unwrap_or_default()),
            )
        });
        if name != kind.root_name() {
            return Err(NativeCommandParseError::Mismatch);
        }
        Ok(match kind {
            NativeCommandKind::Compact => Self::Compact,
            NativeCommandKind::Goal => Self::Goal(if args.is_empty() {
                GoalIntent::Usage
            } else if args.eq_ignore_ascii_case("pause") {
                GoalIntent::Pause
            } else if args.eq_ignore_ascii_case("resume") {
                GoalIntent::Resume
            } else if args.eq_ignore_ascii_case("clear") {
                GoalIntent::Clear
            } else {
                GoalIntent::Set {
                    objective: GoalObjective::trimmed(args)?,
                }
            }),
        })
    }

    /// Receipt admission uses the same parser as the composer before reserving work.
    #[must_use]
    pub fn matches_text(&self, text: &str) -> bool {
        Self::parse(self.kind(), text).is_ok_and(|parsed| parsed == *self)
    }
}

pub(super) fn trim(text: &str) -> &str {
    crate::trim_ecmascript(text)
}
