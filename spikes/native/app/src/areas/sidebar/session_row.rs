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
//! - Labels (ADR 0086): the session's workspace labels as Paseo's chips on
//!   a second line under the title rail (Paseo's workspace meta row), only
//!   when it has any, so an unlabelled row keeps its one line.
//! - Difference, named: no branch chip yet (the page shows the session's
//!   branch; the card's branch is the workspace track's to draw); the meta
//!   line carries only the labels (Paseo's also shows a change request and
//!   checks).
//! - Its menu (`row_menu`): Paseo's kebab at the row's right, shown while
//!   the row is hovered or its menu is open and always on touch (18 points
//!   on a wide window, 32 on touch, corners of 4, the muted mark
//!   over `surface2` when hovered; Paseo also turns the mark to the
//!   foreground), and a right press anywhere on the row.

use gpui::FontWeight;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement,
    MouseButton, MouseDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _,
    Styled as _, div, px, svg,
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
/// An ended session's dash: the page's one-pixel line (`sidebar.css`,
/// `.sdot.ended`, a top border of 1 px across the dot's width).
const DASH: f32 = 1.0;
/// The spinner's size.
const SPIN: f32 = 12.0;
/// A row's height beyond its line (the page's 5 above and below).
const ROW_Y: f32 = 5.0;
/// The kebab's square on a wide window and on touch (Paseo's `md` 18 and
/// `xs` 32).
const DOTS: f32 = 18.0;
const DOTS_TOUCH: f32 = 32.0;
/// Its mark (Paseo's `sm` density icon).
const DOTS_MARK: f32 = 14.0;
/// Its corners (Paseo's 4).
const DOTS_CORNER: f32 = 4.0;
/// The agent's mark ahead of the slot (`Size::Fourteen`).
const AGENT: f32 = 14.0;
/// Where a title starts: the agent's mark, the slot and their gaps, so
/// the labels' line starts under it.
const TITLE_RAIL: f32 = AGENT + spacing::S2 + SLOT + spacing::S2;

/// `card`'s row; `machine`: its machine's name at the right (grouped by
/// status).
pub(super) fn session_row(
    card: &Card,
    machine: Option<&str>,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
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
    let group: SharedString = format!("session-row-{}-{}", card.machine, card.session).into();
    let menu_open = shell.areas.sidebar.rows.is_for(card);
    let pressed = card.clone();
    let title = if active || unread {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };
    let labels = shell
        .areas
        .labels
        .row(&card.machine, card.session, card.member_project())
        .map(|row| row.labels)
        .unwrap_or_default();
    let line = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .child(crate::agent_mark::mark_muted(
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
        .child(dots(card, group.clone(), touch, menu_open, tokens, cx));
    div()
        .id(ElementId::Name(format!("session-{}", card.session).into()))
        .group(group)
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .px(px(INSET))
        .py(px(ROW_Y))
        .rounded(px(radius::R4))
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(line)
        .children(labels_line(&labels, tokens))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                this.row_menu_open(&pressed, event.position, window, cx);
            }),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}

/// The workspace's labels under the title rail, when it has any.
fn labels_line(
    labels: &[vessel_app_view::labels::Label],
    tokens: &Tokens,
) -> Option<vessel_app_kit::ui::Div> {
    (!labels.is_empty()).then(|| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S1))
            .pl(px(TITLE_RAIL))
            .min_w_0()
            .overflow_hidden()
            .children(
                labels
                    .iter()
                    .map(|label| crate::areas::labels::chip(label, tokens)),
            )
    })
}

/// The row's kebab: always shown on touch or while its menu is open, else
/// only while the row is hovered.
fn dots(
    card: &Card,
    group: SharedString,
    touch: bool,
    menu_open: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let size = if touch { DOTS_TOUCH } else { DOTS };
    let shown = touch || menu_open;
    let target = card.clone();
    div()
        .id(ElementId::Name(format!("session-menu-{}", card.session).into()))
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(DOTS_CORNER))
        .when(!shown, |dots| {
            dots.opacity(0.0)
                .group_hover(group, |style| style.opacity(1.0))
        })
        .hover(|style| style.bg(tokens.neutral.surface2))
        .child(
            svg()
                .path(Glyph::More.path())
                .size(px(DOTS_MARK))
                .text_color(tokens.neutral.foreground_muted),
        )
        // Its own press: the row's click (open the session) does not run.
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.row_menu_open(&target, event.position, window, cx);
            }),
        )
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
            .h(px(DASH))
            .bg(tokens.text_disabled)
            .into_any_element(),
    }
}

fn dot() -> vessel_app_kit::ui::Div {
    div().size(px(DOT)).rounded_full()
}
