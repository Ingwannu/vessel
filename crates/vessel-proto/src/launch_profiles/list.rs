//! Whole ordered snapshots with count, identity and escaped-byte invariants.

use super::{InvalidLaunchValue, LaunchPayload, LaunchProfile, LaunchProfileRevision, codec};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{collections::HashSet, sync::Arc};

/// A complete ordered list; no skipped/partial entries and no mutable vector.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(as = "Vec<LaunchProfile>"))]
#[serde(transparent)]
pub struct LaunchProfiles(Arc<[LaunchProfile]>);
impl LaunchProfiles {
    /// Most presets retained in one node's complete snapshot.
    pub const MAX_PROFILES: usize = 64;
    /// Borrow in stored order, preserving stable ids.
    #[must_use]
    pub fn as_slice(&self) -> &[LaunchProfile] {
        &self.0
    }
}
impl TryFrom<Vec<LaunchProfile>> for LaunchProfiles {
    type Error = InvalidLaunchValue;
    fn try_from(profiles: Vec<LaunchProfile>) -> Result<Self, Self::Error> {
        if profiles.len() > Self::MAX_PROFILES {
            return Err(InvalidLaunchValue::Profiles);
        }
        let mut seen = HashSet::new();
        if profiles.iter().any(|p| !seen.insert(p.id())) {
            return Err(InvalidLaunchValue::DuplicateProfile);
        }
        let list = Self(Arc::from(profiles));
        // The later revision may have more digits; budget it now so no
        // accepted edit can become unreadable on the next increment.
        LaunchPayload::check(&LaunchProfilesSnapshot::new(
            LaunchProfileRevision::MAX,
            list.clone(),
        ))?;
        Ok(list)
    }
}
impl<'de> Deserialize<'de> for LaunchProfiles {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let rows = codec::bounded::<D, LaunchProfile, { Self::MAX_PROFILES }>(input)?;
        Self::try_from(rows).map_err(de::Error::custom)
    }
}

/// One atomic database/list response. Its list has already budgeted every revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchProfilesSnapshot {
    revision: LaunchProfileRevision,
    profiles: LaunchProfiles,
}
impl LaunchProfilesSnapshot {
    /// Pair a validated whole list with its committed revision.
    #[must_use]
    pub const fn new(revision: LaunchProfileRevision, profiles: LaunchProfiles) -> Self {
        Self { revision, profiles }
    }
    /// Used for a future whole-list compare-and-replace.
    #[must_use]
    pub const fn revision(&self) -> LaunchProfileRevision {
        self.revision
    }
    /// The complete immutable ordered list.
    #[must_use]
    pub const fn profiles(&self) -> &LaunchProfiles {
        &self.profiles
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedSnapshot {
    revision: LaunchProfileRevision,
    profiles: LaunchProfiles,
}
impl<'de> Deserialize<'de> for LaunchProfilesSnapshot {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let raw: EncodedSnapshot = codec::decode(input)?;
        Ok(Self::new(raw.revision, raw.profiles))
    }
}
