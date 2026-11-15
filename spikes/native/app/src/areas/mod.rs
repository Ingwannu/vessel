//! The app's areas, one module each, and the state each keeps. A worker
//! adding an area adds its module here and its state to [`Areas`], its
//! route in `routes.rs`, and touches nothing else of another area's.
//!
//! An area's module holds its state type, its `impl Shell` block for what
//! a person does there (methods on the shell, so a click can reach the
//! link and the fleet), and its `render`. What a screen means lives in the
//! `view` crate; what it asks of nodes in `link`; its words in `view`'s
//! text table; its controls in `kit`.

pub(crate) mod about;
pub(crate) mod archive;
pub(crate) mod board;
pub(crate) mod common;
pub(crate) mod composer;
pub(crate) mod conversation;
pub(crate) mod draft;
pub(crate) mod edge;
pub(crate) mod git;
pub(crate) mod pair;
pub(crate) mod palette;
pub(crate) mod pulls;
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod shortcuts;
pub(crate) mod sidebar;
pub(crate) mod toasts;
pub(crate) mod workspace;

use vessel_app_kit::ui::{Context, Window};

use crate::shell::Shell;

/// Every area's state.
pub(crate) struct Areas {
    pub(crate) about: about::AboutState,
    pub(crate) draft: draft::Draft,
    pub(crate) session: session::SessionView,
    pub(crate) pair: pair::Pairing,
    pub(crate) conversation: conversation::ConversationArea,
    pub(crate) settings: settings::SettingsState,
    pub(crate) workspace: workspace::Workspace,
    pub(crate) git: git::GitArea,
    pub(crate) pulls: pulls::PullsArea,
    pub(crate) board: board::BoardArea,
    pub(crate) palette: palette::PaletteArea,
    pub(crate) shortcuts: shortcuts::ShortcutListener,
    pub(crate) sidebar: sidebar::SidebarState,
    /// The toasts on screen.
    pub(crate) toasts: vessel_app_view::toasts::Toasts,
    /// Track P's: a running session's option changes in flight.
    pub(crate) composer: composer::ComposerArea,
    /// The parts drawn as views of their own (`crate::region`).
    pub(crate) regions: crate::region::Regions,
}

impl Areas {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let mut areas = Self {
            about: about::AboutState::new(cx),
            draft: draft::Draft::new(window, cx),
            session: session::SessionView::new(window, cx),
            pair: pair::Pairing::new(window, cx),
            conversation: conversation::ConversationArea::new(cx),
            settings: settings::SettingsState::new(window, cx),
            workspace: workspace::Workspace::new(window, cx),
            git: git::GitArea::new(window, cx),
            pulls: pulls::PullsArea::new(window, cx),
            board: board::BoardArea::new(window, cx),
            palette: palette::PaletteArea::new(window, cx),
            shortcuts: shortcuts::ShortcutListener::new(cx),
            sidebar: sidebar::SidebarState::new(window, cx),
            toasts: vessel_app_view::toasts::Toasts::default(),
            composer: composer::ComposerArea::new(cx),
            regions: crate::region::Regions::default(),
        };
        // Track P: the composers' drafts read back, and kept as typed.
        composer::keep_drafts(&mut areas, window, cx);
        areas
    }
}
