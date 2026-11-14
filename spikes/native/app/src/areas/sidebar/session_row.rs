//! A session's row in the sidebar, as the page draws it (`features/shell/
//! sidebar-machines.tsx`, `session-mark.tsx`): the agent's mark, the mark
//! of what the session is doing in a 20-point slot, its title, and (grouped
//! by status) its machine at the right.
//!
//! [Decision Log]
//! - Purpose: the row says at a glance whether the session waits on its
//!   owner, works, rests or has ended, and whether something in it is
//!   unread, in the page's marks (DESIGN.md §2.2).
//! - Choice: the page's marks and sizes (an 8-point dot, its 1-point ring
//!   in the secondary text at 60%, the needs-you fill, a 12-point spinner
//!   in the building colour, a 1-point dash for an ended one, the muted
//!   fill for an unread one that neither works nor waits); an unread
//!   title in the primary colour at medium weight. The agent's mark stays
//!   ahead of it (the native sidebar has drawn it since track S's rows).
//! - Difference, named: no branch chip yet (the page shows the session's
//!   branch; the card's branch is the workspace track's to draw).

use gpui::FontWeight;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;
use vessel_app_view::sidebar::SessionMark;

use super::INSET;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

/// The mark's slot, so titles keep one rail whatever leads them.
const SLOT: f32 = 20.0;
/// A dot's size.
const DOT: f32 = 8.0;
/// The spinner's size.
const SPIN: f32 = 12.0;
/// A row's height beyond its line (the page's 5 above and below).
const ROW_Y: f32 = 5.0;

/// `card`'s row; `machine`: its machine's name at the right (grouped by
/// status).
pub(super) fn session_row(
    card: &Card,
    machine: Option<&str>,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let active = shell.route == Route::Session
        && shell
            .areas
            .session
            .open
            .as_ref()
            .is_some_and(|open| open.session == card.session && open.machine == card.machine);
    let unread = card.detail.marks.unread;
    let open = Open::of(card);
    let title = if active || unread {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };
    div()
        .id(ElementId::Name(format!("session-{}", card.session).into()))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(INSET))
        .py(px(ROW_Y))
        .rounded(px(radius::R4))
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(crate::agent_mark::mark(
            card.detail.harness.as_ref(),
            vessel_app_kit::agent_mark::Size::Fourteen,
            vessel_app_kit::agent_mark::Space::Compact,
            tokens,
        ))
        .child(
            div()
                .w(px(SLOT))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(mark(card, unread, tokens)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Label)
                .text_color(title)
                .when(unread, |label| label.font_weight(FontWeight::MEDIUM))
                .child(shell.t.title(&card.title).to_owned()),
        )
        .children(machine.map(|name| {
            div()
                .flex_none()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(name.to_owned())
        }))
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}

/// What the session is doing, as the page's `SessionMark` draws it.
fn mark(card: &Card, unread: bool, tokens: &Tokens) -> AnyElement {
    let doing = card.session_mark();
    if unread && matches!(doing, SessionMark::Idle | SessionMark::Ended) {
        return dot()
            .bg(tokens.text_secondary.opacity(0.6))
            .into_any_element();
    }
    match doing {
        SessionMark::NeedsYou => dot().bg(tokens.st_needs_you).into_any_element(),
        SessionMark::Working => vessel_app_kit::slider::spinner(
            ElementId::Name(format!("session-spin-{}", card.session).into()),
            SPIN,
            tokens.st_building,
        ),
        SessionMark::Idle => dot()
            .border_1()
            .border_color(tokens.text_secondary.opacity(0.6))
            .into_any_element(),
        SessionMark::Ended => div()
            .w(px(DOT))
            .h(px(1.0))
            .bg(tokens.text_disabled)
            .into_any_element(),
    }
}

fn dot() -> vessel_app_kit::ui::Div {
    div().size(px(DOT)).rounded_full()
}
