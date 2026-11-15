//! Closed typed adapter projections for feature-owned handlers.

use super::Action;

impl Action {
    /// Typed feature adapter, shared by keyboard and command-center dispatch.
    #[must_use]
    pub const fn composer_action(self) -> Option<crate::shortcuts::composer::ComposerAction> {
        match self {
            Self::FocusComposer => Some(crate::shortcuts::composer::ComposerAction::Focus),
            Self::Interrupt => Some(crate::shortcuts::composer::ComposerAction::Interrupt),
            Self::CycleMode => Some(crate::shortcuts::composer::ComposerAction::ModeCycle),
            Self::ToggleDictation => {
                Some(crate::shortcuts::composer::ComposerAction::DictationToggle)
            }
            Self::ConfirmDictation => {
                Some(crate::shortcuts::composer::ComposerAction::DictationConfirm)
            }
            Self::ToggleVoice => Some(crate::shortcuts::composer::ComposerAction::VoiceToggle),
            Self::MuteVoice => Some(crate::shortcuts::composer::ComposerAction::VoiceMuteToggle),
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
            | Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::JumpTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane
            | Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleFocus
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => None,
        }
    }

    /// Typed feature adapter, shared by keyboard and command-center dispatch.
    #[must_use]
    pub const fn desktop_action(self) -> Option<crate::shortcuts::desktop::DesktopAction> {
        match self {
            Self::NewWindow => Some(crate::shortcuts::desktop::DesktopAction::NewWindow),
            Self::CloseWindow => Some(crate::shortcuts::desktop::DesktopAction::CloseWindow),
            Self::Fullscreen => Some(crate::shortcuts::desktop::DesktopAction::Fullscreen),
            Self::ZoomIn => Some(crate::shortcuts::desktop::DesktopAction::ZoomIn),
            Self::ZoomOut => Some(crate::shortcuts::desktop::DesktopAction::ZoomOut),
            Self::ActualSize => Some(crate::shortcuts::desktop::DesktopAction::ActualSize),
            Self::Palette
            | Self::SearchFiles
            | Self::Help
            | Self::Settings
            | Self::CycleTheme
            | Self::OpenProject
            | Self::NewSession
            | Self::JumpSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::ToggleSidebars
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::JumpTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane
            | Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleSidebars
            | Self::ToggleFocus
            | Self::FocusComposer
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => None,
        }
    }

    /// Which feature context receives an already-resolved action.
    #[must_use]
    pub const fn scope(self) -> crate::shortcuts::dispatcher::ActionScope {
        match self {
            Self::FocusComposer
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::MuteVoice
            | Self::ConfirmDictation
            | Self::SteerQueued
            | Self::EditQueued => crate::shortcuts::dispatcher::ActionScope::MessageInput,
            Self::NewSession
            | Self::JumpSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::ToggleSidebars
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::ToggleInspector => crate::shortcuts::dispatcher::ActionScope::Sidebar,
            Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::JumpTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane
            | Self::ToggleFocus
            | Self::Find => crate::shortcuts::dispatcher::ActionScope::Workspace,
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
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::ToggleSidebar
            | Self::Interrupt => crate::shortcuts::dispatcher::ActionScope::Global,
        }
    }

    /// Source `sidebar.*` toggles leave focus mode only after being handled.
    /// Sidebar-scope workspace creation/pin/archive are distinct intentions.
    #[must_use]
    pub const fn exits_focus_mode(self) -> bool {
        matches!(
            self,
            Self::ToggleSidebar | Self::ToggleInspector | Self::ToggleSidebars
        )
    }

    /// Retained core handlers are fallbacks; feature owners may register a
    /// higher-priority active handler without a second keyboard listener.
    #[must_use]
    pub const fn has_builtin_fallback(self) -> bool {
        match self {
            Self::Palette
            | Self::Help
            | Self::Settings
            | Self::CycleTheme
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleSidebars
            | Self::FocusComposer
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => true,
            Self::NewSession
            | Self::JumpSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::SearchFiles
            | Self::NewWindow
            | Self::CloseWindow
            | Self::Fullscreen
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ActualSize
            | Self::OpenProject
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::JumpTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane
            | Self::ToggleFocus
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation => false,
        }
    }
}
