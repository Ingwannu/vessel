//! The options at the top of the model's list, in the app's own words: an
//! agent's option is shown there only when the app can name it in plain
//! words in both languages and say in one line what it does (the
//! coordinator, 0.0.0.0.8: "Agent" and "Collaboration mode" in a Korean
//! list said nothing to the owner). An option the app has no words for
//! gets no row.
//!
//! [Decision Log]
//! - Purpose: the effort control folds in levels, the top step and fast;
//!   what remains of an agent's options sits at the top of the model's
//!   list, and must read as the owner's language does.
//! - Choice: a typed table by agent kind and option id, as the effort
//!   control's (`effort::ROLES`): Claude Code's `agent` is the answering
//!   agent (a choice of the machine's agent definitions), Codex's
//!   `collaboration_mode` its plan mode (a switch: `plan` on, `default`
//!   off). Ids read off the preview node, 2026-11-13.

use vessel_proto::{HarnessKind, OptionId};

/// An option the app names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extra {
    /// Which of the machine's agent definitions answers (Claude Code's
    /// `agent`): a choice; its "default" value is the agent itself.
    AnsweringAgent,
    /// Plan before changing anything (Codex's `collaboration_mode`): a
    /// switch over two values.
    PlanMode,
}

impl Extra {
    /// The value that turns a switch on and the one that turns it off.
    #[must_use]
    pub const fn switch_values(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::AnsweringAgent => None,
            Self::PlanMode => Some(("plan", "default")),
        }
    }
}

/// The value id every agent so far uses for its own default.
pub const DEFAULT_VALUE: &str = "default";

/// The options the app names, by agent kind and option id.
const KNOWN: &[(HarnessKind, &str, Extra)] = &[
    (HarnessKind::ClaudeCode, "agent", Extra::AnsweringAgent),
    (HarnessKind::Codex, "collaboration_mode", Extra::PlanMode),
];

/// What the app calls option `id` of an agent of `kind`; `None`: it has
/// no words for it (no row).
#[must_use]
pub fn extra_of(kind: Option<HarnessKind>, id: &OptionId) -> Option<Extra> {
    let kind = kind?;
    KNOWN
        .iter()
        .find(|(known, option, _)| *known == kind && *option == id.as_str())
        .map(|(_, _, extra)| *extra)
}

#[cfg(test)]
mod tests {
    use vessel_proto::{HarnessKind, OptionId};

    use super::{Extra, extra_of};

    #[test]
    fn the_app_names_claude_codes_agent_and_codexs_plan_mode_only() {
        let id = OptionId::new;
        assert_eq!(
            extra_of(Some(HarnessKind::ClaudeCode), &id("agent")),
            Some(Extra::AnsweringAgent)
        );
        assert_eq!(
            extra_of(Some(HarnessKind::Codex), &id("collaboration_mode")),
            Some(Extra::PlanMode)
        );
        assert_eq!(extra_of(Some(HarnessKind::Codex), &id("agent")), None);
        assert_eq!(
            extra_of(Some(HarnessKind::ClaudeCode), &id("unknown")),
            None
        );
        assert_eq!(extra_of(None, &id("agent")), None);
        assert_eq!(Extra::PlanMode.switch_values(), Some(("plan", "default")));
    }
}
