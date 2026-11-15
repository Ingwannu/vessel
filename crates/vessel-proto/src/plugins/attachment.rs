//! Plugin context attachments use captured source/item identity, never display labels.

use std::{collections::BTreeSet, fmt, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use super::{
    Caption, ContentText, ContributionAddress, ContributionId, ContributionKind, PluginId,
};
use crate::{NodeId, key::serde_via_text};

const MAX_ITEM_ID_BYTES: usize = 256;
const MAX_ITEM_URL_BYTES: usize = 4096;
/// The whole reply fails above this bound; items are never silently truncated.
pub const MAX_ATTACHMENT_ITEMS: usize = 100;

/// A bounded context URL; its constructor rejects executable schemes and userinfo.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct AttachmentUrl(Arc<str>);

impl AttachmentUrl {
    /// Context text only; opening still requires an explicit core action.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for AttachmentUrl {
    type Err = AttachmentError;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.len() > MAX_ITEM_URL_BYTES {
            return Err(AttachmentError::UrlLength);
        }
        if raw
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(AttachmentError::UrlText);
        }
        let parsed = url::Url::parse(raw).map_err(AttachmentError::Url)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AttachmentError::UrlScheme);
        }
        let (_, hierarchy) = raw.split_once(':').ok_or(AttachmentError::UrlAuthority)?;
        let hierarchy = hierarchy
            .strip_prefix("//")
            .ok_or(AttachmentError::UrlAuthority)?;
        let authority = hierarchy
            .split(['/', '?', '#'])
            .next()
            .ok_or(AttachmentError::UrlAuthority)?;
        if authority.contains('@') || !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(AttachmentError::UrlUserinfo);
        }
        if parsed.host().is_none() {
            return Err(AttachmentError::UrlHost);
        }
        Ok(Self(Arc::from(raw)))
    }
}

impl fmt::Display for AttachmentUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(AttachmentUrl);

/// Only an attachment-source contribution can supply retained composer context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "ContributionAddress"))]
#[serde(try_from = "ContributionAddress", into = "ContributionAddress")]
pub struct AttachmentSourceAddress(ContributionAddress);

impl AttachmentSourceAddress {
    /// Captured source lifetime for search/invoke authorization.
    #[must_use]
    pub const fn address(&self) -> &ContributionAddress {
        &self.0
    }
}

impl TryFrom<ContributionAddress> for AttachmentSourceAddress {
    type Error = AttachmentError;
    fn try_from(address: ContributionAddress) -> Result<Self, Self::Error> {
        if address.kind != ContributionKind::AttachmentSource {
            return Err(AttachmentError::NotSource);
        }
        Ok(Self(address))
    }
}

impl From<AttachmentSourceAddress> for ContributionAddress {
    fn from(source: AttachmentSourceAddress) -> Self {
        source.0
    }
}

/// A source-owned item id is opaque text, not a plugin path/id grammar.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct AttachmentItemId(Arc<str>);

impl AttachmentItemId {
    /// Borrow the stable item key independently of its translated title.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for AttachmentItemId {
    type Err = AttachmentError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || text.len() > MAX_ITEM_ID_BYTES || text.chars().any(char::is_control) {
            return Err(AttachmentError::ItemId);
        }
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Display for AttachmentItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(AttachmentItemId);

/// Paseo's attachment item, with bounded text and a safe HTTP(S) URL.
/// The URL is context text; registration never fetches or opens it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AttachmentItem {
    /// Stable source-owned identity, separate from the human identifier.
    pub id: AttachmentItemId,
    /// Human-readable key such as an issue number.
    pub identifier: Caption,
    /// Picker/tray title.
    pub title: Caption,
    /// Optional secondary picker line.
    pub subtitle: Option<Caption>,
    /// Explicit context URL; executable schemes and userinfo are refused.
    pub url: AttachmentUrl,
    /// Context delivered as text through the ordinary composer.
    pub text: ContentText,
    /// Source-defined resource class displayed as text, never used to choose core behavior.
    pub resource_type: Caption,
}

/// A retained draft item still identifies the exact source lifetime that supplied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ContextAttachment {
    /// Captured node, installation generation and attachment-source contribution.
    pub source: AttachmentSourceAddress,
    /// Validated context data; sending retained text needs no helper execution.
    pub item: AttachmentItem,
}

/// Logical selection identity survives a helper restart; it does not authorize
/// another search or invoke. Sending retained text needs no live helper.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttachmentItemKey {
    node: NodeId,
    plugin: PluginId,
    source: ContributionId,
    item: AttachmentItemId,
}

impl ContextAttachment {
    /// Compare draft selection by node/plugin/source/item, independently of labels
    /// or a newer runtime generation.
    #[must_use]
    pub fn key(&self) -> AttachmentItemKey {
        let address = self.source.address();
        AttachmentItemKey {
            node: address.installation.node,
            plugin: address.installation.plugin.clone(),
            source: address.contribution.clone(),
            item: self.item.id.clone(),
        }
    }
}

/// Invalid input is a whole-result refusal; raw item ids are not echoed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttachmentError {
    /// URL errors retain their concrete cause without echoing text in this message.
    #[error("plugin attachment URL is invalid")]
    Url(#[source] url::ParseError),
    /// Control/whitespace text must not be normalized away by the URL parser.
    #[error("plugin attachment URLs cannot contain whitespace or control characters")]
    UrlText,
    /// URLs are inert context, but any later explicit opening retains safe schemes.
    #[error("plugin attachment URLs require HTTP or HTTPS")]
    UrlScheme,
    /// Explicit authority keeps displayed context links unambiguous across clients.
    #[error("plugin attachment URLs require an explicit HTTP(S) authority")]
    UrlAuthority,
    /// Userinfo is a credential-bearing URL component, never a context label.
    #[error("plugin attachment URLs cannot contain userinfo")]
    UrlUserinfo,
    /// A parsed network URL must name a host, not an opaque path.
    #[error("plugin attachment URLs require a host")]
    UrlHost,
    /// Url's existing parser has no per-item allocation cap of its own.
    #[error("plugin attachment URL exceeds {MAX_ITEM_URL_BYTES} bytes")]
    UrlLength,
    /// Context cannot be supplied under a command/theme's identity.
    #[error("context attachments require an attachment-source contribution")]
    NotSource,
    /// Opaque ids are bounded but preserve source case and punctuation.
    #[error(
        "attachment item ids must be nonempty, control-free and at most {MAX_ITEM_ID_BYTES} bytes"
    )]
    ItemId,
    /// Returning an arbitrary prefix could omit an already selected item.
    #[error("plugin attachment search exceeds {MAX_ATTACHMENT_ITEMS} items")]
    TooMany,
    /// Two rows with one key would select/deselect the wrong retained context.
    #[error("plugin attachment search repeats an item identity")]
    Duplicate,
}

/// A source search publishes either a whole bounded distinct list or an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "SearchInput")]
pub struct AttachmentSearchResult {
    items: Vec<AttachmentItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchInput {
    items: Vec<AttachmentItem>,
}

impl AttachmentSearchResult {
    /// Refuse excessive or ambiguous lists before a picker can publish them.
    ///
    /// # Errors
    /// Returns `TooMany` or `Duplicate` without truncating or merging rows.
    pub fn new(items: Vec<AttachmentItem>) -> Result<Self, AttachmentError> {
        if items.len() > MAX_ATTACHMENT_ITEMS {
            return Err(AttachmentError::TooMany);
        }
        let mut ids = BTreeSet::new();
        for item in &items {
            if !ids.insert(&item.id) {
                return Err(AttachmentError::Duplicate);
            }
        }
        Ok(Self { items })
    }

    /// Picker data is already validated; selecting rows never reparses it.
    #[must_use]
    pub fn items(&self) -> &[AttachmentItem] {
        &self.items
    }
}

impl TryFrom<SearchInput> for AttachmentSearchResult {
    type Error = AttachmentError;
    fn try_from(input: SearchInput) -> Result<Self, Self::Error> {
        Self::new(input.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> AttachmentItem {
        serde_json::from_str(r#"{"id":"ISSUE:42","identifier":"ISSUE-42","title":"제목","url":"https://example.invalid/issues/42","text":"Context\nbody","resourceType":"issue"}"#).expect("fixture")
    }

    #[test]
    fn opaque_ids_preserve_case_and_punctuation_without_accepting_controls() {
        assert_eq!(
            "ISSUE:42"
                .parse::<AttachmentItemId>()
                .expect("fixture")
                .as_str(),
            "ISSUE:42"
        );
        assert!("".parse::<AttachmentItemId>().is_err());
        assert!("id\n".parse::<AttachmentItemId>().is_err());
        assert!(
            "x".repeat(MAX_ITEM_ID_BYTES)
                .parse::<AttachmentItemId>()
                .is_ok()
        );
        assert!(
            "x".repeat(MAX_ITEM_ID_BYTES.saturating_add(1))
                .parse::<AttachmentItemId>()
                .is_err()
        );
    }

    #[test]
    fn duplicate_and_oversized_results_refuse_without_truncation() {
        assert_eq!(
            AttachmentSearchResult::new(vec![item(), item()]),
            Err(AttachmentError::Duplicate)
        );
        assert_eq!(
            AttachmentSearchResult::new(vec![item(); MAX_ATTACHMENT_ITEMS.saturating_add(1)]),
            Err(AttachmentError::TooMany)
        );
        assert!(AttachmentSearchResult::new(vec![]).is_ok());
    }

    #[test]
    fn executable_urls_and_duplicate_ids_fail_at_serde_boundary() {
        let data = serde_json::to_string(&item()).expect("encode fixture");
        let executable = data.replace("https://example.invalid/issues/42", "javascript:run()");
        assert!(serde_json::from_str::<AttachmentItem>(&executable).is_err());
        assert!(
            serde_json::from_str::<AttachmentSearchResult>(&format!(
                "{{\"items\":[{data},{data}]}}"
            ))
            .is_err()
        );
    }

    #[test]
    fn context_keys_survive_restart_but_a_different_source_kind_never_validates() {
        use crate::plugins::{PluginAddress, RuntimeGeneration};
        let mut address = ContributionAddress {
            installation: PluginAddress {
                node: NodeId::random(),
                plugin: "example".parse().expect("id"),
                generation: RuntimeGeneration::random(),
            },
            kind: ContributionKind::AttachmentSource,
            contribution: "issues".parse().expect("id"),
        };
        let first = ContextAttachment {
            source: address.clone().try_into().expect("source"),
            item: item(),
        };
        address.installation.generation = RuntimeGeneration::random();
        let next = ContextAttachment {
            source: address.clone().try_into().expect("source"),
            item: item(),
        };
        assert_eq!(first.key(), next.key());
        assert_ne!(first.source, next.source);
        address.kind = ContributionKind::Command;
        assert_eq!(
            AttachmentSourceAddress::try_from(address),
            Err(AttachmentError::NotSource)
        );
    }

    #[test]
    fn context_url_size_and_credentials_are_validated_before_publication() {
        let text = format!("https://example.invalid/{}", "x".repeat(MAX_ITEM_URL_BYTES));
        assert_eq!(
            text.parse::<AttachmentUrl>(),
            Err(AttachmentError::UrlLength)
        );
        assert!(
            "https://user:private@example.invalid/item"
                .parse::<AttachmentUrl>()
                .is_err()
        );
        let items = (0..MAX_ATTACHMENT_ITEMS)
            .map(|number| {
                let mut entry = item();
                entry.id = format!("item:{number}").parse().expect("item id");
                entry
            })
            .collect();
        assert_eq!(
            AttachmentSearchResult::new(items)
                .expect("at bound")
                .items()
                .len(),
            MAX_ATTACHMENT_ITEMS
        );
    }
    #[test]
    fn malformed_authority_and_port_fail_both_direct_and_serde_validation() {
        for input in [
            "https://[bad]/item",
            "http://:bad/item",
            "https://example.invalid:bad/item",
        ] {
            assert!(input.parse::<AttachmentUrl>().is_err());
            let encoded = serde_json::to_string(input).expect("encode fixture");
            assert!(serde_json::from_str::<AttachmentUrl>(&encoded).is_err());
        }
    }

    #[test]
    fn url_refusals_cannot_repeat_input_in_any_diagnostic_path() {
        use std::error::Error as _;
        const CANARY: &str = "PRIVATE-SYNTHETIC-CANARY";
        let inputs = [
            format!("javascript:{CANARY}"),
            format!("https://[bad]/{CANARY}"),
            format!("https://{CANARY}:bad/item"),
            format!("https://user:{CANARY}@example.invalid/item"),
        ];
        for input in inputs {
            let error = input.parse::<AttachmentUrl>().expect_err("invalid fixture");
            assert!(!format!("{error:?} {error}").contains(CANARY));
            let mut source = error.source();
            while let Some(cause) = source {
                assert!(!format!("{cause:?} {cause}").contains(CANARY));
                source = cause.source();
            }
            let encoded = serde_json::to_string(&input).expect("encode fixture");
            let error =
                serde_json::from_str::<AttachmentUrl>(&encoded).expect_err("invalid wire fixture");
            assert!(!format!("{error:?} {error}").contains(CANARY));
            let mut source = error.source();
            while let Some(cause) = source {
                assert!(!format!("{cause:?} {cause}").contains(CANARY));
                source = cause.source();
            }
        }
    }
}
