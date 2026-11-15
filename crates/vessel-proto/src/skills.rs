//! Bundled orchestration skills: values only, with no access to agent homes.
//! Admin-only local requests use the shared staged table until `MCP_PARITY` is numbered.

mod name;

pub use name::{InvalidSkillName, SkillName};

use serde::{Deserialize, Deserializer, Serialize};

/// Maximum complete shipped skill catalogue and selection size.
pub const MAX_SKILLS: usize = 128;

/// A normalized set used for selections, installed names and confirmations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SkillNames(Vec<SkillName>);
impl SkillNames {
    /// Normalize a bounded selection so repeated or reordered names agree.
    ///
    /// # Errors
    /// The input exceeds [`MAX_SKILLS`] before normalization.
    pub fn new(mut names: Vec<SkillName>) -> Result<Self, InvalidSkills> {
        if names.len() > MAX_SKILLS {
            return Err(InvalidSkills::Count);
        }
        names.sort();
        names.dedup();
        Ok(Self(names))
    }
    /// Read the stable order without permitting unchecked insertion.
    #[must_use]
    pub fn as_slice(&self) -> &[SkillName] {
        &self.0
    }
    /// Whether a confirmation names the directory in the current frozen plan.
    #[must_use]
    pub fn contains(&self, name: &SkillName) -> bool {
        self.0.binary_search(name).is_ok()
    }
    /// Whether a custom selection intentionally installs nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl<'de> Deserialize<'de> for SkillNames {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let names = crate::mcp::bounded::decode::<_, _, MAX_SKILLS>(input)?;
        Self::new(names).map_err(D::Error::custom)
    }
}

/// All follows future bundle additions; custom remains the exact chosen set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SkillSelection {
    /// Default selection; installation is still an explicit action.
    #[default]
    All,
    /// Normalized selected bundle directory names.
    Custom {
        /// Unknown available names are refused by the service before writes.
        skills: SkillNames,
    },
}

/// A complete status without conflating selected and actually installed skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SkillsState {
    /// No managed directory exists in any target.
    NotInstalled,
    /// Selected files agree with the current bundle in every target.
    UpToDate,
    /// Adds, updates or explicit deletions remain.
    Drift,
}

/// One user-visible operation against a bundled skill name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SkillOperation {
    /// At least one target lacks the selected skill.
    Add {
        /// The bundled directory.
        name: SkillName,
    },
    /// A selected target differs in managed contents.
    Update {
        /// The bundled directory.
        name: SkillName,
    },
    /// An installed managed directory is no longer selected.
    Delete {
        /// The managed directory, requiring explicit confirmation.
        name: SkillName,
    },
}
impl SkillOperation {
    /// Shared identity for confirmation and deterministic operation display.
    #[must_use]
    pub const fn skill(&self) -> &SkillName {
        match self {
            Self::Add { name } | Self::Update { name } | Self::Delete { name } => name,
        }
    }
}

/// One complete view for host settings and onboarding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SkillsSnapshot {
    /// Actual installed state, independent from the selection.
    pub state: SkillsState,
    /// Complete typed operations; no silently truncated list.
    #[serde(deserialize_with = "crate::mcp::bounded::decode::<_, _, MAX_SKILLS>")]
    pub ops: Vec<SkillOperation>,
    /// Names this release ships.
    pub available: SkillNames,
    /// Names with an existing directory in at least one target.
    pub installed: SkillNames,
    /// Durable user preference.
    pub selection: SkillSelection,
}

/// A selection save either commits or names deletions without changing state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SkillsSaved {
    /// Fresh view after saving, or the unchanged current view on confirmation.
    pub snapshot: SkillsSnapshot,
    /// None means committed; otherwise confirm these exact fresh-plan removals.
    pub confirmation_required: Option<SkillNames>,
}

/// Read host-level state; Admin only, never an agent tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetSkillsStatus {}
/// Explicit non-destructive install/update of selected bundled skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReconcileSkills {}
/// Explicit whole-directory uninstall, confirmed by the human-facing client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UninstallSkills {}
/// Save only after the fresh deletion plan has been explicitly confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SaveSkillsSelection {
    /// New desired set; all picks up future bundled additions.
    pub selection: SkillSelection,
    /// Empty on first attempt; retry with the returned exact names.
    #[serde(default)]
    pub confirmed_removals: SkillNames,
}
/// Import older client selection only if the host has never stored one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ImportSkillsSelection {
    /// Validated legacy preference, never arbitrary input paths.
    pub selection: SkillSelection,
}
/// Import never overwrites an explicit current choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SkillsImported {
    /// The durable choice after import or refusal to replace.
    pub selection: SkillSelection,
    /// Whether this call established the initial selection.
    pub outcome: SkillImportOutcome,
}
/// Import acknowledgement without an ambiguous boolean result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SkillImportOutcome {
    /// Stored because no earlier selection existed.
    Imported,
    /// The existing explicit choice was retained.
    AlreadySet,
}

/// A rejected complete skill contract.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidSkills {
    /// No retained list may exceed the catalogue bound.
    #[error("skill list exceeds {MAX_SKILLS} entries")]
    Count,
}

#[cfg(test)]
mod tests {
    mod admission;
    use super::*;
    use serde_json::json;

    #[test]
    fn selections_normalize_and_do_not_accept_paths() {
        let chosen: SkillSelection =
            serde_json::from_value(json!({"mode":"custom","skills":["b","a","a"]})).unwrap();
        assert_eq!(
            serde_json::to_value(chosen).unwrap(),
            json!({"mode":"custom","skills":["a","b"]})
        );
        for name in ["../outside", ".system", "a/b", "a\\b", "CON", ""] {
            assert!(name.parse::<SkillName>().is_err(), "{name}");
        }
    }

    #[test]
    fn oversized_selection_is_refused_before_normalizing_duplicates() {
        let excess = vec!["same"; MAX_SKILLS.saturating_add(1)];
        assert!(
            serde_json::from_value::<SkillSelection>(json!({"mode":"custom","skills":excess}))
                .is_err()
        );
    }
}
