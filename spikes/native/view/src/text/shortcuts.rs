//! Compatibility row helpers borrow the prepared Fluent catalog. All action
//! labels, groups and whole refusal/conflict sentences live in shortcuts.ftl;
//! the former 17-action English/Korean index tables are deliberately removed.

use crate::localization::{FormatError, Translator};
use crate::shortcuts::{
    Action, Clash, Platform, Refusal, ShortcutGroup, action_label, group_label,
};

/// Prepared adapter for older pure listing/edit helpers. New screens borrow the
/// generated `Translator::shortcuts()` accessors directly. Construction belongs
/// at the language-selection boundary, never during drawing or matching a key.
#[derive(Debug)]
pub struct ShortcutWords {
    catalog: &'static Translator,
    /// Physical sequence separator; the word comes from the same catalog.
    pub then: String,
}

impl ShortcutWords {
    /// Reuse an explicitly initialized process catalog without any I/O or parse.
    #[must_use]
    pub fn new(catalog: &'static Translator) -> Self {
        Self {
            catalog,
            then: format!(" {} ", catalog.shortcuts().then()),
        }
    }

    /// The canonical translated label, including newly registered action kinds.
    #[must_use]
    pub fn label(&self, action: Action) -> &'static str {
        action_label(action, self.catalog)
    }
    /// A canonical help/Settings section title.
    #[must_use]
    pub fn group(&self, group: ShortcutGroup) -> &'static str {
        group_label(group, self.catalog)
    }
    /// Compatibility refusal sentences use the same typed catalog as Settings.
    #[must_use]
    pub fn refusal(&self, refusal: Refusal) -> &'static str {
        let words = self.catalog.shortcuts();
        match refusal {
            Refusal::NoModifier(Platform::Mac) => words.modifier_mac(),
            Refusal::NoModifier(Platform::Other) => words.modifier_other(),
            Refusal::Kept => words.kept(),
            Refusal::Editing => words.editing(),
        }
    }

    /// Format the complete legacy-helper sentence when capture changes.
    ///
    /// # Errors
    /// Returns a typed formatting failure rather than displaying a raw error.
    pub fn clash(&self, clash: Clash, binding: &str, to: Action) -> Result<String, FormatError> {
        let words = self.catalog.shortcuts();
        match clash {
            Clash::Same(_) => words.already_assigned(binding),
            Clash::Fixed(held) => words.fixed_key(self.label(held), binding),
            Clash::Taken(held) => words.move_key(self.label(held), binding, self.label(to)),
        }
    }
}
