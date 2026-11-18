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
use vessel_proto::terminal_profiles::{Profile, Profiles};
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

    /// A terminal profile's mark (the New tab panel's profile rows, TRACKER
    /// 336). Only a profile exactly as Vessel ships it carries its brand:
    /// those are the app's own constants, not a node's words. A configured
    /// profile, even one that reuses a shipped id or icon text, has none, so
    /// it cannot borrow a brand (the rule above). `None` means no mark: the
    /// caller draws Paseo's no-icon form (the terminal glyph); Pi has no
    /// bundled artwork, so it takes that form too.
    #[must_use]
    pub fn of_profile(profile: &Profile) -> Option<Self> {
        let shipped = Profiles::shipped().ok()?;
        if shipped.get(profile.id()) != Some(profile) {
            return None;
        }
        match profile.id().as_str() {
            "claude" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "opencode" => Some(Self::OpenCode),
            _ => None,
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
    #[test]
    fn shipped_profiles_carry_their_brand_and_pi_none() {
        let shipped = Profiles::shipped().unwrap();
        let marks: Vec<Option<AgentMark>> = shipped
            .entries()
            .iter()
            .map(AgentMark::of_profile)
            .collect();
        assert_eq!(
            marks,
            [
                Some(AgentMark::ClaudeCode),
                Some(AgentMark::Codex),
                Some(AgentMark::OpenCode),
                None,
            ]
        );
    }
    #[test]
    fn a_configured_profile_with_a_shipped_id_or_icon_has_no_brand() {
        use vessel_proto::terminal_profiles::{Argument, Icon, Id, Name, Program};
        let claude_like = Profile::new(
            Id::parse("claude").unwrap(),
            Name::parse("Claude Code").unwrap(),
            Program::parse("/usr/local/bin/anything").unwrap(),
            vec![Argument::parse("{{{prompt}}}").unwrap()],
            Some(Icon::parse("claude").unwrap()),
        )
        .unwrap();
        assert_eq!(AgentMark::of_profile(&claude_like), None);
        let iconed = Profile::new(
            Id::parse("mine").unwrap(),
            Name::parse("Mine").unwrap(),
            Program::parse("codex").unwrap(),
            Vec::new(),
            Some(Icon::parse("codex").unwrap()),
        )
        .unwrap();
        assert_eq!(AgentMark::of_profile(&iconed), None);
    }
}
