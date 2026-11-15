//! Canonical non-tool items and provenance; opaque plugin content is bounded data.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    ExternalAgentId, ExternalHostId, PluginItemId, PluginKind, TimelineName, TimelinePluginId,
};
use crate::{RawJson, TokenCount};

/// Severity is independent of whether the agent's turn ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    /// Informational status.
    Info,
    /// A warning requiring no approval.
    Warning,
    /// An error notification, independent of a turn failure.
    Error,
}

/// A provider notice retains its severity in every timeline consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TimelineNotification {
    /// The provider's level, not derived from its text.
    pub level: NotificationLevel,
    /// Human-readable content.
    pub message: String,
}

/// Compaction is work in the turn, separate from completion of that turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CompactionStatus {
    /// Compression has started.
    Loading,
    /// Compression has finished.
    Completed,
}

/// The source of compaction, when the provider reports it explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    /// Provider-initiated context maintenance.
    Auto,
    /// A person requested compaction.
    Manual,
}

/// Completion updates the earliest outstanding loading row, even across other events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Compaction {
    /// Authoritative progress.
    pub status: CompactionStatus,
    /// Absent means unreported, not automatic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<CompactionTrigger>,
    /// Context size before compression, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_tokens: Option<TokenCount>,
}

/// Opaque plugin payloads fit within the upstream per-item byte budget.
pub const PLUGIN_TIMELINE_DATA_MAX_BYTES: usize = 64 * 1024;

/// A rejected plugin payload never echoes the untrusted content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PluginTimelineDataError {
    /// UTF-8 serialization, not character count, determines this bound.
    #[error("plugin timeline data exceeds {PLUGIN_TIMELINE_DATA_MAX_BYTES} bytes")]
    TooLarge,
}

/// Validated opaque extension data; only the plugin runtime interprets it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawJson", into = "RawJson")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "unknown"))]
pub struct PluginTimelineData(RawJson);

impl TryFrom<RawJson> for PluginTimelineData {
    type Error = PluginTimelineDataError;

    fn try_from(value: RawJson) -> Result<Self, Self::Error> {
        if value.as_text().len() > PLUGIN_TIMELINE_DATA_MAX_BYTES {
            return Err(PluginTimelineDataError::TooLarge);
        }
        Ok(Self(value))
    }
}

impl From<PluginTimelineData> for RawJson {
    fn from(value: PluginTimelineData) -> Self {
        value.0
    }
}

impl fmt::Debug for PluginTimelineData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Plugins may carry private content; diagnostic output needs only its size.
        formatter
            .debug_struct("PluginTimelineData")
            .field("bytes", &self.0.as_text().len())
            .finish()
    }
}

impl PluginTimelineData {
    /// Pass the bounded JSON to its plugin renderer without core field access.
    #[must_use]
    pub const fn raw(&self) -> &RawJson {
        &self.0
    }
}

/// Plugin contribution versions cannot use zero as an absent version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct PluginTimelineVersion(std::num::NonZeroU32);

impl PluginTimelineVersion {
    /// Validate a positive version at the contribution boundary.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match std::num::NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// The contribution schema version for its plugin runtime.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// Stable plugin identity is the pair `(plugin, id)`, never a global row id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginTimelineItem {
    /// Contribution-local item identity.
    pub id: PluginItemId,
    /// Plugin owning the contribution.
    pub plugin: TimelinePluginId,
    /// The owning plugin's contribution kind.
    pub kind: PluginKind,
    /// The owning plugin's schema version.
    pub version: PluginTimelineVersion,
    /// Bounded opaque extension content.
    pub data: PluginTimelineData,
}

/// Provenance event vocabulary, independent of provider envelope spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageEvent {
    /// Ordinary inter-agent message.
    Message,
    /// Sender's work finished.
    Finished,
    /// Sender's work failed.
    Errored,
    /// Sender needs human input.
    PermissionRequired,
    /// Sender's session closed.
    Closed,
}

/// Display identity is never substituted for authenticated caller identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentMessageSender {
    /// The source system's agent id.
    pub id: ExternalAgentId,
    /// Source host identity, when the envelope supplies one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<ExternalHostId>,
    /// Sender's display title at delivery time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<TimelineName>,
}

/// Semantic inter-agent provenance stays separate from tool presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentMessageProvenance {
    /// What happened to the sender, independent of the receiving tool status.
    pub event: AgentMessageEvent,
    /// Display provenance, not authority.
    pub sender: AgentMessageSender,
    /// Original human-readable message content.
    pub text: String,
}

/// Failed tools retain readable failure text without arbitrary error trees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ToolFailure {
    /// Reported failure explanation; absence is represented outside this value.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_budget_counts_utf8_serialization_and_applies_during_decode() {
        let value =
            RawJson::from_serialize(&"x".repeat(PLUGIN_TIMELINE_DATA_MAX_BYTES.saturating_sub(2)))
                .unwrap();
        let at_bound = PluginTimelineData::try_from(value).unwrap();
        assert_eq!(
            at_bound.raw().as_text().len(),
            PLUGIN_TIMELINE_DATA_MAX_BYTES
        );
        let too_big = RawJson::from_serialize(&"x".repeat(PLUGIN_TIMELINE_DATA_MAX_BYTES)).unwrap();
        assert_eq!(
            PluginTimelineData::try_from(too_big.clone()).unwrap_err(),
            PluginTimelineDataError::TooLarge
        );
        assert!(serde_json::from_str::<PluginTimelineData>(too_big.as_text()).is_err());
        let multibyte =
            RawJson::from_serialize(&"한".repeat(PLUGIN_TIMELINE_DATA_MAX_BYTES)).unwrap();
        assert!(PluginTimelineData::try_from(multibyte).is_err());
        assert!(PluginTimelineVersion::new(0).is_none());
        assert!(serde_json::from_str::<PluginTimelineVersion>("0").is_err());
    }

    #[test]
    fn debug_omits_plugin_content() {
        let raw = RawJson::from_text(r#"{"private":"hidden-content"}"#).unwrap();
        let bounded = PluginTimelineData::try_from(raw).unwrap();
        assert!(!format!("{bounded:?}").contains("hidden-content"));
    }
}
