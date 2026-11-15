//! Static source identities are typed before leaving this adapter boundary.
use vessel_proto::{
    HarnessKind, HarnessRef,
    usage_reports::{BuiltinUsageSource, InvalidUsageIdentity, UsageSourceId},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SavedSource {
    Claude,
    Codex,
    OpenCode,
}
impl SavedSource {
    pub(crate) fn id(self) -> Result<UsageSourceId, InvalidUsageIdentity> {
        match self {
            Self::Claude => "saved-claude",
            Self::Codex => "saved-codex",
            Self::OpenCode => "saved-opencode",
        }
        .parse()
    }
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude saved accounts",
            Self::Codex => "Codex saved accounts",
            Self::OpenCode => "OpenCode saved OpenAI accounts",
        }
    }
    pub(crate) const fn builtin(self) -> BuiltinUsageSource {
        match self {
            Self::Claude => BuiltinUsageSource::Claude,
            Self::Codex | Self::OpenCode => BuiltinUsageSource::Codex,
        }
    }
}
pub(crate) const fn source(harness: &HarnessRef) -> Option<SavedSource> {
    match harness.known() {
        Some(HarnessKind::ClaudeCode) => Some(SavedSource::Claude),
        Some(HarnessKind::Codex) => Some(SavedSource::Codex),
        Some(HarnessKind::OpenCode) => Some(SavedSource::OpenCode),
        Some(HarnessKind::Gemini) | None => None,
    }
}
