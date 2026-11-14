//! [`tag_modes`]: a known agent's permission modes marked with how much
//! they let it do unasked, as Paseo marks them (ADR 0058).
//!
//! [Decision Log]
//! - Purpose: the owner asked for every agent set up as Paseo sets it up,
//!   and the coordinator for a mode that runs without asking to be named
//!   for what it is wherever it is offered. Paseo carries a colour tier and
//!   an "unattended" mark per mode in its provider manifest; ACP reports a
//!   mode's id, name and description only.
//! - Constraint: harness-specific knowledge lives in `vessel-harness`, one
//!   module per agent (`AGENTS.md`); the options come from the agent and
//!   must pass through truthfully, so nothing is added, renamed or hidden.
//! - Choice: each descriptor lists the mode ids Paseo's manifest (or its
//!   provider plugin) tiers, with the tier; this fills `risk` and
//!   `unattended` on the matching values of the agent's mode option and
//!   leaves everything else as the agent said. A value a direct driver has
//!   already tagged keeps its own tag.
//! - Cost: an id an agent renames loses its tag until the descriptor
//!   follows; an agent whose ids are not known yet (Grok, Amp, Cursor:
//!   only a real run shows them) has none, and a client shows its modes
//!   plain.

use vessel_proto::{HarnessKind, OptionCategory, OptionKind, Risk, SessionOption};

use crate::harnesses::descriptor;

/// One mode a harness offers, as Paseo tiers it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModeTag {
    /// The mode's id, as the agent reports it.
    pub(crate) id: &'static str,
    /// Its tier.
    pub(crate) risk: Risk,
    /// Whether it runs every action without asking.
    pub(crate) unattended: bool,
}

impl ModeTag {
    /// A mode that asks before what its tier says it may not do alone.
    pub(crate) const fn asks(id: &'static str, risk: Risk) -> Self {
        Self {
            id,
            risk,
            unattended: false,
        }
    }

    /// A mode that runs every action without asking.
    pub(crate) const fn unattended(id: &'static str) -> Self {
        Self {
            id,
            risk: Risk::Dangerous,
            unattended: true,
        }
    }
}

/// `options` with `harness`'s known modes tagged: on its mode option, each
/// value whose id the descriptor lists and that carries no tier yet gets
/// the descriptor's. Nothing else changes.
pub fn tag_modes(harness: HarnessKind, options: &mut [SessionOption]) {
    let known = descriptor(harness).modes;
    if known.is_empty() {
        return;
    }
    let modes = options
        .iter_mut()
        .filter(|option| option.category == OptionCategory::Mode);
    for option in modes {
        let values = match &mut option.kind {
            OptionKind::Select { values, .. } => values,
            OptionKind::Toggle { .. } | OptionKind::Unknown => continue,
        };
        for value in values.iter_mut().filter(|value| value.risk.is_none()) {
            if let Some(tag) = known.iter().find(|tag| value.id.as_str() == tag.id) {
                value.risk = Some(tag.risk);
                value.unattended = tag.unattended;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::{OptionId, OptionValue, OptionValueId};

    use super::*;

    fn mode(ids: &[&str]) -> SessionOption {
        SessionOption {
            id: OptionId::new("mode"),
            name: "Mode".to_owned(),
            description: None,
            category: OptionCategory::Mode,
            kind: OptionKind::Select {
                current: OptionValueId::new("default"),
                values: ids
                    .iter()
                    .map(|id| OptionValue {
                        id: OptionValueId::new(*id),
                        name: (*id).to_owned(),
                        description: None,
                        group: None,
                        risk: None,
                        unattended: false,
                    })
                    .collect(),
            },
        }
    }

    fn tags(option: &SessionOption) -> Vec<(String, Option<Risk>, bool)> {
        match &option.kind {
            OptionKind::Select { values, .. } => values
                .iter()
                .map(|value| (value.id.as_str().to_owned(), value.risk, value.unattended))
                .collect(),
            OptionKind::Toggle { .. } | OptionKind::Unknown => Vec::new(),
        }
    }

    #[test]
    fn claude_codes_modes_are_tiered_as_paseo_tiers_them() {
        // The ids `claude-agent-acp` 0.100.0 reports, and one it may add.
        let mut options = [mode(&[
            "default",
            "acceptEdits",
            "plan",
            "auto",
            "bypassPermissions",
            "later",
        ])];
        tag_modes(HarnessKind::ClaudeCode, &mut options);
        assert_eq!(
            tags(&options[0]),
            [
                ("default".to_owned(), Some(Risk::Safe), false),
                ("acceptEdits".to_owned(), Some(Risk::Moderate), false),
                ("plan".to_owned(), Some(Risk::Planning), false),
                ("auto".to_owned(), Some(Risk::Moderate), false),
                ("bypassPermissions".to_owned(), Some(Risk::Dangerous), true),
                ("later".to_owned(), None, false),
            ]
        );
    }

    #[test]
    fn only_the_mode_option_is_tagged_and_a_drivers_own_tag_stays() {
        let mut model = mode(&["full-access"]);
        model.category = OptionCategory::Model;
        let mut options = [model, mode(&["full-access", "auto"])];
        if let OptionKind::Select { values, .. } = &mut options[1].kind {
            values[1].risk = Some(Risk::Safe);
        }
        tag_modes(HarnessKind::Codex, &mut options);
        assert_eq!(tags(&options[0]), [("full-access".to_owned(), None, false)]);
        assert_eq!(
            tags(&options[1]),
            [
                ("full-access".to_owned(), Some(Risk::Dangerous), true),
                ("auto".to_owned(), Some(Risk::Safe), false),
            ]
        );
    }

    #[test]
    fn an_agent_whose_ids_are_not_known_is_left_plain() {
        let mut options = [mode(&["default", "bypassPermissions"])];
        tag_modes(HarnessKind::Grok, &mut options);
        assert_eq!(
            tags(&options[0]),
            [
                ("default".to_owned(), None, false),
                ("bypassPermissions".to_owned(), None, false),
            ]
        );
    }

    /// Every descriptor's unattended mode is dangerous, and no id twice.
    #[test]
    fn descriptors_are_consistent() {
        for kind in HarnessKind::ALL {
            let modes = descriptor(kind).modes;
            for (index, tag) in modes.iter().enumerate() {
                assert!(
                    !tag.unattended || tag.risk == Risk::Dangerous,
                    "{kind} {}",
                    tag.id
                );
                assert!(
                    modes[index + 1..].iter().all(|other| other.id != tag.id),
                    "{kind} {}",
                    tag.id
                );
            }
        }
    }
}
