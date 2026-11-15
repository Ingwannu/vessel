//! Create-mode inputs preserve unknown versus explicitly empty catalogues.
//! Mode semantics belong to the harness, not to the wire decoder.

use super::{InvalidProvider, ModeId, ProviderId};
use crate::agents::AgentLabel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Built-in implementation family; custom identities remain open `ProviderIds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Claude's native permission policy.
    Claude,
    /// Codex app-server policy.
    Codex,
    /// Copilot ACP with its synthetic allow-all mode.
    Copilot,
    /// `OpenCode` with dynamic agent names and an auto-accept feature.
    OpenCode,
    /// Pi RPC has no permission modes.
    Pi,
    /// Oh My Pi's approval-mode policy.
    Omp,
    /// Configured ACP providers may discover their own modes.
    Acp,
    /// Trusted plugin providers negotiate policy separately.
    Plugin,
}

/// Approval semantics must not be inferred from a colour or mode name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalFlow {
    /// Provider may prompt a person.
    Interactive,
    /// Provider's most permissive no-prompt mode.
    Unattended,
}

/// Explicit feature choice; absence differs from a user selecting off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoAccept {
    /// Permission prompts stay with their normal recipient.
    Off,
    /// Provider's `auto_accept` feature is enabled.
    On,
}

/// Visual meaning, independent of actual approval enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeRisk {
    /// Always ask.
    Safe,
    /// Normal workspace permission flow.
    Moderate,
    /// Unattended permissions.
    Dangerous,
    /// Planning mode.
    Planning,
}

/// A discovered or manifest mode, kept separate from supported driver state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    /// Native identifier sent only to a compatible driver.
    pub id: ModeId,
    /// Human label from the provider.
    pub label: AgentLabel,
    /// Visual tier, never an authorization decision.
    pub risk: ModeRisk,
    /// Whether this mode can run without a person answering prompts.
    pub approval: ApprovalFlow,
}

/// Validated mode list. `None` around this type means discovery is unknown;
/// an empty list explicitly means that the provider has no modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeCatalogue {
    modes: Vec<Mode>,
    default: Option<ModeId>,
}

impl ModeCatalogue {
    /// Local bound on retained permission modes.
    pub const MAX: usize = 256;

    /// Keep provider ordering, rejecting ambiguous IDs and impossible defaults.
    ///
    /// # Errors
    /// `InvalidProvider` identifies an oversized, duplicate or inconsistent list.
    pub fn new(modes: Vec<Mode>, default: Option<ModeId>) -> Result<Self, InvalidProvider> {
        if modes.len() > Self::MAX {
            return Err(InvalidProvider::TooManyModes);
        }
        let mut seen = BTreeSet::new();
        for mode in &modes {
            if !seen.insert(&mode.id) {
                return Err(InvalidProvider::DuplicateMode);
            }
        }
        if default.as_ref().is_some_and(|id| !seen.contains(id)) {
            return Err(InvalidProvider::MissingDefault);
        }
        Ok(Self { modes, default })
    }

    /// Modes in discovery order, without inventing a fallback entry.
    #[must_use]
    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

    /// None leaves the native provider free to pick its configured default.
    #[must_use]
    pub fn default_mode(&self) -> Option<&ModeId> {
        self.default.as_ref()
    }
}

impl<'de> Deserialize<'de> for ModeCatalogue {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            #[serde(deserialize_with = "read_modes")]
            modes: Vec<Mode>,
            default: Option<ModeId>,
        }
        let raw = Raw::deserialize(de)?;
        Self::new(raw.modes, raw.default).map_err(serde::de::Error::custom)
    }
}

fn read_modes<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<Mode>, D::Error> {
    struct Modes;
    impl<'de> serde::de::Visitor<'de> for Modes {
        type Value = Vec<Mode>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "at most {} modes", ModeCatalogue::MAX)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut modes = Vec::new();
            while let Some(mode) = seq.next_element()? {
                if modes.len() == ModeCatalogue::MAX {
                    return Err(serde::de::Error::custom(InvalidProvider::TooManyModes));
                }
                modes.push(mode);
            }
            Ok(modes)
        }
    }
    de.deserialize_seq(Modes)
}

/// Parent configuration used only for inheritance, never as launch authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateParent {
    /// Compare effective identities, not implementation families.
    pub provider: ProviderId,
    /// None is an intentional native default.
    pub mode: Option<ModeId>,
    /// Resolved from the parent's effective configuration.
    pub approval: ApprovalFlow,
}

/// Complete pure input before any process is started or settings are mutated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateInput {
    /// Effective provider identity, including a configured alias.
    pub provider: ProviderId,
    /// Explicit request has priority over parent inheritance.
    pub mode: Option<ModeId>,
    /// Explicit off must survive unattended inheritance.
    pub auto_accept: Option<AutoAccept>,
    /// No parent means provider defaults can apply.
    pub parent: Option<CreateParent>,
    /// Caller requested a no-prompt child.
    pub approval: ApprovalFlow,
    /// Unknown discovery is distinct from an explicitly empty list.
    pub available: Option<ModeCatalogue>,
}

/// Provider-specific resolution policy, selected once by the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatePolicy {
    /// Static/ordinary permission mode inheritance.
    Default,
    /// `OpenCode` carries unattendedness as a feature, not an agent name.
    OpenCode,
    /// ACP supports `auto_accept` across provider boundaries.
    Acp,
}

/// Resolved values; absent mode means use the provider's current native default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateConfig {
    /// Mode selected or inherited, without fabricated fallback names.
    pub mode: Option<ModeId>,
    /// Explicit or inherited feature value.
    pub auto_accept: Option<AutoAccept>,
    /// Effective no-prompt classification for a later child's inheritance.
    pub approval: ApprovalFlow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode() -> Mode {
        Mode {
            id: "plan".parse().unwrap(),
            label: "Plan".parse().unwrap(),
            risk: ModeRisk::Planning,
            approval: ApprovalFlow::Interactive,
        }
    }

    #[test]
    fn a_catalogue_cannot_select_missing_modes_or_repeat_ids() {
        assert_eq!(
            ModeCatalogue::new(vec![mode(), mode()], None),
            Err(InvalidProvider::DuplicateMode)
        );
        assert_eq!(
            ModeCatalogue::new(vec![], Some(mode().id)),
            Err(InvalidProvider::MissingDefault)
        );
        let list = ModeCatalogue::new(vec![mode()], Some(mode().id)).unwrap();
        let json = serde_json::to_string(&list).unwrap();
        assert_eq!(serde_json::from_str::<ModeCatalogue>(&json).unwrap(), list);
        assert_eq!(list.default_mode(), Some(&mode().id));
        let json = format!(
            r#"{{"modes":[{}],"default":null}}"#,
            std::iter::repeat_n(
                serde_json::to_string(&mode()).unwrap(),
                ModeCatalogue::MAX + 1
            )
            .collect::<Vec<_>>()
            .join(",")
        );
        assert!(serde_json::from_str::<ModeCatalogue>(&json).is_err());
        assert_eq!(
            ModeCatalogue::new(vec![mode(); ModeCatalogue::MAX + 1], None),
            Err(InvalidProvider::TooManyModes)
        );
    }
}
