//! A plugin theme identity is two existing validated scalar ids, not a URL.
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use vessel_proto::plugins::{ContributionId, InvalidPluginValue, PluginId};

const SEPARATOR: &str = "/theme/";
/// Stable plugin-owned theme identity; no host or runtime generation is saved.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PluginThemeId {
    plugin: PluginId,
    contribution: ContributionId,
}
/// Why an external saved key is refused, without echoing its text.
#[derive(Debug, thiserror::Error)]
pub enum InvalidThemeKey {
    /// Only the stable plugin/theme/contribution shape is accepted.
    #[error("a contributed theme requires a plugin/theme/contribution identity")]
    Shape,
    /// Reuse the plugin owner's scalar validation and retain its error.
    #[error("invalid contributed theme identity: {0}")]
    Scalar(#[from] InvalidPluginValue),
}
impl PluginThemeId {
    /// Form a key from already validated plugin-owned identifiers.
    #[must_use]
    pub const fn new(plugin: PluginId, contribution: ContributionId) -> Self {
        Self {
            plugin,
            contribution,
        }
    }
    /// The plugin owner.
    #[must_use]
    pub const fn plugin(&self) -> &PluginId {
        &self.plugin
    }
    /// The theme within its plugin.
    #[must_use]
    pub const fn contribution(&self) -> &ContributionId {
        &self.contribution
    }
}
impl FromStr for PluginThemeId {
    type Err = InvalidThemeKey;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let (plugin, contribution) = raw.split_once(SEPARATOR).ok_or(InvalidThemeKey::Shape)?;
        Ok(Self::new(plugin.parse()?, contribution.parse()?))
    }
}
impl fmt::Display for PluginThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{SEPARATOR}{}", self.plugin, self.contribution)
    }
}
impl TryFrom<String> for PluginThemeId {
    type Error = InvalidThemeKey;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        raw.parse()
    }
}
impl From<PluginThemeId> for String {
    fn from(key: PluginThemeId) -> Self {
        key.to_string()
    }
}
