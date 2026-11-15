//! Which agent a mark identifies. No toolkit, artwork bytes or network I/O.
//!
//! [Decision Log]
//! - Purpose: a label must not turn a user command into an official brand.
//! - Constraint: only the node's typed built-in identity is authoritative;
//!   registry icons have no safe content transport yet (ADR 0048).
//! - Choice: exhaustive built-in matches; added commands always use Generic.
//!   The app adapts this enum to the kit's reviewed artwork in one place.
//! - Cost: an added Copilot command remains generic, despite the bundled
//!   official Copilot artwork being available for attribution.
use vessel_proto::{HarnessKind, HarnessRef};

/// A mark's semantic identity, independent of how a renderer draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMark {
    /// Anthropic Claude Code.
    ClaudeCode,
    /// `OpenAI` Codex.
    Codex,
    /// Google Gemini CLI.
    Gemini,
    /// `OpenCode`.
    OpenCode,
    /// GitHub Copilot.
    Copilot,
    /// Unknown, user-added, or insufficiently identified.
    Generic,
}
impl AgentMark {
    /// All bundled brand identities, excluding the non-branded fallback.
    pub const BRANDS: [Self; 5] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Gemini,
        Self::OpenCode,
        Self::Copilot,
    ];
    /// Match the node's identity exactly. Display names and suffixes are
    /// deliberately never inspected; doing so would let arbitrary commands
    /// impersonate an official agent.
    #[must_use]
    pub const fn of(agent: &HarnessRef) -> Self {
        match agent {
            HarnessRef::Known(kind) => match kind {
                HarnessKind::ClaudeCode => Self::ClaudeCode,
                HarnessKind::Codex => Self::Codex,
                HarnessKind::Gemini => Self::Gemini,
                HarnessKind::OpenCode => Self::OpenCode,
            },
            HarnessRef::Added(_) => Self::Generic,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_builtin_gets_its_own_mark() {
        assert_eq!(HarnessKind::ALL.len(), 4);
        for (kind, mark) in [
            (HarnessKind::ClaudeCode, AgentMark::ClaudeCode),
            (HarnessKind::Codex, AgentMark::Codex),
            (HarnessKind::Gemini, AgentMark::Gemini),
            (HarnessKind::OpenCode, AgentMark::OpenCode),
        ] {
            assert_eq!(AgentMark::of(&HarnessRef::Known(kind)), mark);
        }
    }
    #[test]
    fn unknown_ids_and_similar_added_names_do_not_impersonate_brands() {
        // Paseo's custom-claude-profile fixture, then case and suffix traps
        // and the actual fake-fleet identities. No display label is consulted.
        for id in [
            "unknown",
            "custom-claude-profile",
            "claude-code-custom",
            "Claude-Code-custom",
            "codex-extra",
            "CODEX-extra",
            "gemini-extra",
            "Gemini-extra",
            "opencode-extra",
            "OpenCode-extra",
            "copilot",
            "Copilot",
            "copilot-extra",
            "fake-goose",
            "ghost-agent",
            "check-failed",
            "needs-sign-in",
            "example-agent",
            "mark-custom-en",
            "mark-custom-ko",
        ] {
            let added = HarnessRef::Added(id.parse().unwrap());
            assert_eq!(AgentMark::of(&added), AgentMark::Generic, "{id}");
        }
    }
}
