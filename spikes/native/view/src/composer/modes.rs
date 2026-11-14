//! Which mark a permissions mode is drawn with, as Paseo draws them: its
//! provider manifest names an icon per mode
//! (`packages/protocol/src/provider-manifest.ts`, `CLAUDE_MODES`,
//! `CODEX_MODES`, `OPENCODE_MODES`), a list row shows that icon beside the
//! mode's name (`composer/agent-controls/mode-control.tsx`,
//! `ModeComboboxOption` with `getAgentModeOptionIcon`: none when the mode
//! has no icon), and the closed control shows the chosen mode's icon, the
//! agent's when it has none (`getAgentModeIcon`, falling back to `Bot`).
//!
//! Vessel's modes are the agent's own ids, as its adapter sends them over
//! ACP; the ids Paseo's manifest names are the same strings where the
//! adapter has the mode (Claude Code's `default`, `acceptEdits`, `plan`,
//! `bypassPermissions`; Codex's `auto`, `full-access`; `OpenCode`'s `build`,
//! `plan`). A mode Paseo does not list (Claude Code's `dontAsk`, an added
//! agent's) has no icon in a row, as in Paseo.

use vessel_proto::{HarnessKind, OptionValueId};

/// A mode's mark (Paseo's `MODE_ICONS`, the shield family).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeMark {
    /// Asks as usual (`Shield`).
    Shield,
    /// Accepts edits (`ShieldPlus`).
    ShieldPlus,
    /// Decides for itself (`ShieldCheck`).
    ShieldCheck,
    /// Asks nothing (`ShieldOff`).
    ShieldOff,
    /// Plans first (`ShieldEllipsis`).
    ShieldEllipsis,
    /// Asks before writing (`ShieldAlert`).
    ShieldAlert,
}

/// What the closed permissions control shows: the mode's mark, or the
/// agent's (Paseo's `Bot`) for a mode with none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillMark {
    /// The chosen mode's mark.
    Mode(ModeMark),
    /// No mark for the mode.
    Agent,
}

/// The mark of mode `id` of an agent of `kind` (`None` for an added agent),
/// or none (Paseo's `getAgentModeOptionIcon`).
#[must_use]
pub fn mark_of(kind: Option<HarnessKind>, id: &OptionValueId) -> Option<ModeMark> {
    let kind = kind?;
    let id = id.as_str();
    match kind {
        HarnessKind::ClaudeCode => match id {
            "plan" => Some(ModeMark::ShieldEllipsis),
            "default" => Some(ModeMark::Shield),
            "acceptEdits" => Some(ModeMark::ShieldPlus),
            "auto" => Some(ModeMark::ShieldCheck),
            "bypassPermissions" => Some(ModeMark::ShieldOff),
            _unlisted => None,
        },
        HarnessKind::Codex => match id {
            "auto" => Some(ModeMark::Shield),
            "auto-review" => Some(ModeMark::ShieldCheck),
            "full-access" => Some(ModeMark::ShieldOff),
            _unlisted => None,
        },
        HarnessKind::OpenCode => match id {
            "build" => Some(ModeMark::Shield),
            "plan" => Some(ModeMark::ShieldEllipsis),
            _unlisted => None,
        },
        // Paseo's manifest has no Gemini modes.
        HarnessKind::Gemini => None,
    }
}

/// The closed control's mark for mode `id` (Paseo's `getAgentModeIcon`).
#[must_use]
pub fn pill_mark_of(kind: Option<HarnessKind>, id: &OptionValueId) -> PillMark {
    mark_of(kind, id).map_or(PillMark::Agent, PillMark::Mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> OptionValueId {
        OptionValueId::new(text)
    }

    #[test]
    fn claude_codes_modes_take_paseos_marks_and_an_unlisted_one_none() {
        let claude = Some(HarnessKind::ClaudeCode);
        for (mode, mark) in [
            ("plan", Some(ModeMark::ShieldEllipsis)),
            ("default", Some(ModeMark::Shield)),
            ("acceptEdits", Some(ModeMark::ShieldPlus)),
            ("auto", Some(ModeMark::ShieldCheck)),
            ("bypassPermissions", Some(ModeMark::ShieldOff)),
            ("dontAsk", None),
        ] {
            assert_eq!(mark_of(claude, &id(mode)), mark, "{mode}");
        }
    }

    #[test]
    fn codex_and_opencode_modes_take_their_own_marks() {
        let codex = Some(HarnessKind::Codex);
        assert_eq!(mark_of(codex, &id("auto")), Some(ModeMark::Shield));
        assert_eq!(
            mark_of(codex, &id("auto-review")),
            Some(ModeMark::ShieldCheck)
        );
        assert_eq!(
            mark_of(codex, &id("full-access")),
            Some(ModeMark::ShieldOff)
        );
        assert_eq!(
            mark_of(codex, &id("plan")),
            None,
            "Claude's id, not Codex's"
        );
        let opencode = Some(HarnessKind::OpenCode);
        assert_eq!(mark_of(opencode, &id("build")), Some(ModeMark::Shield));
        assert_eq!(
            mark_of(opencode, &id("plan")),
            Some(ModeMark::ShieldEllipsis)
        );
        assert_eq!(mark_of(Some(HarnessKind::Gemini), &id("default")), None);
        assert_eq!(mark_of(None, &id("default")), None, "an added agent");
    }

    #[test]
    fn the_closed_control_falls_back_to_the_agents_mark() {
        let claude = Some(HarnessKind::ClaudeCode);
        assert_eq!(
            pill_mark_of(claude, &id("plan")),
            PillMark::Mode(ModeMark::ShieldEllipsis)
        );
        assert_eq!(pill_mark_of(claude, &id("dontAsk")), PillMark::Agent);
        assert_eq!(pill_mark_of(None, &id("plan")), PillMark::Agent);
    }
}
