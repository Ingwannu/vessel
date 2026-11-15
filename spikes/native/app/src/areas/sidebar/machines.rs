//! The sidebar grouped by machine (the Display menu's "Machine", the
//! default): each machine of the book with its state, then its first
//! listed sessions (`Fleet::sidebar_machines`: ended ones only when asked
//! for, pinned first, in the chosen order), then "N more", which opens the
//! board on that machine (the page's `sidebar-machines.tsx`).
//!
//! [Decision Log]
//! - Purpose: a stopped node showed the transport's English text and the
//!   endpoint under the machine's name on a Korean screen (track G2's
//!   find, 2026-11-13).
//! - Choice: the reason is typed (`Reach::Unreachable`), so the row says
//!   it in the app's words in both languages (the state beside the name,
//!   one line of what to do under it); the transport's text is only the
//!   tooltip on that line, for whoever needs the detail.
//! - Trade-off: on touch (no tooltips) the transport's text is not on
//!   screen at all; the line says what to check, which is what a phone's
//!   owner can act on.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, dot, kicker};
use vessel_app_view::Reach;
use vessel_app_view::prefs::SidebarView;
use vessel_app_view::sidebar::SidebarMachine;

use super::{INSET, more_row};
use crate::routes::Route;
use crate::shell::Shell;

/// Every machine's group, in the book's order.
pub(super) fn groups(
    view: SidebarView,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut list = div().flex().flex_col().gap(px(spacing::S3_5));
    for machine in shell
        .fleet
        .sidebar_machines(view, vessel_app_view::sidebar::SIDEBAR_SESSIONS)
    {
        list = list.child(group(&machine, shell, tokens, touch, cx));
    }
    list
}

/// A machine's name, its state, and its listed sessions.
fn group(
    listed: &SidebarMachine<'_>,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let machine = listed.machine;
    let (state, color) = match &machine.reach {
        Reach::Online => (t.online, tokens.st_ready),
        Reach::Unreachable(_) => (t.unreachable, tokens.danger),
    };
    let mut rows = div().flex().flex_col().gap(px(spacing::S0_5)).child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S1_5))
            .px(px(INSET))
            .pb(px(spacing::S0_5))
            .child(kicker(machine.name.clone(), tokens))
            .child(dot(color))
            .child(
                div()
                    .type_(Type::Tiny)
                    .text_color(tokens.text_tertiary)
                    .child(state),
            ),
    );
    if let Reach::Unreachable(why) = &machine.reach {
        let state = &shell.areas.sidebar;
        let line = div()
            .id(ElementId::Name(
                format!("unreachable-{}", machine.name).into(),
            ))
            .px(px(INSET))
            .type_(Type::Tiny)
            .text_color(tokens.text_tertiary)
            .child(t.unreachable_hint);
        rows = rows.child(
            state
                .tips
                .on(line, &state.place(&machine.name), why.clone(), touch),
        );
    }
    for card in &listed.sessions {
        rows = rows.child(super::session_row::session_row(
            card, None, shell, tokens, touch, cx,
        ));
    }
    if listed.more > 0 {
        let name = machine.name.clone();
        rows = rows.child(
            more_row(
                ElementId::Name(format!("more-{}", machine.name).into()),
                listed.more,
                shell,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.scope_board(Some(name.clone()), cx);
                this.go(Route::Board, window, cx);
            })),
        );
    }
    rows
}
