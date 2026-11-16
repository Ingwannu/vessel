//! Retained core fallbacks. Feature handlers register at higher priority and
//! may handle workspace/composer/desktop commands before these local actions.

use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::prefs::PreferenceChange;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::Action;

impl Shell {
    /// Do what `action` says; whether it did anything (else the key goes
    /// on to the field and the controls). The exhaustive dispatch stays
    /// in one table so adding an action cannot leave a handler implicit.
    pub(super) fn run_shortcut(
        &mut self,
        action: Action,
        _index: Option<u8>,
        repeat: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match action {
            Action::Palette => self.toggle_palette(window, cx),
            Action::Help => self.toggle_shortcuts_help(window, cx),
            Action::ToggleSidebar => self.toggle_sidebar(window, cx),
            Action::Settings => {
                if self.route == Route::Settings {
                    self.settings_back(false, window, cx);
                } else {
                    self.open_settings(None, window, cx);
                }
            }
            Action::CycleTheme => {
                let next = self.prefs.settings.appearance.theme.next();
                self.prefer(&PreferenceChange::Theme(next), window, cx);
            }
            Action::GoBoard => self.go(Route::Board, window, cx),
            Action::GoMachines => self.open_settings(Some(Section::Overview), window, cx),
            Action::GoSync => self.open_settings(Some(Section::Sync), window, cx),
            Action::GoDevices => self.open_settings(Some(Section::Devices), window, cx),
            Action::Find => {
                if self.route != Route::Session {
                    return false;
                }
                self.conversation_find(window, cx);
            }
            Action::ToggleInspector => {
                if self.route != Route::Session {
                    return false;
                }
                self.toggle_code(window, cx);
            }
            // GOAT: only in a session, never on a draft, and never for
            // the system's repeat of a held key (one message per press).
            Action::SteerQueued => {
                return !repeat && self.route == Route::Session && self.conversation_steer_next();
            }
            Action::EditQueued => {
                return self.route == Route::Session
                    && self.areas.session.composer.at_start(window, cx)
                    && self.conversation_edit_latest_queued(window, cx);
            }
            Action::NewSession
            | Action::FocusComposer
            | Action::JumpSession
            | Action::PreviousSession
            | Action::NextSession
            | Action::SearchFiles
            | Action::NewWindow
            | Action::CloseWindow
            | Action::Fullscreen
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ActualSize
            | Action::OpenProject
            | Action::PinWorkspace
            | Action::ArchiveWorkspace
            | Action::ToggleSidebars
            | Action::NewTab
            | Action::TabAgent
            | Action::NewTerminal
            | Action::TabBrowser
            | Action::TabChanges
            | Action::TabFiles
            | Action::CloseTab
            | Action::JumpTab
            | Action::PreviousTab
            | Action::NextTab
            | Action::SplitRight
            | Action::SplitDown
            | Action::FocusLeft
            | Action::FocusRight
            | Action::FocusUp
            | Action::FocusDown
            | Action::MoveTabLeft
            | Action::MoveTabRight
            | Action::MoveTabUp
            | Action::MoveTabDown
            | Action::ClosePane
            | Action::ToggleFocus
            | Action::CycleMode
            | Action::ToggleVoice
            | Action::ToggleDictation
            | Action::Interrupt
            | Action::MuteVoice
            | Action::ConfirmDictation => return false,
        }
        true
    }

    /// The machine the window shows: the open session's, or the one the
    /// board is scoped to (the page's `machineInView`).
    pub(in crate::areas) fn machine_in_view(&self) -> Option<String> {
        match self.route {
            Route::Session => self
                .areas
                .session
                .open
                .as_ref()
                .map(|open| open.machine.clone()),
            Route::Board | Route::Archive => self.board_scope().map(str::to_owned),
            Route::NewWorkspace | Route::Draft | Route::Pair | Route::Settings => None,
        }
    }
}
