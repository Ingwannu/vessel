//! Credential-use consent is source-specific; the old three-store opt-in is never widened.
use super::{BuiltinUsageSource, UsageSourceId};
use crate::{
    ProtocolVersion,
    limits::{ExternalLookup, LimitName},
};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;
/// Disclosure changes invalidate prior source consent without changing an opaque report id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageDisclosureVersion(NonZeroU16);
impl UsageDisclosureVersion {
    /// Initial built-in disclosure, independent of RPC protocol assignment.
    pub const INITIAL: Self = Self(NonZeroU16::MIN);
    /// Already validated nonzero revision.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}
/// Known local credential candidates; no actual home, path or token is disclosed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum UsageCredentialStore {
    /// Claude Code's saved OAuth sign-in.
    ClaudeCode,
    /// Codex's saved OAuth sign-in.
    Codex,
    /// `OpenCode`'s saved `OpenAI` OAuth sign-in.
    OpenCode,
    /// Copilot/GitHub environment credentials, then GitHub CLI sign-in.
    GitHubCopilot,
    /// Cursor environment, desktop read-only database, then headless sign-in.
    Cursor,
    /// Grok environment, then Grok CLI sign-in.
    Grok,
    /// Kimi environment, then Kimi Code/Kimi saved credentials.
    Kimi,
    /// `MiniMax` environment, then saved credentials/configuration; never refresh tokens.
    MiniMax,
    /// `OpenCode`'s saved Go API key.
    OpenCodeGo,
    /// Z.ai/GLM environment API key.
    Zai,
}
/// Only fixed vendor destinations, including the explicitly supported regional pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum UsageRecipient {
    /// Anthropic's fixed usage API.
    Anthropic,
    /// `OpenAI`'s fixed `ChatGPT` usage API.
    OpenAi,
    /// GitHub's fixed Copilot API.
    GitHub,
    /// Cursor's fixed dashboard API.
    Cursor,
    /// xAI's fixed Grok CLI billing API.
    Xai,
    /// Kimi's fixed coding API.
    Kimi,
    /// Only `MiniMax`'s global or China API, selected by local configuration.
    MiniMaxGlobalOrChina,
    /// `OpenCode`'s fixed Go usage API.
    OpenCode,
    /// Z.ai's fixed subscription API.
    Zai,
}
/// Built-in consent copy can be translated by clients without interpreting a path or label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageSourceDisclosure {
    /// Exact revision checked by the consent setter and every refresh.
    pub version: UsageDisclosureVersion,
    /// Safe known candidates on the owning node.
    pub store: UsageCredentialStore,
    /// Fixed permitted vendor recipients.
    pub recipient: UsageRecipient,
}
/// Existing three-store permission and new explicit source consent are distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum UsageSourceConsent {
    /// This adapter retains the exact legacy usage.lookup semantics.
    InheritedGlobal,
    /// No prior source-specific consent, or a deliberate source-specific Off.
    Off,
    /// An explicitly saved revision; differing current revisions require fresh consent.
    Granted {
        /// Revision the administrator actually accepted.
        version: UsageDisclosureVersion,
    },
}
/// A client cannot mistake an unsupported adapter or stale consent for a usable On switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum UsageSourceAvailability {
    /// The real source adapter is absent; no consent or refresh action is effective.
    Unsupported,
    /// The existing global credential-use floor is disabled.
    GlobalOff,
    /// Source-specific permission is disabled.
    SourceOff,
    /// A different disclosure revision was accepted; no fetch may run.
    DisclosureNeedsConsent,
    /// Effective permission permits an explicit Operate refresh, never an automatic fetch.
    Enabled,
}
impl UsageSourceConsent {
    /// Apply the global floor and adapter/disclosure checks in one shared order.
    #[must_use]
    pub fn availability(
        self,
        global: ExternalLookup,
        current: UsageDisclosureVersion,
        supported: bool,
    ) -> UsageSourceAvailability {
        if !supported {
            return UsageSourceAvailability::Unsupported;
        }
        if global == ExternalLookup::Off {
            return UsageSourceAvailability::GlobalOff;
        }
        match self {
            Self::InheritedGlobal => UsageSourceAvailability::Enabled,
            Self::Off => UsageSourceAvailability::SourceOff,
            Self::Granted { version } if version == current => UsageSourceAvailability::Enabled,
            Self::Granted { .. } => UsageSourceAvailability::DisclosureNeedsConsent,
        }
    }
}
/// Cache-only source discovery metadata contains no credential or configurable endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageSourceDescriptor {
    /// Stable registered identity, distinct from its label.
    pub id: UsageSourceId,
    /// Bounded display name, never used for policy lookup.
    pub label: LimitName,
    /// Optional official built-in mark.
    pub builtin: Option<BuiltinUsageSource>,
    /// What local sign-in candidates may be read and which vendor may receive them.
    pub disclosure: UsageSourceDisclosure,
    /// Saved permission provenance, independent of current effective availability.
    pub consent: UsageSourceConsent,
    /// Current complete effective state; Unsupported cannot appear enabled.
    pub availability: UsageSourceAvailability,
}
/// Only the accepted source identity and disclosure revision are persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageSourceGrant {
    /// Known source whose explicit consent was accepted.
    pub id: UsageSourceId,
    /// Exact revision accepted; mismatch cannot authorize changed disclosure.
    pub version: UsageDisclosureVersion,
}
/// Source-specific permission is an Admin mutation; it never initiates a fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetUsageSourceLookup {
    /// Exact registered source; unknown ids refuse instead of creating a policy.
    pub source: UsageSourceId,
    /// Explicit permission choice, separate from global usage.lookup.
    pub policy: ExternalLookup,
    /// The displayed revision; stale revisions refuse even when enabling.
    pub disclosure_version: UsageDisclosureVersion,
}
/// Protocol 26 (2026-11-16), the same version as the reports it governs:
/// `usage.source_lookup` (Admin, unary). It takes an exact registered source
/// and the current disclosure revision, never fetches, and cannot enable an
/// Unsupported source (ADR 0042).
pub const USAGE_SOURCE_CONSENT_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(26));
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_legacy_grant_can_enable_an_additional_or_unsupported_source() {
        let version = UsageDisclosureVersion::INITIAL;
        assert_eq!(
            UsageSourceConsent::Off.availability(ExternalLookup::On, version, true),
            UsageSourceAvailability::SourceOff
        );
        assert_eq!(
            UsageSourceConsent::InheritedGlobal.availability(ExternalLookup::On, version, true),
            UsageSourceAvailability::Enabled
        );
        assert_eq!(
            UsageSourceConsent::Granted { version }.availability(
                ExternalLookup::On,
                version,
                false
            ),
            UsageSourceAvailability::Unsupported
        );
        assert_eq!(
            UsageSourceConsent::Granted { version }.availability(
                ExternalLookup::Off,
                version,
                true
            ),
            UsageSourceAvailability::GlobalOff
        );
        let next = serde_json::from_str::<UsageDisclosureVersion>("2").unwrap();
        assert_eq!(
            UsageSourceConsent::Granted { version }.availability(ExternalLookup::On, next, true),
            UsageSourceAvailability::DisclosureNeedsConsent
        );
        assert!(serde_json::from_str::<UsageDisclosureVersion>("0").is_err());
    }
}
