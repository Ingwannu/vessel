//! A client document becomes visible only after its complete graph validates.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::declarations::{Contribution, ContributionContent};
use super::{PluginJsonError, RpcName, SettingsDeclarations, SettingsId, ThemeContribution};
use crate::ProtocolVersion;

#[path = "document_validator.rs"]
mod validator;

/// Maximum encoded document before allocating a parsed graph (Vessel resource limit).
pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
/// Maximum declarations in a single installation's client document.
pub const MAX_CONTRIBUTIONS: usize = 256;
/// Maximum methods advertised by one helper generation.
pub const MAX_METHODS: usize = 256;
/// Maximum control/menu entries across the entire document, not per subtree.
pub const MAX_CONTROLS: usize = 1024;
/// Maximum nested containers/popovers; checked before recursively visiting children.
pub const MAX_DEPTH: usize = 8;
/// Maximum opaque arguments captured in one static action.
pub const MAX_ACTION_INPUT_BYTES: usize = 64 * 1024;

/// A validated declaration graph. Private fields prevent bypassing validation by
/// mutating a previously valid document; catalog holders may borrow it cheaply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClientDocument {
    methods: Vec<RpcName>,
    contributions: Vec<Contribution>,
    themes: Vec<ThemeContribution>,
    /// Settings documents its controls bind; found by the check, not sent.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    bindings: BTreeSet<SettingsId>,
    /// The oldest client protocol that decodes every kind it uses; found by
    /// the check, not sent (the catalog entry carries it).
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    required_client: ProtocolVersion,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentInput {
    methods: Vec<RpcName>,
    contributions: Vec<Contribution>,
    themes: Vec<ThemeContribution>,
}

/// A rejected graph never publishes a partially functioning installation.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The decoder keeps its concrete cause without logging untrusted text.
    #[error("invalid plugin declaration JSON")]
    Decode(#[source] PluginJsonError),
    /// Programmatically constructed documents must obey the same encoded budget.
    #[error("plugin declarations could not be encoded")]
    Encode(#[source] PluginJsonError),
    /// Encoded input must be bounded before deserializing.
    #[error("plugin declarations exceed {MAX_DOCUMENT_BYTES} bytes")]
    Bytes,
    /// Contributions and themes share the same budget.
    #[error("plugin declarations exceed {MAX_CONTRIBUTIONS} contributions")]
    Contributions,
    /// Method lists must not create unbounded runtime maps.
    #[error("plugin declarations exceed {MAX_METHODS} methods")]
    Methods,
    /// Control budgets include menu entries and all popover subtrees.
    #[error("plugin declarations exceed {MAX_CONTROLS} controls/menu entries")]
    Controls,
    /// Bounded recursion prevents stack exhaustion in later renderers.
    #[error("plugin declarations exceed nesting depth {MAX_DEPTH}")]
    Depth,
    /// A duplicate cannot choose which action or state identity wins.
    #[error("plugin declarations contain a duplicate identity")]
    Duplicate,
    /// References resolve inside the same installation, never another host.
    #[error("plugin declarations refer to an undeclared screen")]
    MissingScreen,
    /// An action can invoke only a method this helper advertises.
    #[error("plugin declarations refer to an unregistered method")]
    MissingMethod,
    /// Empty menus, choices and panel locations cannot be rendered usefully.
    #[error("plugin declarations contain an empty option or target list")]
    EmptyOptions,
    /// Selected options must still exist in the declared choice.
    #[error("plugin declarations select an undeclared choice value")]
    MissingChoice,
    /// Plugin registration uses a stricter name grammar than provider commands.
    #[error(
        "plugin slash names require a lowercase letter, then lowercase letters, digits or hyphens"
    )]
    SlashName(#[source] super::InvalidPluginValue),
    /// Argument payloads have their own cap within the document budget.
    #[error("plugin action input exceeds {MAX_ACTION_INPUT_BYTES} bytes")]
    ActionInput,
    /// A control binds a settings document its helper did not declare in
    /// `ready` (a client-only plugin declares none).
    #[error("plugin declarations bind a settings document the helper did not declare")]
    UndeclaredSettings,
}

impl ClientDocument {
    /// Decode only bounded bytes, then validate all cross-references atomically.
    ///
    /// # Errors
    /// Refuses malformed/unknown fields, duplicates, excess budgets and dangling actions.
    pub fn decode(bytes: &[u8]) -> Result<Self, DocumentError> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(DocumentError::Bytes);
        }
        let input = serde_json::from_slice::<DocumentInput>(bytes)
            .map_err(|cause| DocumentError::Decode(PluginJsonError::new(cause)))?;
        input.try_into()
    }

    /// Construct an immutable graph for a helper or typed fixture.
    ///
    /// # Errors
    /// Refuses any graph that could not be safely rendered or dispatched.
    pub fn new(
        methods: Vec<RpcName>,
        contributions: Vec<Contribution>,
        themes: Vec<ThemeContribution>,
    ) -> Result<Self, DocumentError> {
        let mut graph = Self {
            methods,
            contributions,
            themes,
            bindings: BTreeSet::new(),
            required_client: validator::base_protocol(),
        };
        let (bindings, required_client) = graph.validate()?;
        graph.bindings = bindings;
        graph.required_client = required_client;
        graph.validate_encoded_budget()?;
        Ok(graph)
    }

    /// The settings documents its controls bind.
    pub fn bound_settings(&self) -> impl Iterator<Item = &SettingsId> {
        self.bindings.iter()
    }

    /// Whether every bound document is one `declared` names (the same
    /// helper's `ready`; empty for a client-only plugin).
    ///
    /// # Errors
    /// `UndeclaredSettings` names no document, so a log line stays safe.
    pub fn check_settings(&self, declared: &SettingsDeclarations) -> Result<(), DocumentError> {
        if self
            .bindings
            .iter()
            .all(|settings| declared.version_of(settings).is_some())
        {
            Ok(())
        } else {
            Err(DocumentError::UndeclaredSettings)
        }
    }

    /// The oldest client protocol whose decoder knows every kind this
    /// document uses; the catalog omits it for an older caller (ADR 0115,
    /// "Surfaces").
    #[must_use]
    pub const fn required_client(&self) -> ProtocolVersion {
        self.required_client
    }

    /// Registered method identities advertised by this document's helper.
    #[must_use]
    pub fn methods(&self) -> &[RpcName] {
        &self.methods
    }

    /// Borrow every validated contribution without copying retained UI data.
    #[must_use]
    pub fn contributions(&self) -> &[Contribution] {
        &self.contributions
    }

    /// Theme selection is client-owned; these values carry no host preference.
    #[must_use]
    pub fn themes(&self) -> &[ThemeContribution] {
        &self.themes
    }

    fn validate(&self) -> Result<(BTreeSet<SettingsId>, ProtocolVersion), DocumentError> {
        if self.contributions.len().saturating_add(self.themes.len()) > MAX_CONTRIBUTIONS {
            return Err(DocumentError::Contributions);
        }
        if self.methods.len() > MAX_METHODS {
            return Err(DocumentError::Methods);
        }
        let methods: BTreeSet<_> = self.methods.iter().collect();
        if methods.len() != self.methods.len() {
            return Err(DocumentError::Duplicate);
        }
        let screens = self
            .contributions
            .iter()
            .filter_map(|item| {
                matches!(item.content, ContributionContent::Screen { .. }).then_some(&item.id)
            })
            .collect();
        let settings_screens = self
            .contributions
            .iter()
            .filter_map(|item| {
                matches!(item.content, ContributionContent::SettingsScreen { .. })
                    .then_some(&item.id)
            })
            .collect();
        let mut validator = validator::Validator {
            methods,
            screens,
            settings_screens,
            count: 0,
            bindings: BTreeSet::new(),
            required_client: validator::base_protocol(),
        };
        let mut identities = BTreeSet::new();
        let mut slash_names = BTreeSet::new();
        for item in &self.contributions {
            if !identities.insert((item.content.kind(), &item.id)) {
                return Err(DocumentError::Duplicate);
            }
            if let ContributionContent::SlashCommand { name, .. } = &item.content {
                let _validated = name
                    .as_str()
                    .parse::<super::PluginId>()
                    .map_err(DocumentError::SlashName)?;
                if !slash_names.insert(name.as_str()) {
                    return Err(DocumentError::Duplicate);
                }
            }
            validator.contribution(&item.content)?;
        }
        let mut theme_ids = BTreeSet::new();
        for theme in &self.themes {
            if !theme_ids.insert(&theme.id) {
                return Err(DocumentError::Duplicate);
            }
        }
        Ok((validator.bindings, validator.required_client))
    }

    fn validate_encoded_budget(&self) -> Result<(), DocumentError> {
        super::budget::encoded_bytes(self, MAX_DOCUMENT_BYTES)
            .map(|_encoded_bytes| ())
            .map_err(|error| match error {
                super::budget::EncodingError::Exceeded => DocumentError::Bytes,
                super::budget::EncodingError::Encode(cause) => DocumentError::Encode(cause),
            })
    }
}

impl<'de> Deserialize<'de> for ClientDocument {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Generic serde callers (including a containing helper/RPC envelope)
        // must get the same safe boundary. The named byte decoder above retains
        // the concrete sealed parser cause and checks the pre-allocation cap.
        let input = DocumentInput::deserialize(deserializer).map_err(|_untrusted| {
            serde::de::Error::custom("invalid plugin declaration structure")
        })?;
        Self::try_from(input).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<DocumentInput> for ClientDocument {
    type Error = DocumentError;
    fn try_from(value: DocumentInput) -> Result<Self, Self::Error> {
        Self::new(value.methods, value.contributions, value.themes)
    }
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
