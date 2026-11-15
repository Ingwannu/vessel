//! Typed Fluent label dispatch for the shortcut registry. Screens never derive
//! translation keys from ids or keep a second English/Korean action-name list.

use super::{Action, ShortcutGroup};
use crate::localization::Translator;

/// Prepared action text, borrowed from the caller's already initialized catalog.
#[must_use]
pub fn action_label(action: Action, catalog: &Translator) -> &str {
    let words = catalog.shortcuts();
    match action {
        Action::Palette => words.label_palette(),
        Action::NewSession => words.label_new_session(),
        Action::Settings => words.label_settings(),
        Action::GoBoard => words.label_go_board(),
        Action::GoMachines => words.label_go_machines(),
        Action::GoSync => words.label_go_sync(),
        Action::GoDevices => words.label_go_devices(),
        Action::PreviousSession => words.label_previous_session(),
        Action::NextSession => words.label_next_session(),
        Action::JumpSession => words.label_jump_session(),
        Action::Find => words.label_find(),
        Action::FocusComposer => words.label_focus_composer(),
        Action::SteerQueued => words.label_steer_queued(),
        Action::EditQueued => words.label_edit_queued(),
        Action::ToggleSidebar => words.label_toggle_sidebar(),
        Action::ToggleInspector => words.label_toggle_inspector(),
    }
}

/// Prepared section text in the registry's help order.
#[must_use]
pub fn group_label(group: ShortcutGroup, catalog: &Translator) -> &str {
    let words = catalog.shortcuts();
    match group {
        ShortcutGroup::General => words.group_general(),
        ShortcutGroup::Navigation => words.group_navigation(),
        ShortcutGroup::Session => words.group_session(),
        ShortcutGroup::Panels => words.group_panels(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::LANGUAGES;

    #[test]
    fn every_shipped_catalog_has_every_typed_shortcut_action_and_group() {
        for locale in LANGUAGES {
            let catalog = Translator::get(*locale).unwrap();
            for action in Action::ALL {
                assert!(!action_label(action, catalog).is_empty());
            }
            for group in ShortcutGroup::ALL {
                assert!(!group_label(group, catalog).is_empty());
            }
        }
    }
}
