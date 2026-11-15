//! A preset's immutable target/metadata and exact revision representation.

use super::{
    LaunchChoices, LaunchColorKey, LaunchIconKey, LaunchProfileName, LaunchProfileNotes, codec,
};
use crate::{HarnessRef, LaunchProfileId, ProfileName};
use serde::{Deserialize, Deserializer, Serialize};

/// A JSON-exact revision; comparison/increment belongs to the store transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct LaunchProfileRevision(u32);
impl<'de> Deserialize<'de> for LaunchProfileRevision {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        codec::decode(input).map(Self)
    }
}
impl LaunchProfileRevision {
    /// Initial empty state, not an unknown-state sentinel.
    pub const ZERO: Self = Self(0);
    /// The representation's greatest exactly JSON-representable revision.
    pub const MAX: Self = Self(u32::MAX);
    /// Wrap an already bounded unsigned revision.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    /// SQL binding and presentation value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
    /// Exhaustion never wraps and cannot collide with an earlier edit.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// A launch selection only; no credentials, environment or execution authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchSelection {
    harness: HarnessRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    account_profile: Option<ProfileName>,
    options: LaunchChoices,
}
impl LaunchSelection {
    /// Keep one typed selection; availability is resolved only on actual use.
    #[must_use]
    pub const fn new(
        harness: HarnessRef,
        account_profile: Option<ProfileName>,
        options: LaunchChoices,
    ) -> Self {
        Self {
            harness,
            account_profile,
            options,
        }
    }
    /// The agent, without guessing a provider-specific configuration.
    #[must_use]
    pub const fn harness(&self) -> &HarnessRef {
        &self.harness
    }
    /// An existing account name; absence selects that harness's default.
    #[must_use]
    pub const fn account_profile(&self) -> Option<&ProfileName> {
        self.account_profile.as_ref()
    }
    /// The agent's ordered choices.
    #[must_use]
    pub const fn options(&self) -> &LaunchChoices {
        &self.options
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedSelection {
    harness: HarnessRef,
    #[serde(default, deserialize_with = "codec::optional")]
    account_profile: Option<ProfileName>,
    #[serde(default)]
    options: LaunchChoices,
}
impl<'de> Deserialize<'de> for LaunchSelection {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let raw: EncodedSelection = codec::decode(input)?;
        Ok(Self::new(raw.harness, raw.account_profile, raw.options))
    }
}

/// A stable-id named bundle; later replacement never mutates existing sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchProfile {
    id: LaunchProfileId,
    name: LaunchProfileName,
    launch: LaunchSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    icon: Option<LaunchIconKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    color: Option<LaunchColorKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    notes: Option<LaunchProfileNotes>,
}
impl LaunchProfile {
    /// A minimal bundle; missing optional values are deliberately absent.
    #[must_use]
    pub const fn new(
        id: LaunchProfileId,
        name: LaunchProfileName,
        launch: LaunchSelection,
    ) -> Self {
        Self {
            id,
            name,
            launch,
            icon: None,
            color: None,
            notes: None,
        }
    }
    /// Stable across replacement and reorder.
    #[must_use]
    pub const fn id(&self) -> LaunchProfileId {
        self.id
    }
    /// Display name; duplicate names are allowed.
    #[must_use]
    pub const fn display_name(&self) -> &LaunchProfileName {
        &self.name
    }
    /// Selection to materialize, without retaining a live profile binding.
    #[must_use]
    pub const fn launch(&self) -> &LaunchSelection {
        &self.launch
    }
    /// Registry icon key, with client fallback for unknown keys.
    #[must_use]
    pub const fn icon(&self) -> Option<&LaunchIconKey> {
        self.icon.as_ref()
    }
    /// Registry color key, with client fallback for unknown keys.
    #[must_use]
    pub const fn color(&self) -> Option<&LaunchColorKey> {
        self.color.as_ref()
    }
    /// Public guidance readable with the list, not secret storage.
    #[must_use]
    pub const fn notes(&self) -> Option<&LaunchProfileNotes> {
        self.notes.as_ref()
    }
    /// Replace/clear the icon without merging another record.
    #[must_use]
    pub fn with_icon(mut self, icon: Option<LaunchIconKey>) -> Self {
        self.icon = icon;
        self
    }
    /// Replace/clear the color without merging another record.
    #[must_use]
    pub fn with_color(mut self, color: Option<LaunchColorKey>) -> Self {
        self.color = color;
        self
    }
    /// Replace/clear notes without retaining omitted old text.
    #[must_use]
    pub fn with_notes(mut self, notes: Option<LaunchProfileNotes>) -> Self {
        self.notes = notes;
        self
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedProfile {
    id: LaunchProfileId,
    name: LaunchProfileName,
    launch: LaunchSelection,
    #[serde(default, deserialize_with = "codec::optional")]
    icon: Option<LaunchIconKey>,
    #[serde(default, deserialize_with = "codec::optional")]
    color: Option<LaunchColorKey>,
    #[serde(default, deserialize_with = "codec::optional")]
    notes: Option<LaunchProfileNotes>,
}
impl<'de> Deserialize<'de> for LaunchProfile {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let raw: EncodedProfile = codec::decode(input)?;
        Ok(Self::new(raw.id, raw.name, raw.launch)
            .with_icon(raw.icon)
            .with_color(raw.color)
            .with_notes(raw.notes))
    }
}
