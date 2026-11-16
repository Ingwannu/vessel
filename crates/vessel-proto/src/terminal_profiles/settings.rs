//! Settings CAS and safe picker metadata share the validated profile values.

use super::{Icon, Id, Name, ProfileError, Profiles};
use serde::{Deserialize, Serialize};

/// A settings revision is distinct from a creation-operation revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "TerminalProfileRevision"))]
pub struct ProfileRevision(u64);
impl ProfileRevision {
    /// Absent configuration uses reference defaults at the initial revision.
    pub const INITIAL: Self = Self(0);
    /// Decode `SQLite` only after its nonnegative integer constraint is checked.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// A changed configuration never wraps into an older CAS version.
    /// # Errors
    /// The revision no longer fits the existing `SQLite` integer representation.
    pub fn next(self) -> Result<Self, ProfileError> {
        let next = self
            .0
            .checked_add(1)
            .filter(|next| i64::try_from(*next).is_ok())
            .ok_or(ProfileError::RevisionExhausted)?;
        Ok(Self(next))
    }
    /// Storage/query boundaries preserve the revision's unit.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Whether the creation form accepts a prompt or shows a command read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "TerminalProfilePromptMode"))]
pub enum PromptMode {
    /// The profile contains the literal sentinel in command or argv.
    Editable,
    /// The template has no prompt input.
    ReadOnly,
}

/// Read-scope picker metadata never exposes configured command/argv contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "TerminalProfileSummary"))]
pub struct ProfileSummary {
    /// Stable configuration identity, preserving configured order.
    pub id: Id,
    /// A selected id cannot silently resolve under a later configuration.
    pub revision: ProfileRevision,
    /// Display label, independently validated from the executable.
    pub name: Name,
    /// Optional validated visual catalog identity.
    pub icon: Option<Icon>,
    /// The form's actual editor mode rather than a boolean at the boundary.
    pub prompt: PromptMode,
}

/// Exact configuration choice retained in one creation intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "TerminalProfileSelection"))]
pub struct ProfileSelection {
    /// The selected profile, independent of its program/template.
    pub profile: Id,
    /// The settings snapshot shown by the safe metadata picker.
    pub revision: ProfileRevision,
}
impl ProfileSummary {
    /// Retain the exact safe picker choice without copying command templates.
    #[must_use]
    pub fn selection(&self) -> ProfileSelection {
        ProfileSelection {
            profile: self.id.clone(),
            revision: self.revision,
        }
    }
}

/// Administrator-only configuration snapshot. Explicit empty disables profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "TerminalProfileSettings"))]
pub struct ProfileSettings {
    /// Expected by the next mutation, preventing lost concurrent edits.
    pub revision: ProfileRevision,
    /// Validated ordered templates; never serialized in a creation receipt.
    pub profiles: Profiles,
}
impl ProfileSettings {
    /// The picker receives only safe metadata and no execution templates.
    #[must_use]
    pub fn summaries(&self) -> Vec<ProfileSummary> {
        self.profiles
            .entries()
            .iter()
            .map(|profile| ProfileSummary {
                id: profile.id().clone(),
                revision: self.revision,
                name: profile.name().clone(),
                icon: profile.icon().cloned(),
                prompt: if profile.takes_prompt() {
                    PromptMode::Editable
                } else {
                    PromptMode::ReadOnly
                },
            })
            .collect()
    }
}

/// Request safe picker metadata without exposing administrator-defined argv.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListTerminalProfiles;
/// Read full configuration only with administrator scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetTerminalProfiles;
/// Replace one complete bounded configuration with compare-and-swap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetTerminalProfiles {
    /// The last observed configuration version.
    pub expected: ProfileRevision,
    /// Validation happens before the writer receives this payload.
    pub profiles: Profiles,
}
