//! Which screen draws each of Settings' sections, and so which sections
//! this build lists: one registration per section.
//!
//! [Decision Log]
//! - Purpose: Paseo's list has 26 sections (`view::settings::Section`);
//!   a build draws some of them. A section with nothing behind it is not
//!   listed at all (the coordinator's rule since 0.0.0.0.7: nothing shown
//!   may do nothing, no empty promises).
//! - Before: two tables kept in step by hand (`drawn` for the list,
//!   `content` for the page) and a sentence for a section listed but not
//!   drawn.
//! - Choice: one table, [`page`]: a section is listed exactly when it has
//!   a renderer, so the list and the pages cannot disagree and no
//!   placeholder page exists. Another track's section is one line here
//!   and a module of its own, as a route is one line in `routes.rs`.
//! - Also here: which surface this build is (Paseo lists five sections on
//!   its desktop app only and one on the web only), and whether a machine
//!   section can be listed (Paseo lists the host group only while a host
//!   exists; without one its place is "Add host").

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_view::settings::{Section, Surface};

use super::{
    agents, appearance, chat, choices, connections, devices, diagnostics, general, host_agents,
    overview, sync,
};
use crate::shell::Shell;

/// The surface this build runs on: a phone build is Android's, a browser
/// build WebAssembly; every other build is the desktop app.
pub(crate) const SURFACE: Surface = if cfg!(target_os = "android") {
    Surface::Phone
} else if cfg!(target_arch = "wasm32") {
    Surface::Browser
} else {
    Surface::Desktop
};

/// A section's page: the shell, the tokens, whether the window is a
/// phone's.
type Draw = fn(&Shell, &Tokens, bool, &mut Window, &mut Context<'_, Shell>) -> AnyElement;

/// Which screen draws `section`'s page; `None`: nothing is behind it in
/// this build, so it is not listed.
fn page(section: Section) -> Option<Draw> {
    match section {
        Section::General => {
            Some(|shell, tokens, phone, _, cx| general::render(shell, tokens, phone, cx))
        }
        Section::Appearance => {
            Some(|shell, tokens, phone, _, cx| appearance::render(shell, tokens, phone, cx))
        }
        Section::Chat => Some(|shell, tokens, phone, _, cx| chat::render(shell, tokens, phone, cx)),
        Section::Diagnostics => {
            Some(|shell, tokens, phone, _, cx| diagnostics::render(shell, tokens, phone, cx))
        }
        Section::About => Some(|shell, tokens, phone, _, cx| {
            crate::areas::about::content(shell, tokens, phone, cx)
        }),
        Section::Projects => Some(|shell, tokens, phone, _, cx| {
            crate::areas::workspace::projects_content(shell, tokens, phone, cx)
        }),
        Section::Devices => {
            Some(|shell, tokens, phone, _, cx| devices::render(shell, tokens, phone, cx))
        }
        Section::Overview => {
            Some(|shell, tokens, phone, _, cx| overview::render(shell, tokens, phone, cx))
        }
        Section::Connections => {
            Some(|shell, tokens, phone, _, _| connections::render(shell, tokens, phone))
        }
        Section::Agents => {
            Some(|shell, tokens, phone, _, cx| host_agents::render(shell, tokens, phone, cx))
        }
        Section::Providers => Some(agents::render),
        Section::Sync => {
            Some(|shell, tokens, phone, window, cx| sync::render(shell, tokens, phone, window, cx))
        }
        Section::Usage => Some(|shell, tokens, phone, _, cx| {
            crate::areas::usage::render(shell, tokens, phone, cx)
        }),
        Section::Shortcuts => Some(crate::areas::shortcuts::page::render),
        // Not drawn yet. Sidebar and Notifications wait on S3's sidebar
        // reading its items and on notifications being delivered; the rest
        // have nothing behind them in Vessel yet.
        Section::Sidebar
        | Section::Terminal
        | Section::Browser
        | Section::Editor
        | Section::Files
        | Section::Integrations
        | Section::Notifications
        | Section::Permissions
        | Section::Metadata
        | Section::Voice
        | Section::Workspaces
        | Section::Terminals
        | Section::Plugins
        | Section::Models => None,
    }
}

/// Whether this build draws `section` on this surface.
pub(crate) fn drawn(section: Section) -> bool {
    section.shown() && section.available_on(SURFACE) && page(section).is_some()
}

/// Whether Settings lists `section` now: drawn, and a machine section
/// only while there is a machine to be about.
pub(crate) fn listed(section: Section, shell: &Shell) -> bool {
    drawn(section) && (!section.per_machine() || choices::chosen_machine(shell).is_some())
}

/// `section`'s page, if it is drawn.
pub(crate) fn draw(
    section: Section,
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    page(section)
        .filter(|_| drawn(section))
        .map(|draw| draw(shell, tokens, phone, window, cx))
}
