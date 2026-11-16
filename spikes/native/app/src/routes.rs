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
    /// Chronological session history, including archived sessions.
    Sessions,
    /// Ended and archived sessions.
    Archive,
    /// One session's conversation.
    Session,
    /// Adding a machine by pairing.
    Pair,
    /// Settings: its sections listed in the sidebar's place, the chosen
    /// one beside them (`areas/settings`).
    Settings,
}

impl Route {
    /// The routes the sidebar lists at its top, in order (a session is
    /// reached from its row, pairing from the foot).
    pub(crate) const NAV: [Self; 4] = [Self::Draft, Self::Board, Self::Sessions, Self::Archive];

    /// Its name in the sidebar.
    pub(crate) const fn label(self, t: &vessel_app_view::Text) -> &'static str {
        match self {
            Self::Draft => t.new_session,
            Self::Board => t.board,
            Self::Sessions => t.lang.sidebar().sessions,
            Self::Archive => t.archive,
            Self::Session => t.app,
            Self::Pair => t.add_machine,
            Self::Settings => t.lang.settings().settings,
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
        // The archive is a dialog over the board.
        Route::Board | Route::Archive => areas::board::render(shell, tokens, window, cx),
        Route::Sessions => areas::sessions::render(shell, tokens, window, cx),
        Route::Session => areas::session::render(shell, tokens, window, cx).into_any_element(),
        Route::Pair => areas::pair::render(shell, tokens, window, cx).into_any_element(),
        Route::Settings => areas::settings::render(shell, tokens, window, cx).into_any_element(),
    }
}

/// What stands in the sidebar's place on a wide window: its content kept
/// between frames as a region (`crate::region`), in its resizable frame.
pub(crate) fn sidebar(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let region = shell.areas.regions.sidebar.as_ref();
    let inner = crate::region::place(region, sidebar_inner, shell, tokens, window, cx);
    areas::sidebar::frame(shell, inner, tokens, window, cx).into_any_element()
}

/// The sidebar's content beside the shell's route.
pub(crate) fn sidebar_inner(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    match shell.route {
        Route::Settings => areas::settings::sidebar(shell, tokens, cx).into_any_element(),
        Route::Draft
        | Route::Board
        | Route::Sessions
        | Route::Archive
        | Route::Session
        | Route::Pair => {
            areas::sidebar::render(shell, tokens, false, window, cx).into_any_element()
        }
    }
}

/// What arriving at `route` does (focus its field, settle its choices).
pub(crate) fn arrive(
    route: Route,
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    areas::shortcuts::navigation_changed(shell);
    match route {
        Route::Draft => areas::draft::arrive(shell, window, cx),
        Route::Pair => areas::pair::arrive(shell, window, cx),
        Route::Archive => areas::archive::arrive(shell, window, cx),
        Route::Sessions => areas::sessions::arrive(shell, window, cx),
        Route::Board | Route::Session | Route::Settings => {}
    }
}
