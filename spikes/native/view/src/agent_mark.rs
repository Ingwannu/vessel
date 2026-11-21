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
//! - Terminal profiles (ADR 0117) follow Paseo instead: an admin-saved
//!   profile's `icon`, else its command's guess (`AgentMark::of_profile`).
use vessel_proto::terminal_profiles::{Profile, ProfileIcon};
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

    /// A terminal profile's mark (ADR 0117; Paseo's `getTerminalProfileIcon`
    /// and `TerminalProfileIcon`): the profile's `icon`, else a guess from
    /// its command's base name (`Profile::mark`). A name Vessel has artwork
    /// for gets its brand; a name outside the catalogue the generic mark
    /// (Paseo's Bot); no name, or one without artwork here (Pi, Cursor),
    /// `None`: the caller draws the terminal glyph. The node's admin sets
    /// both the label and the program, so the guess says what runs (the
    /// coordinator's ruling of 2026-11-21); harness marks keep the exact
    /// identity rule above.
    #[must_use]
    pub fn of_profile(profile: &Profile) -> Option<Self> {
        match profile.mark() {
            ProfileIcon::Known(name) => match name {
                "claude" => Some(Self::ClaudeCode),
                "codex" => Some(Self::Codex),
                "opencode" => Some(Self::OpenCode),
                "gemini" => Some(Self::Gemini),
                "copilot" => Some(Self::Copilot),
                _ => None,
            },
            ProfileIcon::Unknown => Some(Self::Generic),
            ProfileIcon::Terminal => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::terminal_profiles::Profiles;
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
    fn a_configured_profile_takes_its_icon_else_its_commands_guess() {
        use vessel_proto::terminal_profiles::{Argument, Icon, Id, Name, Program};
        let profile = |command: &str, icon: Option<&str>| {
            Profile::new(
                Id::parse("profile_a").unwrap(),
                Name::parse("Mine").unwrap(),
                Program::parse(command).unwrap(),
                vec![Argument::parse("{{{prompt}}}").unwrap()],
                icon.map(|icon| Icon::parse(icon).unwrap()),
            )
            .unwrap()
        };
        // The coordinator's ruling (2026-11-21): Paseo's `icon ?? guess`.
        for (command, icon, mark) in [
            (
                "/usr/local/bin/anything",
                Some("claude"),
                Some(AgentMark::ClaudeCode),
            ),
            ("codex", None, Some(AgentMark::Codex)),
            (r"C:\tools\opencode.cmd", None, Some(AgentMark::OpenCode)),
            ("gemini", None, Some(AgentMark::Gemini)),
            ("copilot", None, Some(AgentMark::Copilot)),
            ("cursor-agent", None, None),
            ("zsh", None, None),
            ("zsh", Some("not-a-provider"), Some(AgentMark::Generic)),
        ] {
            assert_eq!(
                AgentMark::of_profile(&profile(command, icon)),
                mark,
                "{command}"
            );
        }
    }
}
