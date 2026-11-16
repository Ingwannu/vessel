//! A client document becomes visible only after its complete graph validates.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::declarations::{
    Action, Button, ButtonBehavior, Contribution, ContributionContent, ControlContent, MenuEntry,
    Surface,
};
use super::{ContributionId, PluginJsonError, RpcName, ThemeContribution};

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
        let graph = Self {
            methods,
            contributions,
            themes,
        };
        graph.validate()?;
        graph.validate_encoded_budget()?;
        Ok(graph)
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

    fn validate(&self) -> Result<(), DocumentError> {
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
        let mut validator = Validator {
            methods,
            screens,
            count: 0,
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
        Ok(())
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

struct Validator<'a> {
    methods: BTreeSet<&'a RpcName>,
    screens: BTreeSet<&'a ContributionId>,
    count: usize,
}

impl Validator<'_> {
    fn contribution(&mut self, content: &ContributionContent) -> Result<(), DocumentError> {
        let mut ids = BTreeSet::new();
        match content {
            ContributionContent::Screen { surface }
            | ContributionContent::SettingsScreen { surface }
            | ContributionContent::TimelineRenderer { surface, .. } => {
                self.surface(surface, 0, &mut ids)
            }
            ContributionContent::Panel {
                locations, surface, ..
            } => {
                Self::panel_locations(locations)?;
                self.surface(surface, 0, &mut ids)
            }
            ContributionContent::FilePanel {
                extensions,
                filenames,
                surface,
            } => {
                self.file_selectors(extensions, filenames)?;
                self.surface(surface, 0, &mut ids)
            }
            ContributionContent::Sidebar { screen, .. } => self.screen(screen),
            ContributionContent::Command {
                action, keywords, ..
            } => {
                self.add_count(keywords.len())?;
                self.action(action)
            }
            ContributionContent::SlashCommand { method, .. }
            | ContributionContent::AttachmentSource { search: method } => self.method(method),
            ContributionContent::HeaderButton { button, .. }
            | ContributionContent::ComposerPill { button, .. } => self.button(button, 0, &mut ids),
        }
    }

    fn panel_locations(locations: &[super::PanelLocation]) -> Result<(), DocumentError> {
        if locations.is_empty() {
            return Err(DocumentError::EmptyOptions);
        }
        if locations.iter().collect::<BTreeSet<_>>().len() != locations.len() {
            return Err(DocumentError::Duplicate);
        }
        Ok(())
    }

    fn file_selectors(
        &mut self,
        extensions: &[super::FileExtension],
        filenames: &[super::FileNamePattern],
    ) -> Result<(), DocumentError> {
        if extensions.is_empty() && filenames.is_empty() {
            return Err(DocumentError::EmptyOptions);
        }
        self.add_count(extensions.len().saturating_add(filenames.len()))?;
        let distinct_extensions = extensions
            .iter()
            .map(|e| e.as_str().to_lowercase())
            .collect::<BTreeSet<_>>();
        let distinct_names = filenames
            .iter()
            .map(super::FileNamePattern::as_str)
            .collect::<BTreeSet<_>>();
        if distinct_extensions.len() != extensions.len() || distinct_names.len() != filenames.len()
        {
            return Err(DocumentError::Duplicate);
        }
        Ok(())
    }

    fn add_count(&mut self, count: usize) -> Result<(), DocumentError> {
        // Saturation makes a theoretical counter overflow fail the same bound.
        self.count = self.count.saturating_add(count);
        if self.count > MAX_CONTROLS {
            return Err(DocumentError::Controls);
        }
        Ok(())
    }

    fn surface<'a>(
        &mut self,
        surface: &'a Surface,
        depth: usize,
        ids: &mut BTreeSet<&'a ContributionId>,
    ) -> Result<(), DocumentError> {
        if depth > MAX_DEPTH {
            return Err(DocumentError::Depth);
        }
        self.add_count(surface.controls.len())?;
        for control in &surface.controls {
            if !ids.insert(&control.id) {
                return Err(DocumentError::Duplicate);
            }
            self.control(&control.content, depth, ids)?;
        }
        Ok(())
    }

    fn control<'a>(
        &mut self,
        content: &'a ControlContent,
        depth: usize,
        ids: &mut BTreeSet<&'a ContributionId>,
    ) -> Result<(), DocumentError> {
        match content {
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Status { .. } => Ok(()),
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. } => {
                self.surface(surface, depth.saturating_add(1), ids)
            }
            ControlContent::Button { button } => self.button(button, depth, ids),
            ControlContent::TextField { submit, .. } | ControlContent::Toggle { submit, .. } => {
                self.method(submit)
            }
            ControlContent::Choice {
                options,
                selected,
                submit,
                ..
            } => {
                self.add_count(options.len())?;
                if options.is_empty() {
                    return Err(DocumentError::EmptyOptions);
                }
                let values: BTreeSet<_> = options.iter().map(|option| &option.id).collect();
                if values.len() != options.len() {
                    return Err(DocumentError::Duplicate);
                }
                if selected.as_ref().is_some_and(|id| !values.contains(id)) {
                    return Err(DocumentError::MissingChoice);
                }
                self.method(submit)
            }
        }
    }

    fn button<'a>(
        &mut self,
        button: &'a Button,
        depth: usize,
        ids: &mut BTreeSet<&'a ContributionId>,
    ) -> Result<(), DocumentError> {
        self.behavior(&button.behavior, depth, ids)
    }

    fn behavior<'a>(
        &mut self,
        behavior: &'a ButtonBehavior,
        depth: usize,
        ids: &mut BTreeSet<&'a ContributionId>,
    ) -> Result<(), DocumentError> {
        if depth > MAX_DEPTH {
            return Err(DocumentError::Depth);
        }
        match behavior {
            ButtonBehavior::Action { action } => self.action(action),
            ButtonBehavior::Menu { items } => {
                if items.is_empty() {
                    return Err(DocumentError::EmptyOptions);
                }
                self.add_count(items.len())?;
                let mut entries = BTreeSet::new();
                for item in items {
                    let id = match item {
                        MenuEntry::Separator { id } => id,
                        MenuEntry::Item { id, behavior, .. } => {
                            self.behavior(behavior, depth.saturating_add(1), ids)?;
                            id
                        }
                    };
                    if !entries.insert(id) {
                        return Err(DocumentError::Duplicate);
                    }
                }
                Ok(())
            }
            ButtonBehavior::Popover { surface } => {
                self.surface(surface, depth.saturating_add(1), ids)
            }
        }
    }

    fn action(&self, action: &Action) -> Result<(), DocumentError> {
        match action {
            Action::OpenScreen { screen } => self.screen(screen),
            Action::Invoke { method, input } => {
                if input.as_text().len() > MAX_ACTION_INPUT_BYTES {
                    return Err(DocumentError::ActionInput);
                }
                self.method(method)
            }
        }
    }

    fn screen(&self, screen: &ContributionId) -> Result<(), DocumentError> {
        if self.screens.contains(screen) {
            Ok(())
        } else {
            Err(DocumentError::MissingScreen)
        }
    }

    fn method(&self, method: &RpcName) -> Result<(), DocumentError> {
        if self.methods.contains(method) {
            Ok(())
        } else {
            Err(DocumentError::MissingMethod)
        }
    }
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
