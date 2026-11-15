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
        Action::SearchFiles => words.label_search_files(),
        Action::Help => words.label_help(),
        Action::Settings => words.label_settings(),
        Action::CycleTheme => words.label_cycle_theme(),
        Action::NewWindow => words.label_new_window(),
        Action::CloseWindow => words.label_close_window(),
        Action::Fullscreen => words.label_fullscreen(),
        Action::ZoomIn => words.label_zoom_in(),
        Action::ZoomOut => words.label_zoom_out(),
        Action::ActualSize => words.label_actual_size(),
        Action::OpenProject => words.label_open_project(),
        Action::NewSession => words.label_new_session(),
        Action::JumpSession => words.label_jump_session(),
        Action::PreviousSession => words.label_previous_session(),
        Action::NextSession => words.label_next_session(),
        Action::PinWorkspace => words.label_pin_workspace(),
        Action::ArchiveWorkspace => words.label_archive_workspace(),
        Action::GoBoard => words.label_go_board(),
        Action::GoMachines => words.label_go_machines(),
        Action::GoSync => words.label_go_sync(),
        Action::GoDevices => words.label_go_devices(),
        Action::NewTab => words.label_new_tab(),
        Action::TabAgent => words.label_tab_agent(),
        Action::NewTerminal => words.label_new_terminal(),
        Action::TabBrowser => words.label_tab_browser(),
        Action::TabChanges => words.label_tab_changes(),
        Action::TabFiles => words.label_tab_files(),
        Action::CloseTab => words.label_close_tab(),
        Action::JumpTab => words.label_jump_tab(),
        Action::PreviousTab => words.label_previous_tab(),
        Action::NextTab => words.label_next_tab(),
        Action::SplitRight => words.label_split_right(),
        Action::SplitDown => words.label_split_down(),
        Action::FocusLeft => words.label_focus_left(),
        Action::FocusRight => words.label_focus_right(),
        Action::FocusUp => words.label_focus_up(),
        Action::FocusDown => words.label_focus_down(),
        Action::MoveTabLeft => words.label_move_tab_left(),
        Action::MoveTabRight => words.label_move_tab_right(),
        Action::MoveTabUp => words.label_move_tab_up(),
        Action::MoveTabDown => words.label_move_tab_down(),
        Action::ClosePane => words.label_close_pane(),
        Action::ToggleSidebar => words.label_toggle_sidebar(),
        Action::ToggleInspector => words.label_toggle_inspector(),
        Action::ToggleSidebars => words.label_toggle_sidebars(),
        Action::ToggleFocus => words.label_toggle_focus(),
        Action::FocusComposer => words.label_focus_composer(),
        Action::CycleMode => words.label_cycle_mode(),
        Action::ToggleVoice => words.label_toggle_voice(),
        Action::ToggleDictation => words.label_toggle_dictation(),
        Action::Interrupt => words.label_interrupt(),
        Action::MuteVoice => words.label_mute_voice(),
        Action::ConfirmDictation => words.label_confirm_dictation(),
        Action::Find => words.label_find(),
        Action::SteerQueued => words.label_steer_queued(),
        Action::EditQueued => words.label_edit_queued(),
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
        ShortcutGroup::TabsPanes => words.group_tabs_panes(),
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
