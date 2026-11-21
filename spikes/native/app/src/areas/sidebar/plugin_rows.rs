//! The sidebar's plugin rows (ADR 0115, "Surfaces"): Paseo's
//! `addSidebarHeaderItem` and `addSidebarFooterItem` rows
//! (public-docs/plugins/reference.md:718-846), each opening one of its
//! plugin's screens, with Paseo's shared plugin glyph (`Blocks`,
//! `screens/settings/sidebar/sidebar-nav-section.tsx:50-64`).
//!
//! [Decision Log]
//! - Header rows follow the pages at the top of the list; footer rows sit
//!   above the foot. A row declared on several connected machines is drawn
//!   once (the view's `Catalogs::sidebar_rows`). It is active while its
//!   screen is open from any machine.
//! - A device without admin sees the rows too: a screen is drawn read-only
//!   there.

use vessel_app_kit::Tokens;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::plugins::SidebarChoice;
use vessel_proto::plugins::SidebarPlacement;

use super::nav_row;
use crate::routes::Route;
use crate::shell::Shell;

/// The `placement` rows; the footer's in their own block, nothing when
/// there are none.
pub(super) fn rows(
    shell: &Shell,
    placement: SidebarPlacement,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let online: Vec<MachineKey> = shell
        .fleet
        .online()
        .map(|machine| MachineKey::from_book(&machine.name))
        .collect();
    let area = &shell.areas.plugin_screen;
    let current = area.open.as_ref().filter(|_| shell.route == Route::Plugin);
    let choice = SidebarChoice {
        online: &online,
        current,
        remembered: &area.remembered,
    };
    let rows = shell.areas.plugin_catalogs.sidebar_rows(placement, &choice);
    if rows.is_empty() {
        return None;
    }
    let mut block = div().flex().flex_col().gap(px(spacing::S0_5));
    if placement == SidebarPlacement::Footer {
        block = block.flex_none();
    }
    for row in rows {
        let active = current.is_some_and(|open| {
            open.plugin == row.target.plugin && open.screen == row.target.screen
        });
        let target = row.target.clone();
        block = block.child(
            nav_row(
                ElementId::Name(format!("plugin-row-{}", row.key).into()),
                row.title,
                Some(Glyph::Blocks),
                active,
                touch,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.open_plugin_screen(target.clone(), window, cx);
            })),
        );
    }
    Some(block.into_any_element())
}
