//! Legacy limits adapters and old-peer native pins share their registered source identities.
use super::{BuiltinUsageSource, InvalidUsageIdentity, UsageSourceId};
use crate::{HarnessKind, HarnessRef};
/// Configured saved-login projections, distinct from full vendor-account discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedUsageSource {
    /// Claude Code's configured saved sign-in.
    Claude,
    /// Codex's configured saved sign-in.
    Codex,
    /// `OpenCode`'s configured saved `OpenAI` sign-in.
    OpenCode,
}
impl SavedUsageSource {
    /// Existing registered namespace, shared by old-peer view projections and node reports.
    /// # Errors
    /// A programmer supplied an invalid registered constant; never a user label fallback.
    pub fn id(self) -> Result<UsageSourceId, InvalidUsageIdentity> {
        match self {
            Self::Claude => "saved-claude",
            Self::Codex => "saved-codex",
            Self::OpenCode => "saved-opencode",
        }
        .parse()
    }
    /// Bounded display copy; it never identifies or resolves an account.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude saved accounts",
            Self::Codex => "Codex saved accounts",
            Self::OpenCode => "OpenCode saved OpenAI accounts",
        }
    }
    /// Existing client mark; no provider is inferred from a presentation label.
    #[must_use]
    pub const fn builtin(self) -> BuiltinUsageSource {
        match self {
            Self::Claude => BuiltinUsageSource::Claude,
            Self::Codex | Self::OpenCode => BuiltinUsageSource::Codex,
        }
    }
    /// Only the three actual legacy adapters have saved-source identities.
    #[must_use]
    pub const fn for_harness(harness: &HarnessRef) -> Option<Self> {
        match harness.known() {
            Some(HarnessKind::ClaudeCode) => Some(Self::Claude),
            Some(HarnessKind::Codex) => Some(Self::Codex),
            Some(HarnessKind::OpenCode) => Some(Self::OpenCode),
            Some(HarnessKind::Gemini) | None => None,
        }
    }
}
