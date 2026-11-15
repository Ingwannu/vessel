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
pub(crate) mod import;
pub(crate) mod inbox;
pub(crate) mod layout;
pub(crate) mod pair;
pub(crate) mod palette;
pub(crate) mod pulls;
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod shortcuts;
pub(crate) mod sidebar;
pub(crate) mod terminal;
pub(crate) mod toasts;
pub(crate) mod workspace;

use vessel_app_kit::ui::{Context, Window};
use vessel_proto::{EventRecord, SessionId};

use crate::shell::Shell;

/// Every area's state.
pub(crate) struct Areas {
    pub(crate) import: import::ImportArea,
    pub(crate) inbox: inbox::InboxArea,
    pub(crate) about: about::AboutState,
    pub(crate) draft: draft::Draft,
    pub(crate) session: session::SessionView,
    pub(crate) pair: pair::Pairing,
    pub(crate) conversation: conversation::ConversationArea,
    pub(crate) settings: settings::SettingsState,
    pub(crate) workspace: workspace::Workspace,
    pub(crate) git: git::GitArea,
    pub(crate) layout: layout::LayoutArea,
    // Sole future resource owner; legacy globals remain product-only until cutover.
    #[cfg(test)]
    pub(crate) session_residents: layout::residents::SessionResidents,
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
    /// A terminal session's screen (ADR 0073).
    pub(crate) terminal: terminal::TerminalArea,
}

impl Areas {
    pub(crate) fn new(
        general: settings::PreparedGeneral,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        let mut areas = Self {
            import: import::ImportArea::new(window, cx),
            inbox: inbox::InboxArea::new(window, cx),
            about: about::AboutState::new(cx),
            draft: draft::Draft::new(window, cx),
            session: session::SessionView::new(window, cx),
            pair: pair::Pairing::new(window, cx),
            conversation: conversation::ConversationArea::new(cx),
            settings: settings::SettingsState::new(general, window, cx),
            workspace: workspace::Workspace::new(window, cx),
            git: git::GitArea::new(window, cx),
            layout: layout::LayoutArea::new(cx),
            #[cfg(test)]
            session_residents: layout::residents::SessionResidents::new(),
            pulls: pulls::PullsArea::new(window, cx),
            board: board::BoardArea::new(window, cx),
            palette: palette::PaletteArea::new(window, cx),
            shortcuts: shortcuts::ShortcutListener::new(cx),
            sidebar: sidebar::SidebarState::new(window, cx),
            toasts: vessel_app_view::toasts::Toasts::default(),
            composer: composer::ComposerArea::new(cx),
            regions: crate::region::Regions::default(),
            terminal: terminal::TerminalArea::new(cx),
        };
        // Track P: the composers' drafts read back, and kept as typed.
        composer::keep_drafts(&mut areas, window, cx);
        inbox::start(&areas, window, cx);
        areas
    }
}

/// Whether a batch of records needs the shell drawn again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Redraw {
    /// Something the shell draws may have changed.
    Shell,
    /// Only the shown terminal's output: its grid view, which the terminal
    /// area notifies by itself, is all that changed (ADR 0073).
    GridAlone,
}

impl Shell {
    /// A batch of the followed session's records: the session area, the
    /// conversation and the terminal take it, and the session's reported
    /// `/` commands are kept for its agent (track P). Here, not in
    /// `shell.rs`, which is at its 500 lines.
    pub(crate) fn link_records(
        &mut self,
        session: &SessionId,
        records: &[EventRecord],
        cx: &mut Context<'_, Self>,
    ) -> Redraw {
        self.records(session, records, cx);
        // Track P: a session's reported `/` commands kept for its agent,
        // and a composer's files waiting on their prompt's records.
        self.offers_commands_seen();
        self.composer_files_seen(cx);
        if self.terminal_output_only(session, records) {
            Redraw::GridAlone
        } else {
            Redraw::Shell
        }
    }
}
