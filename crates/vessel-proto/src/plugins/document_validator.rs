//! The client document's graph check: budgets, identities and references
//! (moved out of `document.rs`, which keeps the shape and its accessors).
//! It also records what a client needs to know beyond validity: the settings
//! documents the controls bind and the oldest client protocol that decodes
//! every kind used (ADR 0115, "Surfaces").

use std::collections::BTreeSet;

use super::DocumentError;
use crate::ProtocolVersion;
use crate::plugins::declarations::{
    Action, Button, ButtonBehavior, ChoiceOption, ContributionContent, ControlContent, MenuEntry,
    Surface,
};
use crate::plugins::{
    ContributionId, MAX_ACTION_INPUT_BYTES, MAX_CONTROLS, MAX_DEPTH, PLUGIN_CATALOG_PROTOCOL,
    PLUGIN_SURFACES_PROTOCOL, RpcName, SettingsId,
};

pub(super) struct Validator<'a> {
    pub(super) methods: BTreeSet<&'a RpcName>,
    pub(super) screens: BTreeSet<&'a ContributionId>,
    pub(super) settings_screens: BTreeSet<&'a ContributionId>,
    pub(super) count: usize,
    /// Settings documents the controls bind.
    pub(super) bindings: BTreeSet<SettingsId>,
    /// The oldest client protocol that decodes every kind seen so far.
    pub(super) required_client: ProtocolVersion,
}

/// The catalog's protocol: every kind that existed when the catalog was served.
pub(super) fn base_protocol() -> ProtocolVersion {
    PLUGIN_CATALOG_PROTOCOL.unwrap_or(crate::PROTOCOL_VERSION)
}

impl Validator<'_> {
    /// A kind added with the surfaces (`card`, the settings controls,
    /// `open_settings`): an older client's decoder refuses it.
    fn newer(&mut self) {
        let surfaces = PLUGIN_SURFACES_PROTOCOL.unwrap_or(crate::PROTOCOL_VERSION);
        self.required_client = self.required_client.max(surfaces);
    }

    fn bind(&mut self, settings: &SettingsId) {
        self.newer();
        let _new = self.bindings.insert(settings.clone());
    }

    /// A choice's options: counted, nonempty and unique.
    fn options<'o>(
        &mut self,
        options: &'o [ChoiceOption],
    ) -> Result<BTreeSet<&'o ContributionId>, DocumentError> {
        self.add_count(options.len())?;
        if options.is_empty() {
            return Err(DocumentError::EmptyOptions);
        }
        let values: BTreeSet<_> = options.iter().map(|option| &option.id).collect();
        if values.len() != options.len() {
            return Err(DocumentError::Duplicate);
        }
        Ok(values)
    }

    pub(super) fn contribution(
        &mut self,
        content: &ContributionContent,
    ) -> Result<(), DocumentError> {
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

    fn panel_locations(locations: &[crate::plugins::PanelLocation]) -> Result<(), DocumentError> {
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
        extensions: &[crate::plugins::FileExtension],
        filenames: &[crate::plugins::FileNamePattern],
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
            .map(crate::plugins::FileNamePattern::as_str)
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
                let values = self.options(options)?;
                if selected.as_ref().is_some_and(|id| !values.contains(id)) {
                    return Err(DocumentError::MissingChoice);
                }
                self.method(submit)
            }
            ControlContent::Card { surface } => {
                self.newer();
                self.surface(surface, depth.saturating_add(1), ids)
            }
            ControlContent::SettingsSwitch { settings, .. }
            | ControlContent::SettingsInput { settings, .. } => {
                self.bind(settings);
                Ok(())
            }
            ControlContent::SettingsSelect {
                settings, options, ..
            } => {
                let _values = self.options(options)?;
                self.bind(settings);
                Ok(())
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

    fn action(&mut self, action: &Action) -> Result<(), DocumentError> {
        match action {
            Action::OpenScreen { screen } => self.screen(screen),
            Action::OpenSettings { screen } => {
                self.newer();
                if self.settings_screens.contains(screen) {
                    Ok(())
                } else {
                    Err(DocumentError::MissingScreen)
                }
            }
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
