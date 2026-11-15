//! Fixed source registration metadata never reads a credential or accepts a configurable URL.
use vessel_proto::{
    limits::ExternalLookup,
    usage_reports::{
        BuiltinUsageSource, UsageCredentialStore, UsageDisclosureVersion, UsageRecipient,
        UsageSourceAvailability, UsageSourceConsent, UsageSourceDescriptor, UsageSourceDisclosure,
        UsageSourceId,
    },
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Copilot,
    Cursor,
    Grok,
    Kimi,
    MiniMax,
    OpenCodeGo,
    Zai,
}
impl Source {
    pub(super) const ALL: [Self; 7] = [
        Self::Copilot,
        Self::Cursor,
        Self::Grok,
        Self::Kimi,
        Self::MiniMax,
        Self::OpenCodeGo,
        Self::Zai,
    ];
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Copilot => "copilot",
            Self::Cursor => "cursor",
            Self::Grok => "grok",
            Self::Kimi => "kimi",
            Self::MiniMax => "minimax",
            Self::OpenCodeGo => "opencode-go",
            Self::Zai => "zai",
        }
    }
    pub(super) fn from_id(id: &UsageSourceId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|source| source.name() == id.as_str())
    }
    pub(super) const fn builtin(self) -> BuiltinUsageSource {
        match self {
            Self::Copilot => BuiltinUsageSource::Copilot,
            Self::Cursor => BuiltinUsageSource::Cursor,
            Self::Grok => BuiltinUsageSource::Grok,
            Self::Kimi => BuiltinUsageSource::Kimi,
            Self::MiniMax => BuiltinUsageSource::MiniMax,
            Self::OpenCodeGo => BuiltinUsageSource::OpenCodeGo,
            Self::Zai => BuiltinUsageSource::Zai,
        }
    }
    pub(super) fn descriptor(
        self,
        global: ExternalLookup,
        consent: UsageSourceConsent,
        supported: bool,
    ) -> Result<UsageSourceDescriptor, vessel_proto::WireError> {
        let (label, store, recipient) = match self {
            Self::Copilot => (
                "GitHub Copilot",
                UsageCredentialStore::GitHubCopilot,
                UsageRecipient::GitHub,
            ),
            Self::Cursor => (
                "Cursor",
                UsageCredentialStore::Cursor,
                UsageRecipient::Cursor,
            ),
            Self::Grok => ("Grok", UsageCredentialStore::Grok, UsageRecipient::Xai),
            Self::Kimi => ("Kimi", UsageCredentialStore::Kimi, UsageRecipient::Kimi),
            Self::MiniMax => (
                "MiniMax",
                UsageCredentialStore::MiniMax,
                UsageRecipient::MiniMaxGlobalOrChina,
            ),
            Self::OpenCodeGo => (
                "OpenCode Go",
                UsageCredentialStore::OpenCodeGo,
                UsageRecipient::OpenCode,
            ),
            Self::Zai => ("Z.ai", UsageCredentialStore::Zai, UsageRecipient::Zai),
        };
        let version = UsageDisclosureVersion::INITIAL;
        Ok(UsageSourceDescriptor {
            id: self.name().parse().map_err(|_invalid| super::changed())?,
            label: label.parse().map_err(|_invalid| super::changed())?,
            builtin: Some(self.builtin()),
            disclosure: UsageSourceDisclosure {
                version,
                store,
                recipient,
            },
            consent,
            availability: consent.availability(global, version, supported),
        })
    }
}
pub(super) fn legacy(
    global: ExternalLookup,
    supported: bool,
) -> Result<Vec<UsageSourceDescriptor>, vessel_proto::WireError> {
    use crate::usage_reports::source::SavedSource;
    [
        SavedSource::Claude,
        SavedSource::Codex,
        SavedSource::OpenCode,
    ]
    .into_iter()
    .map(|source| {
        let (store, recipient) = match source {
            SavedSource::Claude => (UsageCredentialStore::ClaudeCode, UsageRecipient::Anthropic),
            SavedSource::Codex => (UsageCredentialStore::Codex, UsageRecipient::OpenAi),
            SavedSource::OpenCode => (UsageCredentialStore::OpenCode, UsageRecipient::OpenAi),
        };
        let version = UsageDisclosureVersion::INITIAL;
        let consent = UsageSourceConsent::InheritedGlobal;
        Ok(UsageSourceDescriptor {
            id: source.id().map_err(|_invalid| super::changed())?,
            label: source
                .label()
                .parse()
                .map_err(|_invalid| super::changed())?,
            builtin: Some(source.builtin()),
            disclosure: UsageSourceDisclosure {
                version,
                store,
                recipient,
            },
            consent,
            availability: if supported {
                consent.availability(global, version, true)
            } else {
                UsageSourceAvailability::Unsupported
            },
        })
    })
    .collect()
}
