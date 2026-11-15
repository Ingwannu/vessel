//! Pending named-profile request/result contracts; one feature gates every call.

use super::{
    ConfigurationNotice, InvalidLaunchValue, LaunchChoice, LaunchChoices, LaunchPayload,
    LaunchProfileRevision, LaunchProfiles, LaunchSelection, codec,
};
use crate::{SessionId, WireError};
use serde::{Deserialize, Deserializer, Serialize, de};

/// Read the complete editable host snapshot; notes are public to Read agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ListLaunchProfiles {}

/// Replace the entire ordered snapshot with CAS; omitted fields are cleared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ReplaceLaunchProfiles {
    /// Revision the editor actually read; a conflict requires rereading.
    pub expected: LaunchProfileRevision,
    /// The exact new whole list, not a merge patch.
    pub profiles: LaunchProfiles,
}

/// Probe the actual selected configuration for a host-wide editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetLaunchProfileOptions {
    /// The exact request key echoed by the completed result.
    pub selection: LaunchSelection,
}

/// Read only presets applicable to this structured session's actual account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetApplicableLaunchProfiles {
    /// Session whose real harness/account fixes eligibility.
    pub session: SessionId,
}

/// Apply concrete choices to one captured process, with normal ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ApplySessionOptions {
    /// The session; no provider/account/process override is accepted.
    pub session: SessionId,
    /// Ordered choices; unlike profile reconciliation this is strict.
    pub options: LaunchChoices,
}

/// A filtered read result, deliberately distinct from the editable host list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ApplicableLaunchProfiles {
    revision: LaunchProfileRevision,
    profiles: LaunchProfiles,
}
impl ApplicableLaunchProfiles {
    /// Pair a validated filtered list with its host-list revision.
    #[must_use]
    pub const fn new(revision: LaunchProfileRevision, profiles: LaunchProfiles) -> Self {
        Self { revision, profiles }
    }
    /// Revision used for this eligibility read; not permission to replace it.
    #[must_use]
    pub const fn revision(&self) -> LaunchProfileRevision {
        self.revision
    }
    /// Eligible presets in their original host order.
    #[must_use]
    pub const fn profiles(&self) -> &LaunchProfiles {
        &self.profiles
    }
}

/// First failed setter, preserving the ordinary typed wire refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LaunchConfigurationFailure {
    /// The first choice not acknowledged; following choices were not sent.
    pub choice: LaunchChoice,
    /// Same boundary mapping as the old single setter.
    pub error: WireError,
}

/// Actual completed prefix and first failure; previous changes remain in effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ApplySessionOptionsResult {
    applied: LaunchChoices,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    notice: Option<ConfigurationNotice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    failure: Option<LaunchConfigurationFailure>,
}
impl ApplySessionOptionsResult {
    /// Refuse an oversized whole result rather than truncate its failure text.
    ///
    /// # Errors
    /// Combined escaped JSON exceeds the complete launch body budget.
    pub fn new(
        applied: LaunchChoices,
        notice: Option<ConfigurationNotice>,
        failure: Option<LaunchConfigurationFailure>,
    ) -> Result<Self, InvalidLaunchValue> {
        let value = Self {
            applied,
            notice,
            failure,
        };
        LaunchPayload::check(&value)?;
        Ok(value)
    }
    /// Choices the same process actually acknowledged, in order.
    #[must_use]
    pub const fn applied(&self) -> &LaunchChoices {
        &self.applied
    }
    /// First explicitly reported notice, without suppressing later steps.
    #[must_use]
    pub const fn notice(&self) -> Option<&ConfigurationNotice> {
        self.notice.as_ref()
    }
    /// The first refusal, absent only after every submitted setter completed.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaunchConfigurationFailure> {
        self.failure.as_ref()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedResult {
    applied: LaunchChoices,
    #[serde(default, deserialize_with = "codec::optional")]
    notice: Option<ConfigurationNotice>,
    #[serde(default, deserialize_with = "codec::optional")]
    failure: Option<LaunchConfigurationFailure>,
}
impl<'de> Deserialize<'de> for ApplySessionOptionsResult {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let raw: EncodedResult = codec::decode(input)?;
        Self::new(raw.applied, raw.notice, raw.failure).map_err(de::Error::custom)
    }
}

/// Stable machine-readable reasons for launch-profile refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum LaunchProfileRefusal {
    /// Finite metadata/probe or apply admission is full.
    Busy,
    /// Account/agent configuration changed during a probe; discard its result.
    ConfigurationChanged,
    /// The editor's whole-list revision is stale.
    StaleRevision {
        /// Revision now committed by the node.
        current: LaunchProfileRevision,
    },
    /// The checked revision cannot advance.
    RevisionExhausted,
    /// This editor operation is human-only, independently of scopes.
    AgentOrigin,
    /// A structured driver is unavailable, distinct from a successful empty list.
    StructuredUnavailable,
    /// A future reason remains explicit and readable.
    #[serde(other)]
    Unknown,
}
