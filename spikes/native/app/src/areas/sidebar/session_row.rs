//! A session's row in the sidebar, as Paseo's workspace row
//! (`components/sidebar/sidebar-workspace-row-content.tsx`,
//! `sidebar-workspace-list.tsx` `workspaceRow`): the status slot, the
//! title, the meta line under it, the kebab, the shortcut number.
//!
//! [Decision Log] (TRACKER 663)
//! - Purpose: the row reads as Paseo's: one status slot leading it, a
//!   quiet title, what else is known on one line under it.
//! - Before: the web page's row (`sidebar-machines.tsx`): the agent's mark,
//!   the page's 8 dot, spinner and dash, an unread title in the primary
//!   colour at medium weight, labels on a second line only, the machine at
//!   the end (grouped by status).
//! - Choice: Paseo's `WorkspaceStatusIndicator` in the slot (`kit::
//!   status_mark`, the bucket from `Card::bucket`); no agent mark (a Paseo
//!   row has none, the coordinator's ruling 2026-11-19); the title 14 on 20
//!   in the foreground at 76%, full on hover; the meta line (`row_meta`);
//!   Paseo's row box: 36 high (44 on a phone), 8 above and below, 16 from
//!   the left under a group header (padding, so the fills keep the group's
//!   width), 12 from the right, corners of 8; the sidebar's fills
//!   (`SidebarSurfaces`): hover, selected while the session is open,
//!   pressed.
//! - Its menu (`row_menu`): Paseo's vertical kebab over the title's end on
//!   hover, the row's hover fill fading the title into it (Paseo's 48
//!   scrim, solid from 55%); always shown on touch, where it keeps its room
//!   (32 square) and needs no scrim; a right press anywhere on the row.
//! - The shortcut number (Paseo's `SidebarWorkspaceShortcutBadge`) at the
//!   title's end while the jump modifier is held (`shortcut_badges_visible`),
//!   for the first nine rows in the order the keys walk; it hides the kebab.
//! - Differences, named: the hover card (TRACKER 810) and the rows under
//!   their project (TRACKER 682) are their own items; Paseo's "creating"
//!   title (92%) has no state here.

use gpui::{FontWeight, linear_color_stop, linear_gradient};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::status_mark::{StatusMark, status_slot};
use vessel_app_kit::theme::{SidebarSurfaces, radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, MouseDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _,
    Styled as _, div, px, svg,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;
use vessel_app_view::attention::Bucket;
use vessel_app_view::sidebar::status::WorkspaceKind;

use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

/// A row's least height, on a wide window and on a phone (Paseo's
/// `density.rowHeight`).
const ROW: f32 = 36.0;
const ROW_TOUCH: f32 = 44.0;
/// Its left padding under a group header (Paseo's `rowIndented`: 8 + 8).
const INDENT: f32 = 16.0;
/// The title's line.
const TITLE_LINE: f32 = 20.0;
/// The title at rest and hovered.
const TITLE_REST: f32 = 0.76;
/// The kebab's square on a wide window and on touch (Paseo's `md` 18 and
/// `xs` 32).
const DOTS: f32 = 18.0;
const DOTS_TOUCH: f32 = 32.0;
/// Its mark (Paseo's `sm` density icon).
const DOTS_MARK: f32 = 14.0;
/// Its corners (Paseo's 4).
const DOTS_CORNER: f32 = 4.0;
/// The scrim fading the title into the kebab (`SCRIM_WIDTH`), solid from
/// 55% of its width.
const SCRIM: f32 = 48.0;
const SCRIM_SOLID: f32 = 0.55;
/// The shortcut number's box (Paseo's `shortcutBadge`).
const NUMBER: f32 = 18.0;
const NUMBER_LINE: f32 = 14.0;
/// It sits 1 below the row content's top (`shortcutBadgeOverlay`).
const NUMBER_TOP: f32 = 1.0;
/// Paseo's `borderRadius.sm`.
const NUMBER_CORNER: f32 = 2.0;

/// What a row is drawn with beyond its card.
#[derive(Debug, Clone, Copy)]
pub(super) struct RowExtras<'a> {
    /// Its machine's name on the meta line (grouped by status).
    pub(super) machine: Option<&'a str>,
    /// Its shortcut number, while the badges show.
    pub(super) number: Option<usize>,
}

/// `card`'s row.
pub(super) fn session_row(
    card: &Card,
    extras: RowExtras<'_>,
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
    let open = Open::of(card);
    let group: SharedString = format!("session-row-{}-{}", card.machine, card.session).into();
    let menu_open = shell.areas.sidebar.rows.is_for(card);
    let pressed = card.clone();
    let fills = SidebarSurfaces::of(tokens);
    // The fill a hovered row paints: what the scrim fades into.
    let hovered_fill = if active { fills.selected } else { fills.hover };
    let title = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .type_(Type::Label)
        .font_weight(FontWeight::NORMAL)
        .line_height(px(TITLE_LINE))
        .text_color(tokens.neutral.foreground)
        .opacity(TITLE_REST)
        .group_hover(group.clone(), |style| style.opacity(1.0))
        .child(shell.t.title(&card.title).to_owned());
    let title_row = div()
        .relative()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::S2))
        .child(title)
        .children(match extras.number {
            Some(number) => Some(shortcut_number(number, tokens)),
            None => dots(
                card,
                group.clone(),
                touch,
                menu_open,
                hovered_fill,
                tokens,
                cx,
            ),
        });
    let content = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .child(title_row)
        .children(super::row_meta::meta_line(
            card,
            extras.machine,
            shell,
            tokens,
        ));
    div()
        .id(ElementId::Name(format!("session-{}", card.session).into()))
        .group(group)
        .min_h(px(if touch { ROW_TOUCH } else { ROW }))
        .flex()
        .flex_col()
        .justify_center()
        .py(px(spacing::S2))
        .pl(px(INDENT))
        .pr(px(spacing::S3))
        .rounded(px(radius::R2))
        .when(active, |row| row.bg(fills.selected))
        .when(!active, |row| row.hover(move |style| style.bg(fills.hover)))
        .active(move |style| style.bg(fills.pressed))
        .cursor_pointer()
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(spacing::S2))
                .child(status_slot(
                    ElementId::Name(format!("session-status-{}", card.session).into()),
                    mark(card.bucket()),
                    kind_glyph(card.workspace_kind()),
                    touch,
                    tokens,
                ))
                .child(content),
        )
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

/// Paseo's bucket as the kit's mark (the project row's badge too).
pub(crate) const fn mark(bucket: Bucket) -> StatusMark {
    match bucket {
        Bucket::NeedsInput => StatusMark::NeedsInput,
        Bucket::Failed => StatusMark::Failed,
        Bucket::Working => StatusMark::Running,
        Bucket::Review => StatusMark::Attention,
        Bucket::Done => StatusMark::Done,
    }
}

/// The glyph a failed row shows (Paseo's `Monitor`, `FolderGit2`,
/// `Folder`).
const fn kind_glyph(kind: WorkspaceKind) -> Glyph {
    match kind {
        WorkspaceKind::Checkout => Glyph::Monitor,
        WorkspaceKind::Worktree => Glyph::FolderGit,
        WorkspaceKind::Folder => Glyph::Folder,
    }
}

/// The row's kebab: always shown on touch (keeping its room) or while its
/// menu is open; else over the title's end while the row is hovered, the
/// title fading into the row's fill under it.
fn dots(
    card: &Card,
    group: SharedString,
    touch: bool,
    menu_open: bool,
    hovered_fill: Hsla,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let target = card.clone();
    let size = if touch { DOTS_TOUCH } else { DOTS };
    let button = div()
        .id(ElementId::Name(format!("session-menu-{}", card.session).into()))
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(DOTS_CORNER))
        .hover(|style| style.bg(tokens.neutral.surface2))
        .child(
            svg()
                .path(Glyph::MoreVertical.path())
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
        );
    if touch {
        return Some(button.into_any_element());
    }
    let scrim = div()
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(px(SCRIM))
        .bg(linear_gradient(
            90.0,
            linear_color_stop(hovered_fill.opacity(0.0), 0.0),
            linear_color_stop(hovered_fill, SCRIM_SOLID),
        ));
    Some(
        div()
            .absolute()
            .top_0()
            .right_0()
            .h(px(TITLE_LINE))
            .flex()
            .flex_row()
            .justify_end()
            .when(!menu_open, |overlay| {
                overlay
                    .opacity(0.0)
                    .group_hover(group, |style| style.opacity(1.0))
            })
            .when(!menu_open, |overlay| overlay.child(scrim))
            .child(button)
            .into_any_element(),
    )
}

/// The jump's number at the title's end (Paseo's shortcut badge: 18 tall,
/// `surface0` on a `surface2` hairline, the number 12 medium and muted).
fn shortcut_number(number: usize, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    div()
        .absolute()
        .top(px(NUMBER_TOP))
        .right_0()
        .min_w(px(NUMBER))
        .h(px(NUMBER))
        .px(px(spacing::S1))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(NUMBER_CORNER))
        .border_1()
        .border_color(neutral.surface2)
        .bg(neutral.surface0)
        .type_(Type::Meta)
        .font_weight(FontWeight::MEDIUM)
        .line_height(px(NUMBER_LINE))
        .text_color(neutral.foreground_muted)
        .child(number.to_string())
        .into_any_element()
}
