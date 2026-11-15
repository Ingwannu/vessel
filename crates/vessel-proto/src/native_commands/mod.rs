//! Closed native command intents shared by composer, receipt actor and transport.
//! These values perform no I/O and activate no RPC or provider feature.
//!
//! [Decision Log]
//! - Purpose: a native command must not fabricate a model turn or busy state.
//! - Constraint: only the provider transport can prove runtime/launch capability.
//! - Choice: the composer produces a validated intent; the actor fingerprints it
//!   with original content and the existing client id; transport returns a closed
//!   status. No output repeats an objective or embeds a provider RPC diagnostic.
//! - Why: one keyed acceptance path can distinguish commands from prompt delivery
//!   while preserving the actual foreground turn. The shared parser lets the
//!   actor validate intent against original text before reserving any mutation.
//! - Cost: objective text has an explicit64KiB bound, unlike the reference's
//!   unbounded argument. These are dormant values until the joint wire review.

mod decode;
mod objective;
mod parse;

pub use objective::{GoalObjective, GoalObjectiveError, MAX_GOAL_OBJECTIVE_BYTES};
pub use parse::NativeCommandParseError;

use serde::{Deserialize, Serialize};

/// Capability metadata, supplied only after the actual transport is proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeCommandKind {
    /// Compact context without creating a user/model turn.
    Compact,
    /// Manage a goal through the provider's real goal runtime.
    Goal,
}
impl NativeCommandKind {
    /// Canonical root name; the composer does not infer execution from a label.
    #[must_use]
    pub const fn root_name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Goal => "goal",
        }
    }
}

/// A parsed operation, independent of its original submitted text or message id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    content = "action",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NativeCommandIntent {
    /// Request context compaction; reference arguments are ignored.
    Compact,
    /// One of the provider's goal operations.
    Goal(GoalIntent),
}
impl NativeCommandIntent {
    /// The actor and transport compare against the same advertised capability.
    #[must_use]
    pub const fn kind(&self) -> NativeCommandKind {
        match self {
            Self::Compact => NativeCommandKind::Compact,
            Self::Goal(_) => NativeCommandKind::Goal,
        }
    }
}

/// Goal subcommands follow Paseo's exact whole-argument interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalIntent {
    /// No argument: show localized usage, without a provider mutation.
    Usage,
    /// A nonempty objective, with no silent truncation.
    Set {
        /// Validated original objective after argument trimming.
        objective: GoalObjective,
    },
    /// Pause the provider goal.
    Pause,
    /// Resume the provider goal.
    Resume,
    /// Clear the provider goal.
    Clear,
}

/// Localizable successful acceptance; neither kind nor objective is duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeCommandStatus {
    /// Compaction was requested, not necessarily completed.
    CompactStarted,
    /// The objective was accepted.
    GoalSet,
    /// The goal was paused.
    GoalPaused,
    /// The goal was resumed.
    GoalResumed,
    /// The goal was cleared.
    GoalCleared,
    /// Display the static usage message.
    GoalUsage,
}
impl NativeCommandStatus {
    /// Derivation prevents an impossible status/kind pair in a receipt.
    #[must_use]
    pub const fn kind(self) -> NativeCommandKind {
        match self {
            Self::CompactStarted => NativeCommandKind::Compact,
            Self::GoalSet
            | Self::GoalPaused
            | Self::GoalResumed
            | Self::GoalCleared
            | Self::GoalUsage => NativeCommandKind::Goal,
        }
    }
}

#[cfg(test)]
mod tests;

impl NativeCommandIntent {
    /// Native commands use exactly one original text block, never attachments,
    /// hidden context or a provider-built prompt substituted by a caller.
    #[must_use]
    pub fn matches_content(&self, content: &[crate::ContentBlock]) -> bool {
        matches!(content, [crate::ContentBlock::Text { text }] if self.matches_text(text))
    }
}
