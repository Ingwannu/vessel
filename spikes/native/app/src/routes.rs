//! The one place a screen is registered: its route, what draws it, and
//! whether the sidebar lists it. Adding an area is a module in `areas/`,
//! its state in `areas::Areas`, and one line in each match below.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, Window};

use crate::areas;
use crate::shell::Shell;

/// What fills the frame beside the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// A new conversation: the composer, with its pickers.
    Draft,
    /// The board's lanes.
    Board,
    /// Ended and archived sessions.
    Archive,
    /// One session's conversation.
    Session,
    /// Adding a machine by pairing.
    Pair,
    /// The app's version and its updates (Zed's About and its update
    /// settings; the start of a Settings screen).
    About,
}

impl Route {
    /// The routes the sidebar lists at its top, in order (a session is
    /// reached from its row, pairing from the foot).
    pub(crate) const NAV: [Self; 3] = [Self::Draft, Self::Board, Self::Archive];

    /// Its name in the sidebar.
    pub(crate) const fn label(self, t: &vessel_app_view::Text) -> &'static str {
        match self {
            Self::Draft => t.new_session,
            Self::Board => t.board,
            Self::Archive => t.archive,
            Self::Session => t.app,
            Self::Pair => t.add_machine,
            Self::About => t.lang.update().about,
        }
    }
}

/// Draw `route`.
pub(crate) fn render(
    route: Route,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    match route {
        Route::Draft => areas::draft::render(shell, tokens, window, cx).into_any_element(),
        Route::Board => areas::board::render(shell, tokens, cx).into_any_element(),
        Route::Archive => areas::archive::render(shell, tokens, cx).into_any_element(),
        Route::Session => areas::session::render(shell, tokens, window, cx).into_any_element(),
        Route::Pair => areas::pair::render(shell, tokens, window, cx).into_any_element(),
        Route::About => areas::about::render(shell, tokens, cx).into_any_element(),
    }
}

/// What arriving at `route` does (focus its field, settle its choices).
pub(crate) fn arrive(
    route: Route,
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    match route {
        Route::Draft => areas::draft::arrive(shell, window, cx),
        Route::Pair => areas::pair::arrive(shell, window, cx),
        Route::Board | Route::Archive | Route::Session | Route::About => {}
    }
}
