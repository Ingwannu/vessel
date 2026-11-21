//! Which mark a profile's rows draw (ADR 0117; Paseo's
//! `getTerminalProfileIcon`, `protocol/src/terminal-profiles.ts` 108-132,
//! and `TerminalProfileIcon`, `app/src/components/terminal-profile-icon.tsx`
//! 26-37).
//!
//! [Decision Log]
//! - Purpose: a profile row shows the agent it runs, as Paseo's New tab,
//!   menus and Settings do.
//! - Reference: the profile's `icon` when set, else a guess from the
//!   command's base name among the known names (exact, plus the alias
//!   `cursor-agent` → `cursor`); no name draws the terminal glyph, a name
//!   outside the catalogue the generic agent mark.
//! - Choice: the same rule here, so the native app and the page agree; each
//!   client maps a known name to the artwork it has. The node's admin sets
//!   both the label and the program, so a guessed mark tells what runs; a
//!   harness's mark keeps its exact-identity rule (coordinator, 2026-11-21).
//! - Cost: the catalogue is a copy of Paseo's names; a new provider is added
//!   here when Vessel draws it.

use super::Profile;
use super::adopt::base_name;

/// Paseo's `KNOWN_PROVIDER_ICON_NAMES` (`provider-icon-names.ts` 1-60):
/// its built-in providers, its ACP registry's, and `agy`.
pub const KNOWN_ICONS: [&str; 48] = [
    "antigravity",
    "claude",
    "codex",
    "copilot",
    "kiro",
    "minimax",
    "omp",
    "opencode",
    "pi",
    "agoragentic-acp",
    "amp-acp",
    "auggie",
    "autohand",
    "cline",
    "codebuddy-code",
    "codewhale",
    "cortex-code",
    "corust-agent",
    "crow-cli",
    "cursor",
    "deepagents",
    "devin",
    "dimcode",
    "dirac",
    "factory-droid",
    "fast-agent",
    "gemini",
    "gjc",
    "glm-acp-agent",
    "goose",
    "grok",
    "hermes",
    "hoplite",
    "junie",
    "kilo",
    "kimi",
    "minimax-code",
    "minion-code",
    "mistral-vibe",
    "nova",
    "poolside",
    "qoder",
    "qwen-code",
    "sigit",
    "stakpak",
    "traecli",
    "vtcode",
    "agy",
];

/// Programs not named after their mark (`COMMAND_ICON_ALIASES`, 110).
const ALIASES: [(&str, &str); 1] = [("cursor-agent", "cursor")];

/// What a profile's row draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileIcon {
    /// No name, set or guessed: the terminal glyph (`SquareTerminal`).
    Terminal,
    /// A catalogue name; a client without its artwork draws the terminal
    /// glyph.
    Known(&'static str),
    /// A set name outside the catalogue: the generic agent mark (Paseo's
    /// Bot fallback).
    Unknown,
}

fn known(name: &str) -> Option<&'static str> {
    KNOWN_ICONS.iter().copied().find(|known| *known == name)
}

/// The name a command's base name guesses (Paseo's
/// `guessTerminalProfileIcon`, 126-128).
#[must_use]
pub fn guess(command: &str) -> Option<&'static str> {
    let base = base_name(command);
    known(&base).or_else(|| {
        ALIASES
            .iter()
            .find(|(alias, _)| *alias == base)
            .map(|(_, name)| *name)
    })
}

impl Profile {
    /// The row's mark: `icon`, else the command's guess.
    #[must_use]
    pub fn mark(&self) -> ProfileIcon {
        match self.icon() {
            Some(icon) => known(icon.as_str()).map_or(ProfileIcon::Unknown, ProfileIcon::Known),
            None => {
                guess(self.command().as_str()).map_or(ProfileIcon::Terminal, ProfileIcon::Known)
            }
        }
    }
}
