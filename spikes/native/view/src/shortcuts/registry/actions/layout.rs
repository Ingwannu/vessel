//! The pane owner defines operations and index conventions. This is only the
//! closed translation from canonical keyboard intent into that shared API.
use super::Action;
use crate::layout::{CloseScope, Direction, LayoutAction, Position, TabIndex};
use crate::shortcuts::engine::ShortcutIndex;

impl Action {
    /// Translate a pane/tab shortcut without creating resources or resolving a
    /// workspace. Missing index payloads remain unhandled, not a guessed tab.
    #[must_use]
    pub fn layout_action(self, index: Option<ShortcutIndex>) -> Option<LayoutAction> {
        Some(match self {
            Self::NewTab => LayoutAction::NewTab,
            Self::CloseTab => LayoutAction::Close(CloseScope::Current),
            Self::JumpTab => {
                LayoutAction::JumpTab(TabIndex::new(usize::from(index?.get()).checked_sub(1)?))
            }
            Self::PreviousTab => LayoutAction::StepTab(Direction::Left),
            Self::NextTab => LayoutAction::StepTab(Direction::Right),
            Self::SplitRight => LayoutAction::Split(Position::Right),
            Self::SplitDown => LayoutAction::Split(Position::Bottom),
            Self::FocusLeft => LayoutAction::Focus(Direction::Left),
            Self::FocusRight => LayoutAction::Focus(Direction::Right),
            Self::FocusUp => LayoutAction::Focus(Direction::Up),
            Self::FocusDown => LayoutAction::Focus(Direction::Down),
            Self::MoveTabLeft => LayoutAction::Move(Direction::Left),
            Self::MoveTabRight => LayoutAction::Move(Direction::Right),
            Self::MoveTabUp => LayoutAction::Move(Direction::Up),
            Self::MoveTabDown => LayoutAction::Move(Direction::Down),
            Self::ClosePane => LayoutAction::ClosePane,
            Self::ToggleFocus => LayoutAction::ToggleFocus,
            Self::Palette
            | Self::SearchFiles
            | Self::Help
            | Self::Settings
            | Self::CycleTheme
            | Self::NewWindow
            | Self::CloseWindow
            | Self::Fullscreen
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ActualSize
            | Self::OpenProject
            | Self::NewSession
            | Self::JumpSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleSidebars
            | Self::FocusComposer
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_digits_are_one_based_while_pane_indices_are_zero_based() {
        assert_eq!(
            Action::JumpTab.layout_action(ShortcutIndex::new(1)),
            Some(LayoutAction::JumpTab(TabIndex::new(0)))
        );
        assert_eq!(
            Action::JumpTab.layout_action(ShortcutIndex::new(9)),
            Some(LayoutAction::JumpTab(TabIndex::new(8)))
        );
        assert_eq!(Action::JumpTab.layout_action(None), None);
        assert_eq!(
            Action::NewTab.layout_action(None),
            Some(LayoutAction::NewTab)
        );
    }
}
