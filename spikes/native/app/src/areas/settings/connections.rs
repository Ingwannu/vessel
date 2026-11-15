//! Settings › Connections for the chosen machine, as Paseo's
//! `HostConnectionsPage` (`screens/settings/host-page.tsx`): the alert
//! while the machine does not answer (`HostConnectionError`), then
//! "Connections" (`ConnectionsSection`), a card with a row per way the
//! book reaches it (`ConnectionRow`), titled as Paseo's
//! `formatHostConnectionLabel` ("Relay (relay.example:443)").
//!
//! [Decision Log]
//! - Purpose: show how this device reaches each machine, from the book
//!   the CLI and the app share (`link::machines`, the CLI's own book code).
//! - Constraint: a machine's book entry is one address: a websocket with a
//!   token, an `ssh` target, or a paired machine's own addresses and then
//!   its relays. The routes are those, in the order they are tried
//!   (`view::connections`), endpoints only.
//! - Choice: Paseo's section, card and rows, read-only. Paseo's row has a
//!   latency and a ghost "Remove"; the book's actor has no route edit
//!   (rename, default, forget a machine), so a row cannot be removed here:
//!   removing the machine is Overview's "Danger zone".
//! - Not shown, nothing behind them yet: the latency probe (Paseo's "…",
//!   "12 ms", "Timeout"), the remove confirm dialog.
//! - Cost: a paired machine's routes cannot be trimmed from the app; the
//!   CLI pairs the machine again for a new set.

use vessel_app_kit::ui::{AnyElement, Div, IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings};
use vessel_app_view::connections::Route;

use super::{choices, overview};
use crate::shell::Shell;

/// The Connections page.
pub(super) fn render(shell: &Shell, tokens: &Tokens, compact: bool) -> AnyElement {
    let Some(machine) = choices::chosen_machine(shell) else {
        return div().into_any_element();
    };
    // Drawn once the book has answered (a frame or two after the page is
    // shown); a book entry with no route draws no empty card.
    let routes = shell
        .areas
        .settings
        .book
        .entry(&machine.name)
        .map(|entry| entry.routes.as_slice())
        .filter(|routes| !routes.is_empty());
    div()
        .flex()
        .flex_col()
        .children(overview::unreachable(machine, shell, tokens))
        .children(routes.map(|routes| {
            let card = routes
                .iter()
                .enumerate()
                .fold(settings::card(tokens), |card, (index, route)| {
                    card.child(row(shell, route, index == 0, compact, tokens))
                });
            settings::section(shell.t.lang.overview().connections, card, tokens)
        }))
        .into_any_element()
}

/// Paseo's `ConnectionRow`: its title on one line (`numberOfLines={1}`),
/// cut with an ellipsis when it does not fit.
fn row(shell: &Shell, route: &Route, first: bool, compact: bool, tokens: &Tokens) -> Div {
    let title = div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
        .truncate()
        .child(shell.t.lang.overview().route(route));
    settings::row_titled(
        title.into_any_element(),
        None,
        None,
        None,
        first,
        compact,
        tokens,
    )
}
