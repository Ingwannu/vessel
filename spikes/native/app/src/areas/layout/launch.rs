//! What each launch is called, its icon, whether it can start today and how
//! it starts: one table for the pane header's "+" menu (TRACKER 776) and the
//! New tab panel (TRACKER 336), as Paseo feeds both from
//! `useWorkspaceTabLaunchCatalog`.
//!
//! Agent and Terminal start today. Agent opens the new-agent page, because
//! a workspace draft tab cannot be created outside tests yet (W2's seed
//! bridge); Terminal opens a shell as a tab (TRACKER 481). The other
//! launches are listed disabled, as Paseo's `item.disabled`, until their
//! panels render in a pane: Changes, Diff and Files (the explorer panels),
//! Browser (row 509) and Pull request. Change a launch here, and both
//! surfaces follow.
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{Availability, Launch, TabId};

/// Which launches start today (see the module's note).
pub(super) const fn availability(launch: Launch) -> Availability {
    match launch {
        Launch::Agent | Launch::Terminal => Availability::Ready,
        Launch::Changes | Launch::Diff | Launch::Files | Launch::Browser | Launch::PullRequest => {
            Availability::Disabled
        }
    }
}

/// Paseo's launch item words (`workspace.tabs.fallback.*` and the panels'
/// labels).
pub(super) fn label(shell: &Shell, launch: Launch) -> &str {
    let words = shell.words.layout();
    match launch {
        Launch::Agent => words.launch_agent(),
        Launch::Terminal => words.launch_terminal(),
        Launch::Changes => words.changes(),
        Launch::Diff => words.launch_diff(),
        Launch::Files => words.files(),
        Launch::Browser => words.launch_browser(),
        Launch::PullRequest => words.launch_pull_request(),
    }
}

/// Paseo's launch item icons (`SquarePen`, `SquareTerminal`, the diff
/// panels' `FileDiff`, `Files`, `Globe`, `GitPullRequest`).
pub(super) const fn glyph(launch: Launch) -> Glyph {
    match launch {
        Launch::Agent => Glyph::SquarePen,
        Launch::Terminal => Glyph::SquareTerminal,
        Launch::Changes | Launch::Diff => Glyph::FileDiff,
        Launch::Files => Glyph::Files,
        Launch::Browser => Glyph::Globe,
        Launch::PullRequest => Glyph::PullRequest,
    }
}

/// Start `launch`, in place of the New tab tab `replaces` when it was
/// chosen in one (Paseo's `replace` destination); a launch that cannot
/// start yet does nothing.
pub(super) fn run(
    shell: &mut Shell,
    launch: Launch,
    replaces: Option<TabId>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    match launch {
        Launch::Agent => shell.go(Route::Draft, window, cx),
        Launch::Terminal => shell.terminal_tab_new(replaces, window, cx),
        Launch::Changes | Launch::Diff | Launch::Files | Launch::Browser | Launch::PullRequest => {}
    }
}
